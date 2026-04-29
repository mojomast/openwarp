use crate::api::schema::{
    PermissionRequest, ProviderListResult, QuestionRequest, Session, SessionId, SessionStatus,
};
use crate::api::{ApiClient, ApiConfig, ApiError, Auth};
use crate::config::Config;
use crate::sse_loop::SseLoop;
use crate::state::{AppStore, ConnectionStatus};
use crate::views::draft_buffer::DraftBuffer;
use crate::views::root::RootView;
use std::collections::HashMap;
use warpui::color::ColorU;
use warpui::elements::{
    Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
    DispatchEventResult, Element, EventHandler, Flex, MainAxisAlignment, MainAxisSize,
    ParentElement, Radius, Text,
};
use warpui::fonts::FamilyId;
use warpui::{
    AppContext, Entity, SingletonEntity as _, TypedActionView, View, ViewContext, ViewHandle,
};

#[derive(Debug, Clone)]
pub enum OnboardingAction {
    Insert(String),
    Paste(String),
    PasteUrl(String),
    PasteToken(String),
    SetUrl(String),
    SetToken(String),
    Backspace,
    Connect,
    FocusUrl,
    FocusToken,
    FocusConnect,
    FocusNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusedField {
    Url,
    Token,
    Connect,
}

struct ConnectedApp {
    client: ApiClient,
    store: AppStore,
    initial_model: crate::state::AppModel,
    server_url: String,
    token: String,
}

pub struct OnboardingView {
    font_family: FamilyId,
    username: String,
    url: DraftBuffer,
    token: DraftBuffer,
    focused: FocusedField,
    connecting: bool,
    error: Option<String>,
    root: Option<ViewHandle<RootView>>,
    sse_handle: Option<tokio::task::JoinHandle<()>>,
}

impl OnboardingView {
    pub fn new(ctx: &mut ViewContext<Self>, config: Config, username: String) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());
        let mut view = Self {
            font_family,
            username,
            url: DraftBuffer::from(config.server_url.unwrap_or_default()),
            token: DraftBuffer::from(config.token.unwrap_or_default()),
            focused: FocusedField::Url,
            connecting: false,
            error: None,
            root: None,
            sse_handle: None,
        };
        if !view.url.trim_text().is_empty() {
            view.connect(ctx);
        }
        view
    }

    fn connect(&mut self, ctx: &mut ViewContext<Self>) {
        if self.connecting || self.root.is_some() {
            return;
        }

        let server_url = normalize_server_url(&self.url.trim_text());
        if server_url.is_empty() {
            self.error = Some("Server URL is required.".to_string());
            ctx.notify();
            return;
        }
        if let Err(error) = ApiConfig::new(&server_url) {
            self.error = Some(format!("Invalid server URL: {error}"));
            ctx.notify();
            return;
        }

        let token = self.token.trim_text();
        let username = self.username.clone();
        self.connecting = true;
        self.error = None;
        ctx.notify();

        ctx.spawn(
            async move { connect_and_bootstrap(server_url, token, username).await },
            |view, result, ctx| {
                view.connecting = false;
                match result {
                    Ok(connected) => {
                        let root = ctx.add_view({
                            let client = connected.client.clone();
                            let store = connected.store.clone();
                            let model = connected.initial_model.clone();
                            move |ctx| {
                                RootView::new(ctx, client.clone(), store.clone(), model.clone())
                            }
                        });
                        let sse_handle =
                            SseLoop::new(connected.store, connected.server_url, connected.token)
                                .spawn();
                        view.root = Some(root);
                        view.sse_handle = Some(sse_handle);
                        view.error = None;
                    }
                    Err(error) => view.error = Some(format!("Could not connect: {error}")),
                }
                ctx.notify();
            },
        );
    }

    fn input_text(&self, field: FocusedField) -> String {
        match field {
            FocusedField::Url => {
                if self.url.is_empty() {
                    "https://opencode.myserver.com".to_string()
                } else if self.focused == FocusedField::Url {
                    self.url.display_with_caret()
                } else {
                    self.url.to_string()
                }
            }
            FocusedField::Token => {
                let masked = "*".repeat(self.token.len_chars());
                if self.token.is_empty() {
                    "optional".to_string()
                } else if self.focused == FocusedField::Token {
                    format!("{masked}|")
                } else {
                    masked
                }
            }
            FocusedField::Connect => String::new(),
        }
    }

    fn input_color(&self, field: FocusedField) -> ColorU {
        match field {
            FocusedField::Url if self.url.is_empty() => palette::muted(),
            FocusedField::Token if self.token.is_empty() => palette::muted(),
            _ => palette::text(),
        }
    }

    fn render_field(&self, label: &str, field: FocusedField) -> Box<dyn Element> {
        let focused = self.focused == field;
        let input = Container::new(text(
            self.input_text(field),
            self.font_family,
            14.,
            self.input_color(field),
        ))
        .with_uniform_padding(12.)
        .with_background_color(ColorU::new(10, 13, 18, 255))
        .with_border(Border::all(1.).with_border_color(if focused {
            ColorU::new(68, 122, 255, 255)
        } else {
            ColorU::new(54, 62, 78, 255)
        }))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
        .finish();
        let action = match field {
            FocusedField::Url => OnboardingAction::FocusUrl,
            FocusedField::Token => OnboardingAction::FocusToken,
            FocusedField::Connect => OnboardingAction::FocusConnect,
        };
        let input = EventHandler::new(input)
            .on_left_mouse_up(move |ctx, _app, _position| {
                ctx.dispatch_typed_action(action.clone());
                DispatchEventResult::StopPropagation
            })
            .finish();

        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(6.)
            .with_child(text(label, self.font_family, 12., palette::muted()))
            .with_child(input)
            .finish()
    }

    fn render_connect_button(&self) -> Box<dyn Element> {
        let label = if self.connecting {
            "Connecting..."
        } else {
            "Connect ->"
        };
        let button = Container::new(text(
            label,
            self.font_family,
            14.,
            ColorU::new(255, 255, 255, 255),
        ))
        .with_uniform_padding(12.)
        .with_background_color(if self.connecting {
            ColorU::new(62, 76, 112, 255)
        } else {
            ColorU::new(56, 116, 255, 255)
        })
        .with_border(
            Border::all(1.).with_border_color(if self.focused == FocusedField::Connect {
                ColorU::new(147, 197, 253, 255)
            } else {
                ColorU::new(56, 116, 255, 255)
            }),
        )
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
        .finish();

        EventHandler::new(button)
            .on_left_mouse_up(|ctx, _app, _position| {
                ctx.dispatch_typed_action(OnboardingAction::Connect);
                DispatchEventResult::StopPropagation
            })
            .finish()
    }

    fn render_onboarding(&self) -> Box<dyn Element> {
        let mut card = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(16.)
            .with_child(text("OpenWarp", self.font_family, 28., palette::text()))
            .with_child(text(
                "Connect to a remote OpenCode server to get started.",
                self.font_family,
                14.,
                palette::muted(),
            ))
            .with_child(self.render_field("Server URL", FocusedField::Url))
            .with_child(self.render_field("Auth Token (optional)", FocusedField::Token))
            .with_child(self.render_connect_button());

        if let Some(error) = &self.error {
            card = card.with_child(text(error.clone(), self.font_family, 13., palette::error()));
        }

        let card = ConstrainedBox::new(
            Container::new(card.finish())
                .with_uniform_padding(28.)
                .with_background_color(ColorU::new(18, 22, 30, 255))
                .with_border(Border::all(1.).with_border_color(ColorU::new(50, 60, 78, 255)))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(16.)))
                .finish(),
        )
        .with_width(480.)
        .finish();

        let page = Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(card)
                .finish(),
        )
        .with_background_color(ColorU::new(9, 11, 17, 255))
        .finish();

        EventHandler::new(page)
            .with_always_handle()
            .on_keydown(|ctx, _app, keystroke| {
                match keystroke.key.as_str() {
                    "v" if (keystroke.ctrl || keystroke.cmd) && !keystroke.alt => {
                        if let Some(text) = clipboard_text() {
                            ctx.dispatch_typed_action(OnboardingAction::Paste(text));
                        }
                    }
                    "tab" => ctx.dispatch_typed_action(OnboardingAction::FocusNext),
                    "enter" => ctx.dispatch_typed_action(OnboardingAction::Connect),
                    "backspace" => ctx.dispatch_typed_action(OnboardingAction::Backspace),
                    key if is_plain_printable_key(key, keystroke) => {
                        ctx.dispatch_typed_action(OnboardingAction::Insert(key.to_string()))
                    }
                    _ => return DispatchEventResult::PropagateToParent,
                }
                DispatchEventResult::StopPropagation
            })
            .finish()
    }
}

