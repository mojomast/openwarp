use crate::api::schema::{PermissionReply, PermissionReplyKind, Session};
use crate::api::ApiClient;
use crate::state::{AppModel, AppStore};
use crate::views::chat_thread::ChatThreadView;
use crate::views::input_bar::InputBarView;
use crate::views::pty_panel::render_pty_panel;
use crate::views::question_prompt::render_question_prompt_overlay;
use crate::views::session_list::{render_session_list, SessionListSnapshot};
use crate::views::status_bar::StatusBarView;
use crate::views::tool_approval::render_tool_approval_overlay;
use crate::views::UiAction;
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::presenter::ChildView;
use warpui::SingletonEntity as _;
use warpui::{
    elements::{
        ConstrainedBox, Container, DispatchEventResult, Element, EventHandler, Expanded, Flex,
        MainAxisSize, ParentElement,
    },
    AppContext, Entity, TypedActionView, UpdateView, View, ViewContext, ViewHandle,
};

pub struct RootView {
    font_family: FamilyId,
    client: ApiClient,
    store: AppStore,
    model: AppModel,
    chat_thread: ViewHandle<ChatThreadView>,
    input_bar: ViewHandle<InputBarView>,
    status_bar: ViewHandle<StatusBarView>,
    pty_visible: bool,
}

impl RootView {
    pub fn new(
        ctx: &mut ViewContext<Self>,
        client: ApiClient,
        store: AppStore,
        model: AppModel,
    ) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());

        let chat_snapshot = model.clone();
        let chat_thread = ctx.add_view(move |ctx| ChatThreadView::new(ctx, chat_snapshot));

        let input_client = client.clone();
        let input_snapshot = model.clone();
        let input_bar = ctx.add_typed_action_view(move |ctx| {
            InputBarView::new(ctx, input_client.clone(), input_snapshot.clone())
        });

        let status_bar = ctx.add_view({
            let model = model.clone();
            move |_| StatusBarView::new(font_family, model.clone())
        });

        let spawner = ctx.spawner();
        let mut changes = store.subscribe();
        let store_for_task = store.clone();
        tokio::spawn(async move {
            while changes.recv().await.is_ok() {
                let snapshot = store_for_task.snapshot().await;
                let _ = spawner
                    .spawn(move |view, ctx| {
                        view.set_model(snapshot, ctx);
                    })
                    .await;
            }
        });

        Self {
            font_family,
            client,
            store,
            model,
            chat_thread,
            input_bar,
            status_bar,
            pty_visible: false,
        }
    }

    fn set_model(&mut self, model: AppModel, ctx: &mut ViewContext<Self>) {
        self.model = model;
        self.sync_child_snapshots(ctx);
        ctx.notify();
    }

    fn sync_child_snapshots(&mut self, ctx: &mut ViewContext<Self>) {
        let model = self.model.clone();
        ctx.update_view(&self.chat_thread, |view, child_ctx| {
            view.set_snapshot(model.clone());
            child_ctx.notify();
        });
        let model = self.model.clone();
        ctx.update_view(&self.input_bar, |view, child_ctx| {
            view.set_snapshot(model.clone(), child_ctx);
        });
        let model = self.model.clone();
        ctx.update_view(&self.status_bar, |view, child_ctx| {
            view.update_model(model);
            child_ctx.notify();
        });
    }

    fn apply_session(&mut self, session: Session, ctx: &mut ViewContext<Self>) {
        self.model.upsert_session(session);
        self.sync_child_snapshots(ctx);
        ctx.notify();
    }

    fn handle_permission(&mut self, request_id: String, reply: PermissionReply) {
        self.model.permissions.remove(&request_id);
        let client = self.client.clone();
        let store = self.store.clone();
        tokio::spawn(async move {
            match client.reply_permission(&request_id, &reply).await {
                Ok(true) => store.remove_permission(&request_id).await,
                Ok(false) => tracing::warn!(%request_id, "permission reply was not accepted"),
                Err(error) => {
                    tracing::warn!(%request_id, %error, "failed to reply to permission request")
                }
            }
        });
    }
}

impl Entity for RootView {
    type Event = ();
}

impl TypedActionView for RootView {
    type Action = UiAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            UiAction::NewSession => {
                let client = self.client.clone();
                let store = self.store.clone();
                ctx.spawn(
                    async move {
                        let session = client.create_session(None).await?;
                        store.upsert_session(session.clone()).await;
                        Ok::<_, crate::api::ApiError>(session)
                    },
                    |view, result, ctx| match result {
                        Ok(session) => view.apply_session(session, ctx),
                        Err(error) => tracing::warn!(%error, "failed to create session"),
                    },
                );
            }
            UiAction::SelectSession(session_id) => {
                self.model.active_session_id = Some(session_id.clone());
                let store = self.store.clone();
                let session_id = session_id.clone();
                tokio::spawn(async move {
                    store.set_active_session(Some(session_id)).await;
                });
                self.sync_child_snapshots(ctx);
                ctx.notify();
            }
            UiAction::AllowPermission(request_id) => {
                self.handle_permission(
                    request_id.clone(),
                    PermissionReply {
                        reply: PermissionReplyKind::Once,
                        message: None,
                    },
                );
                ctx.notify();
            }
            UiAction::DenyPermission(request_id) => {
                self.handle_permission(
                    request_id.clone(),
                    PermissionReply {
                        reply: PermissionReplyKind::Reject,
                        message: None,
                    },
                );
                ctx.notify();
            }
            UiAction::TogglePty => {
                self.pty_visible = !self.pty_visible;
                ctx.notify();
            }
        }
    }
}

impl View for RootView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeRoot"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let session_list =
            render_session_list(&SessionListSnapshot::from(&self.model), self.font_family);

        let main_content = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(Expanded::new(1., ChildView::new(&self.chat_thread).finish()).finish())
            .with_child(ChildView::new(&self.input_bar).finish())
            .finish();

        let workspace = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(ConstrainedBox::new(session_list).with_width(280.).finish())
            .with_child(Expanded::new(1., main_content).finish())
            .finish();

        let mut root = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_child(Expanded::new(1., workspace).finish());

        if self.pty_visible {
            root = root.with_child(render_pty_panel(&self.model, self.font_family));
        }

        let base = Container::new(
            root.with_child(ChildView::new(&self.status_bar).finish())
                .finish(),
        )
        .with_background_color(ColorU::new(12, 14, 20, 255))
        .finish();

        let base = EventHandler::new(base)
            .with_always_handle()
            .on_keydown(|ctx, _app, keystroke| {
                if keystroke.ctrl && matches!(keystroke.key.as_str(), "`" | "backquote") {
                    ctx.dispatch_typed_action(UiAction::TogglePty);
                    return DispatchEventResult::StopPropagation;
                }
                DispatchEventResult::PropagateToParent
            })
            .finish();

        let base = render_question_prompt_overlay(base, &self.model, self.font_family);
        render_tool_approval_overlay(base, &self.model, self.font_family)
    }
}
