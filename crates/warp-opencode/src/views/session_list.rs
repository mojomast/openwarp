//! Reusable session-list rendering helpers.
//!
//! This module intentionally renders from an already-owned snapshot of app state.  WarpUI render
//! methods are synchronous, so callers should refresh the snapshot outside render (for example via
//! an `AppStore::subscribe` task) and then pass it to [`SessionListPanel::render_snapshot`].

use crate::api::schema::{Session, SessionId};
use crate::state::AppModel;
use crate::views::UiAction;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::{
    elements::{
        Container, CrossAxisAlignment, DispatchEventResult, Element, EventHandler, Flex,
        MainAxisAlignment, MainAxisSize, ParentElement, Text,
    },
    AppContext, Entity, TypedActionView, View,
};

pub const NEW_SESSION_LABEL: &str = "New Session";

fn panel_bg() -> ColorU {
    ColorU::new(24, 28, 36, 255)
}
fn header_text() -> ColorU {
    ColorU::new(238, 242, 255, 255)
}
fn body_text() -> ColorU {
    ColorU::new(218, 224, 238, 255)
}
fn muted_text() -> ColorU {
    ColorU::new(142, 152, 170, 255)
}
fn button_bg() -> ColorU {
    ColorU::new(58, 92, 178, 255)
}
fn row_bg() -> ColorU {
    ColorU::new(31, 36, 47, 255)
}
fn active_row_bg() -> ColorU {
    ColorU::new(67, 92, 142, 255)
}
fn empty_bg() -> ColorU {
    ColorU::new(20, 23, 30, 255)
}

/// Snapshot data required to render the session sidebar.
#[derive(Debug, Clone, Default)]
pub struct SessionListSnapshot {
    pub sessions: Vec<Session>,
    pub active_session_id: Option<SessionId>,
}

impl SessionListSnapshot {
    pub fn new(sessions: Vec<Session>, active_session_id: Option<SessionId>) -> Self {
        Self {
            sessions,
            active_session_id,
        }
    }
}

impl From<&AppModel> for SessionListSnapshot {
    fn from(model: &AppModel) -> Self {
        Self::new(model.sessions.clone(), model.active_session_id.clone())
    }
}

/// Typed actions emitted by the reusable session list elements.
#[derive(Debug, Clone)]
pub enum SessionListAction {
    NewSession,
    SelectSession(SessionId),
}

/// Stateless renderer for the session list panel.
pub struct SessionListPanel {
    font_family: FamilyId,
}

impl SessionListPanel {
    pub fn new(font_family: FamilyId) -> Self {
        Self { font_family }
    }

    /// Renders the sidebar from the supplied snapshot without awaiting store state.
    pub fn render_snapshot(&self, snapshot: &SessionListSnapshot) -> Box<dyn Element> {
        render_session_list(snapshot, self.font_family)
    }
}

impl Entity for SessionListPanel {
    type Event = ();
}

impl TypedActionView for SessionListPanel {
    type Action = UiAction;
}

impl View for SessionListPanel {
    fn ui_name() -> &'static str {
        "WarpOpenCodeSessionList"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        self.render_snapshot(&SessionListSnapshot::default())
    }
}

/// Convenience helper for callers that do not need to construct [`SessionListPanel`].
pub fn render_session_list(
    snapshot: &SessionListSnapshot,
    font_family: FamilyId,
) -> Box<dyn Element> {
    let mut rows = Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_spacing(6.);

    if snapshot.sessions.is_empty() {
        rows = rows.with_child(empty_state(font_family));
    } else {
        for session in &snapshot.sessions {
            let is_active = snapshot.active_session_id.as_deref() == Some(session.id.as_str());
            rows = rows.with_child(session_row(session, is_active, font_family));
        }
    }

    Container::new(
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(12.)
            .with_child(label("Sessions", font_family, 18., header_text()))
            .with_child(new_session_button(font_family))
            .with_child(rows.finish())
            .finish(),
    )
    .with_uniform_padding(16.)
    .with_background_color(panel_bg())
    .finish()
}

fn new_session_button(font_family: FamilyId) -> Box<dyn Element> {
    let button = Container::new(
        Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(label(
                format!("+ {NEW_SESSION_LABEL}"),
                font_family,
                14.,
                header_text(),
            ))
            .finish(),
    )
    .with_vertical_padding(9.)
    .with_horizontal_padding(12.)
    .with_background_color(button_bg())
    .finish();

    EventHandler::new(button)
        .on_left_mouse_up(|ctx, _app, _position| {
            ctx.dispatch_typed_action(UiAction::NewSession);
            DispatchEventResult::StopPropagation
        })
        .finish()
}

fn session_row(session: &Session, is_active: bool, font_family: FamilyId) -> Box<dyn Element> {
    let title = if session.title.trim().is_empty() {
        session
            .slug
            .as_deref()
            .filter(|slug| !slug.trim().is_empty())
            .unwrap_or("Untitled session")
            .to_owned()
    } else {
        session.title.clone()
    };
    let timestamp = format_session_time(&session.time);
    let row_bg = if is_active { active_row_bg() } else { row_bg() };
    let session_id = session.id.clone();

    let row = Container::new(
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(4.)
            .with_child(label(title, font_family, 14., body_text()))
            .with_child(label(timestamp, font_family, 12., muted_text()))
            .finish(),
    )
    .with_vertical_padding(9.)
    .with_horizontal_padding(10.)
    .with_background_color(row_bg)
    .finish();

    EventHandler::new(row)
        .on_left_mouse_up(move |ctx, _app, _position| {
            ctx.dispatch_typed_action(UiAction::SelectSession(session_id.clone()));
            DispatchEventResult::StopPropagation
        })
        .finish()
}

fn empty_state(font_family: FamilyId) -> Box<dyn Element> {
    Container::new(label(
        "No sessions yet. Create one to get started.",
        font_family,
        13.,
        muted_text(),
    ))
    .with_uniform_padding(10.)
    .with_background_color(empty_bg())
    .finish()
}

fn label(
    text: impl Into<String>,
    font_family: FamilyId,
    size: f32,
    color: ColorU,
) -> Box<dyn Element> {
    Text::new(text.into(), font_family, size)
        .with_color(color)
        .finish()
}

fn format_session_time(time: &Value) -> String {
    let Some(epoch_millis) = extract_epoch_millis(time) else {
        return "recently".to_string();
    };

    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i128)
        .unwrap_or_default();
    let age_seconds = ((now_millis - epoch_millis as i128) / 1_000).max(0);

    if age_seconds < 60 {
        "just now".to_string()
    } else if age_seconds < 3_600 {
        format!("{}m ago", age_seconds / 60)
    } else if age_seconds < 86_400 {
        format!("{}h ago", age_seconds / 3_600)
    } else if age_seconds < 604_800 {
        format!("{}d ago", age_seconds / 86_400)
    } else {
        format!("{}w ago", age_seconds / 604_800)
    }
}

fn extract_epoch_millis(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value as i64))
            .map(normalize_epoch_millis),
        Value::String(text) => text.parse::<i64>().ok().map(normalize_epoch_millis),
        Value::Object(map) => ["updated", "modified", "created", "time", "timestamp"]
            .iter()
            .find_map(|key| map.get(*key).and_then(extract_epoch_millis)),
        _ => None,
    }
}

fn normalize_epoch_millis(value: i64) -> i64 {
    // Treat small epoch values as seconds, larger values as milliseconds.
    if value.abs() < 10_000_000_000 {
        value.saturating_mul(1_000)
    } else {
        value
    }
}