impl Drop for OnboardingView {
    fn drop(&mut self) {
        if let Some(handle) = &self.sse_handle {
            handle.abort();
        }
    }
}

impl Entity for OnboardingView {
    type Event = ();
}

impl TypedActionView for OnboardingView {
    type Action = OnboardingAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            OnboardingAction::Insert(text) => {
                match self.focused {
                    FocusedField::Url => self.url.insert_str(text),
                    FocusedField::Token => self.token.insert_str(text),
                    FocusedField::Connect => {}
                }
                ctx.notify();
            }
            OnboardingAction::Paste(text) => {
                match self.focused {
                    FocusedField::Url => self.url.paste(text),
                    FocusedField::Token => self.token.paste(text),
                    FocusedField::Connect => {}
                }
                ctx.notify();
            }
            OnboardingAction::PasteUrl(text) => {
                self.focused = FocusedField::Url;
                self.url.paste(text);
                ctx.notify();
            }
            OnboardingAction::PasteToken(text) => {
                self.focused = FocusedField::Token;
                self.token.paste(text);
                ctx.notify();
            }
            OnboardingAction::SetUrl(text) => {
                self.focused = FocusedField::Url;
                self.url.insert_str(text);
                ctx.notify();
            }
            OnboardingAction::SetToken(text) => {
                self.focused = FocusedField::Token;
                self.token.insert_str(text);
                ctx.notify();
            }
            OnboardingAction::Backspace => {
                match self.focused {
                    FocusedField::Url => {
                        self.url.delete_backward();
                    }
                    FocusedField::Token => {
                        self.token.delete_backward();
                    }
                    FocusedField::Connect => {}
                }
                ctx.notify();
            }
            OnboardingAction::Connect => self.connect(ctx),
            OnboardingAction::FocusUrl => {
                self.focused = FocusedField::Url;
                ctx.notify();
            }
            OnboardingAction::FocusToken => {
                self.focused = FocusedField::Token;
                ctx.notify();
            }
            OnboardingAction::FocusConnect => {
                self.focused = FocusedField::Connect;
                ctx.notify();
            }
            OnboardingAction::FocusNext => {
                self.focused = match self.focused {
                    FocusedField::Url => FocusedField::Token,
                    FocusedField::Token => FocusedField::Connect,
                    FocusedField::Connect => FocusedField::Url,
                };
                ctx.notify();
            }
        }
    }
}

