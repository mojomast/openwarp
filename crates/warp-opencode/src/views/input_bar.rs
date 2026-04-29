//! Bottom composer/input bar rendering and prompt submission helpers.
//!
//! WarpUI does not currently expose a text-editor widget through this crate, so
//! this module renders a composer shell and keeps a local draft string on the
//! view.  The keyboard handler below supports only minimal printable-character,
//! backspace, enter, and shift-enter behavior; it is intentionally small and can
//! be replaced with a real editor when one is wired into `warp-opencode`.

use crate::api::client::{ApiClient, ApiError};
use crate::api::schema::{
    MessageWithParts, ModelRef, PromptPartInput, SendMessageRequest, SessionStatus,
};
use crate::state::AppModel;
use std::collections::HashMap;
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::SingletonEntity as _;
use warpui::{
    elements::{
        Border, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, DispatchEventResult,
        Element, EventHandler, Flex, MainAxisAlignment, MainAxisSize, ParentElement, Radius, Text,
    },
    AppContext, Entity, TypedActionView, View, ViewContext,
};

const PLACEHOLDER: &str = "Ask OpenCode…";

#[derive(Debug, Clone)]
pub enum InputBarAction {
    Insert(String),
    Backspace,
    Newline,
    Send,
    Clear,
}

#[derive(Debug, Clone)]
pub enum SendState {
    Idle,
    Sending,
    Sent,
    Error(String),
}

/// Renderable WarpUI input bar plus local composer state.
pub struct InputBarView {
    font_family: FamilyId,
    api: ApiClient,
    snapshot: AppModel,
    draft: String,
    send_state: SendState,
}

impl InputBarView {
    pub fn new(ctx: &mut ViewContext<Self>, api: ApiClient, snapshot: AppModel) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());
        Self {
            font_family,
            api,
            snapshot,
            draft: String::new(),
            send_state: SendState::Idle,
        }
    }

    pub fn set_snapshot(&mut self, snapshot: AppModel, ctx: &mut ViewContext<Self>) {
        self.snapshot = snapshot;
        ctx.notify();
    }

    pub fn draft(&self) -> &str {
        &self.draft
    }

    pub fn set_draft(&mut self, draft: impl Into<String>, ctx: &mut ViewContext<Self>) {
        self.draft = draft.into();
        ctx.notify();
    }

    pub fn clear(&mut self, ctx: &mut ViewContext<Self>) {
        self.draft.clear();
        self.send_state = SendState::Idle;
        ctx.notify();
    }

    pub fn send(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.can_send() {
            return;
        }

        let Some(session_id) = self.snapshot.active_session_id.clone() else {
            return;
        };
        let text = self.draft.trim().to_owned();
        let request = SendMessageRequest {
            message_id: None,
            model: self.selected_model_ref(),
            agent: None,
            no_reply: None,
            tools: None,
            parts: vec![PromptPartInput::Text { text }],
        };
        let api = self.api.clone();

        self.draft.clear();
        self.send_state = SendState::Sending;
        ctx.notify();

        ctx.spawn(
            async move { api.send_message(&session_id, &request).await },
            |view, result: Result<MessageWithParts, ApiError>, ctx| {
                view.send_state = match result {
                    Ok(_) => SendState::Sent,
                    Err(err) => SendState::Error(err.to_string()),
                };
                ctx.notify();
            },
        );
    }

    fn can_send(&self) -> bool {
        self.is_active_session_idle()
            && self.snapshot.active_session_id.is_some()
            && !self.draft.trim().is_empty()
            && !matches!(self.send_state, SendState::Sending)
    }

    fn is_active_session_idle(&self) -> bool {
        let Some(session_id) = self.snapshot.active_session_id.as_deref() else {
            return false;
        };
        matches!(
            self.snapshot.statuses.get(session_id),
            Some(SessionStatus::Idle) | None
        )
    }

    fn active_status_label(&self) -> &'static str {
        let Some(session_id) = self.snapshot.active_session_id.as_deref() else {
            return "No session";
        };
        match self.snapshot.statuses.get(session_id) {
            Some(SessionStatus::Busy) => "Busy",
            Some(SessionStatus::Retry { .. }) => "Retrying",
            Some(SessionStatus::Idle) | None => "Idle",
        }
    }

    fn selected_model_ref(&self) -> Option<ModelRef> {
        let providers = self.snapshot.providers.as_ref()?;

        if let Some((provider_id, model_id)) = providers.default.iter().next() {
            return Some(ModelRef {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
            });
        }

        let provider = providers
            .connected
            .iter()
            .find_map(|id| providers.all.iter().find(|provider| &provider.id == id))
            .or_else(|| providers.all.first())?;
        let model_id = first_sorted_key(&provider.models)?;

        Some(ModelRef {
            provider_id: provider.id.clone(),
            model_id,
        })
    }

    fn model_label(&self) -> String {
        let Some(providers) = self.snapshot.providers.as_ref() else {
            return "Model loading".to_string();
        };
        let Some(model_ref) = self.selected_model_ref() else {
            return "No model".to_string();
        };

        let provider_name = providers
            .all
            .iter()
            .find(|provider| provider.id == model_ref.provider_id)
            .map(|provider| provider.name.as_str())
            .unwrap_or(model_ref.provider_id.as_str());

        format!("{provider_name} / {}", model_ref.model_id)
    }

    fn send_state_label(&self) -> Option<String> {
        match &self.send_state {
            SendState::Idle => None,
            SendState::Sending => Some("Sending…".to_string()),
            SendState::Sent => Some("Sent".to_string()),
            SendState::Error(err) => Some(format!("Send failed: {err}")),
        }
    }

    fn label(&self, text: impl Into<String>, size: f32, color: ColorU) -> Box<dyn Element> {
        Text::new(text.into(), self.font_family, size)
            .with_color(color)
            .finish()
    }

    fn pill(&self, text: impl Into<String>, disabled: bool) -> Box<dyn Element> {
        let (bg, fg, border) = if disabled {
            (
                ColorU::new(34, 36, 44, 255),
                ColorU::new(132, 136, 148, 255),
                ColorU::new(58, 61, 72, 255),
            )
        } else {
            (
                ColorU::new(34, 42, 56, 255),
                ColorU::new(214, 221, 234, 255),
                ColorU::new(72, 87, 112, 255),
            )
        };

        Container::new(self.label(text, 12., fg))
            .with_horizontal_padding(10.)
            .with_vertical_padding(5.)
            .with_background_color(bg)
            .with_border(Border::all(1.).with_border_color(border))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(999.)))
            .finish()
    }
}

