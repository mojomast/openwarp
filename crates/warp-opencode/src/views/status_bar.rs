//! Fixed-height status strip for high-level OpenCode runtime state.

use crate::api::schema::{ProviderListResult, SessionStatus};
use crate::state::{AppModel, ConnectionStatus};
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::{
    elements::{
        ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, MainAxisSize, ParentElement,
        Text,
    },
    AppContext, Entity, View,
};

const STATUS_BAR_HEIGHT: f32 = 28.;
const FONT_SIZE: f32 = 12.;
const SESSION_ID_CHARS: usize = 12;

/// Renderable snapshot for the bottom status strip.
///
/// The view is intentionally data-only: callers update it with a fresh
/// [`AppModel`] snapshot when store subscriptions notify. Rendering then stays
/// synchronous and uses only WarpUI element builders.
#[derive(Clone)]
pub struct StatusBarView {
    font_family: FamilyId,
    model: AppModel,
}

impl StatusBarView {
    pub fn new(font_family: FamilyId, model: AppModel) -> Self {
        Self { font_family, model }
    }

    pub fn update_model(&mut self, model: AppModel) {
        self.model = model;
    }

    fn label(&self, text: impl Into<String>, color: ColorU) -> Box<dyn Element> {
        Text::new(text.into(), self.font_family, FONT_SIZE)
            .soft_wrap(false)
            .with_color(color)
            .finish()
    }

    fn item(&self, label: &str, value: String, color: ColorU) -> Box<dyn Element> {
        Flex::row()
            .with_main_axis_size(MainAxisSize::Min)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.)
            .with_child(self.label(format!("{label}:"), muted_text()))
            .with_child(self.label(value, color))
            .finish()
    }

    fn separator(&self) -> Box<dyn Element> {
        self.label("|", ColorU::new(65, 72, 88, 255))
    }
}

impl Entity for StatusBarView {
    type Event = ();
}

impl View for StatusBarView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeStatusBar"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let (connection, connection_color) = connection_display(&self.model.connection);
        let (session_status, session_color) = session_status_display(&self.model);

        ConstrainedBox::new(
            Container::new(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_spacing(10.)
                    .with_child(self.item("conn", connection, connection_color))
                    .with_child(self.separator())
                    .with_child(self.item("session", session_status, session_color))
                    .with_child(self.separator())
                    .with_child(self.item("model", active_model_display(&self.model), text()))
                    .with_child(self.separator())
                    .with_child(self.item("id", active_session_id_display(&self.model), text()))
                    .finish(),
            )
            .with_uniform_padding(6.)
            .with_background_color(ColorU::new(18, 22, 30, 255))
            .finish(),
        )
        .with_height(STATUS_BAR_HEIGHT)
        .finish()
    }
}

pub fn status_bar_height() -> f32 {
    STATUS_BAR_HEIGHT
}

fn connection_display(status: &ConnectionStatus) -> (String, ColorU) {
    match status {
        ConnectionStatus::Disconnected => ("disconnected".to_string(), warning()),
        ConnectionStatus::Connecting => ("connecting".to_string(), pending()),
        ConnectionStatus::Connected => ("connected".to_string(), ok()),
        ConnectionStatus::Reconnecting { attempt } => {
            (format!("reconnecting #{attempt}"), pending())
        }
        ConnectionStatus::Error(message) => (truncate_middle(message, 28), danger()),
    }
}

fn session_status_display(model: &AppModel) -> (String, ColorU) {
    let Some(session_id) = model.active_session_id.as_deref() else {
        return ("none".to_string(), muted_text());
    };

    match model.statuses.get(session_id) {
        Some(SessionStatus::Idle) => ("idle".to_string(), ok()),
        Some(SessionStatus::Busy) => ("busy".to_string(), pending()),
        Some(SessionStatus::Retry {
            attempt, message, ..
        }) => (
            format!("retry #{attempt}: {}", truncate_middle(message, 18)),
            warning(),
        ),
        None => ("unknown".to_string(), muted_text()),
    }
}

fn active_model_display(model: &AppModel) -> String {
    let Some(providers) = model.providers.as_ref() else {
        return "unavailable".to_string();
    };

    default_provider_model(providers).unwrap_or_else(|| "unavailable".to_string())
}

fn default_provider_model(providers: &ProviderListResult) -> Option<String> {
    let mut defaults: Vec<_> = providers.default.iter().collect();
    defaults.sort_by(|(left, _), (right, _)| left.cmp(right));

    if let Some((provider_id, model_id)) = defaults.first() {
        return Some(format!(
            "{}/{}",
            provider_name(providers, provider_id),
            model_id
        ));
    }

    let mut connected = providers.connected.clone();
    connected.sort();
    connected
        .first()
        .map(|provider_id| format!("{}/-", provider_name(providers, provider_id)))
}

fn provider_name(providers: &ProviderListResult, provider_id: &str) -> String {
    providers
        .all
        .iter()
        .find(|provider| provider.id == provider_id)
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| provider_id.to_string())
}

fn active_session_id_display(model: &AppModel) -> String {
    model.active_session_id.as_deref().map_or_else(
        || "-".to_string(),
        |session_id| truncate_middle(session_id, SESSION_ID_CHARS),
    )
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    if max_chars <= 1 {
        return "…".to_string();
    }

    let left_count = (max_chars - 1) / 2;
    let right_count = max_chars - 1 - left_count;
    let left: String = value.chars().take(left_count).collect();
    let right: String = value
        .chars()
        .rev()
        .take(right_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!("{left}…{right}")
}

fn text() -> ColorU {
    ColorU::new(222, 226, 235, 255)
}

fn muted_text() -> ColorU {
    ColorU::new(143, 151, 166, 255)
}

fn ok() -> ColorU {
    ColorU::new(89, 214, 140, 255)
}

fn pending() -> ColorU {
    ColorU::new(94, 170, 255, 255)
}

fn warning() -> ColorU {
    ColorU::new(247, 190, 83, 255)
}

fn danger() -> ColorU {
    ColorU::new(255, 112, 112, 255)
}