impl View for OnboardingView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeOnboarding"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        if let Some(root) = &self.root {
            return ChildView::new(root).finish();
        }
        self.render_onboarding()
    }
}

async fn connect_and_bootstrap(
    server_url: String,
    token: String,
    username: String,
) -> Result<ConnectedApp, String> {
    let mut api_config = ApiConfig::new(&server_url).map_err(|error| error.to_string())?;
    if !token.is_empty() {
        api_config.auth = Auth::Basic {
            username,
            password: token.clone(),
        };
    }
    let client = ApiClient::new(api_config).map_err(|error| error.to_string())?;
    let store = AppStore::default();
    bootstrap(client.clone(), store.clone())
        .await
        .map_err(|error| error.to_string())?;
    Config {
        server_url: Some(server_url.clone()),
        token: (!token.is_empty()).then_some(token.clone()),
    }
    .save()
    .map_err(|error| error.to_string())?;
    let initial_model = store.snapshot().await;
    Ok(ConnectedApp {
        client,
        store,
        initial_model,
        server_url,
        token,
    })
}

async fn bootstrap(client: ApiClient, store: AppStore) -> Result<(), ApiError> {
    store.set_connection(ConnectionStatus::Connecting).await;
    let result = async {
        client.health().await?;
        let sessions = client.get_or_default::<Vec<Session>>("/session/").await?;
        let statuses = client
            .get_or_default::<HashMap<SessionId, SessionStatus>>("/session/status")
            .await
            .unwrap_or_default();
        let permissions = client
            .get_or_default::<Vec<PermissionRequest>>("/permission/")
            .await
            .unwrap_or_default();
        let questions = client
            .get_or_default::<Vec<QuestionRequest>>("/question/")
            .await
            .unwrap_or_default();
        let providers = client
            .get_or_default::<ProviderListResult>("/provider/")
            .await
            .ok();
        store
            .replace_bootstrap(sessions, statuses, permissions, questions, providers)
            .await;
        Ok::<(), ApiError>(())
    }
    .await;
    match result {
        Ok(()) => {
            store.set_connection(ConnectionStatus::Connected).await;
            Ok(())
        }
        Err(error) => {
            store
                .set_connection(ConnectionStatus::Error(error.to_string()))
                .await;
            Err(error)
        }
    }
}

fn normalize_server_url(server_url: &str) -> String {
    server_url.trim().trim_end_matches('/').to_string()
}

fn is_plain_printable_key(key: &str, keystroke: &warpui::keymap::Keystroke) -> bool {
    !keystroke.ctrl
        && !keystroke.alt
        && !keystroke.cmd
        && !keystroke.meta
        && key.chars().count() == 1
}

#[cfg(not(target_family = "wasm"))]
fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .ok()
        .filter(|text| !text.is_empty())
}

#[cfg(target_family = "wasm")]
fn clipboard_text() -> Option<String> {
    None
}

fn text(
    value: impl Into<String>,
    font_family: FamilyId,
    size: f32,
    color: ColorU,
) -> Box<dyn Element> {
    Text::new(value.into(), font_family, size)
        .with_color(color)
        .finish()
}

mod palette {
    use warpui::color::ColorU;

    pub fn text() -> ColorU {
        ColorU::new(235, 239, 246, 255)
    }
    pub fn muted() -> ColorU {
        ColorU::new(154, 164, 180, 255)
    }
    pub fn error() -> ColorU {
        ColorU::new(248, 113, 113, 255)
    }
}