impl Entity for InputBarView {
    type Event = ();
}

impl TypedActionView for InputBarView {
    type Action = InputBarAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            InputBarAction::Insert(text) if self.is_active_session_idle() => {
                self.draft.push_str(text);
                self.send_state = SendState::Idle;
                ctx.notify();
            }
            InputBarAction::Backspace if self.is_active_session_idle() => {
                self.draft.pop();
                self.send_state = SendState::Idle;
                ctx.notify();
            }
            InputBarAction::Newline if self.is_active_session_idle() => {
                self.draft.push('\n');
                self.send_state = SendState::Idle;
                ctx.notify();
            }
            InputBarAction::Send => self.send(ctx),
            InputBarAction::Clear => self.clear(ctx),
            _ => {}
        }
    }
}

impl View for InputBarView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeInputBar"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let disabled = !self.is_active_session_idle();
        let can_send = self.can_send();
        let draft_text = if self.draft.is_empty() {
            PLACEHOLDER.to_string()
        } else {
            self.draft.clone()
        };
        let draft_color = if disabled {
            ColorU::new(106, 110, 122, 255)
        } else if self.draft.is_empty() {
            ColorU::new(132, 136, 148, 255)
        } else {
            ColorU::new(235, 238, 245, 255)
        };

        let editor = Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Min)
                .with_spacing(8.)
                .with_child(self.label(draft_text, 14., draft_color))
                .finish(),
        )
        .with_uniform_padding(12.)
        .with_background_color(if disabled {
            ColorU::new(20, 22, 28, 255)
        } else {
            ColorU::new(16, 18, 24, 255)
        })
        .with_border(Border::all(1.).with_border_color(if disabled {
            ColorU::new(54, 56, 64, 255)
        } else {
            ColorU::new(74, 91, 122, 255)
        }))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(10.)))
        .finish();

        let send_button = Container::new(self.label(
            if can_send { "Send ↵" } else { "Send" },
            13.,
            if can_send {
                ColorU::new(255, 255, 255, 255)
            } else {
                ColorU::new(130, 134, 146, 255)
            },
        ))
        .with_horizontal_padding(14.)
        .with_vertical_padding(9.)
        .with_background_color(if can_send {
            ColorU::new(56, 116, 255, 255)
        } else {
            ColorU::new(40, 43, 52, 255)
        })
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
        .finish();

        let mut footer = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Flex::row()
                    .with_spacing(8.)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(self.pill(self.model_label(), disabled))
                    .with_child(self.pill(self.active_status_label(), disabled))
                    .finish(),
            )
            .with_child(send_button);

        if let Some(status) = self.send_state_label() {
            footer = footer.with_child(self.label(status, 12., ColorU::new(156, 163, 178, 255)));
        }

        let content = Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Min)
                .with_spacing(10.)
                .with_child(editor)
                .with_child(footer.finish())
                .finish(),
        )
        .with_uniform_padding(14.)
        .with_background_color(ColorU::new(12, 14, 20, 255))
        .with_border(Border::top(1.).with_border_color(ColorU::new(35, 39, 50, 255)))
        .finish();

        EventHandler::new(ConstrainedBox::new(content).with_min_height(104.).finish())
            .with_always_handle()
            .on_keydown(|ctx, _app, keystroke| {
                match keystroke.key.as_str() {
                    "enter" if keystroke.shift => {
                        ctx.dispatch_typed_action(InputBarAction::Newline)
                    }
                    "enter" => ctx.dispatch_typed_action(InputBarAction::Send),
                    "backspace" => ctx.dispatch_typed_action(InputBarAction::Backspace),
                    key if is_plain_printable_key(key, keystroke) => {
                        ctx.dispatch_typed_action(InputBarAction::Insert(key.to_string()))
                    }
                    _ => return DispatchEventResult::PropagateToParent,
                }
                DispatchEventResult::StopPropagation
            })
            .finish()
    }
}

fn first_sorted_key(values: &HashMap<String, serde_json::Value>) -> Option<String> {
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    keys.first().map(|key| (*key).clone())
}

fn is_plain_printable_key(key: &str, keystroke: &warpui::keymap::Keystroke) -> bool {
    !keystroke.ctrl
        && !keystroke.alt
        && !keystroke.cmd
        && !keystroke.meta
        && key.chars().count() == 1
}
