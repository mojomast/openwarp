//! Snapshot-driven chat thread rendering.
//!
//! The view intentionally renders only from the `AppModel` snapshot stored on the
//! view.  Store subscription and async snapshot refresh should happen outside of
//! `View::render` (for example from a background task that updates this view and
//! calls `ctx.notify()`).

use crate::api::schema::{MessageWithParts, Part, SessionStatus};
use crate::state::{AppModel, SessionThread};
use markdown_parser::parse_markdown;
use serde_json::Value;
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::scene::{CornerRadius, Radius};
use warpui::SingletonEntity as _;
use warpui::{
    elements::{
        Border, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container, Element,
        Fill, Flex, FormattedTextElement, MainAxisAlignment, MainAxisSize, ParentElement,
        ScrollbarWidth, Text,
    },
    AppContext, Entity, View, ViewContext,
};

const FONT_SIZE: f32 = 13.;
const SMALL_FONT_SIZE: f32 = 11.;
const BUBBLE_MAX_WIDTH: f32 = 760.;

/// Renders the active thread in an [`AppModel`] snapshot.
pub struct ChatThreadView {
    snapshot: AppModel,
    scroll_state: ClippedScrollStateHandle,
    font_family: FamilyId,
    code_font_family: FamilyId,
}

impl ChatThreadView {
    pub fn new(ctx: &mut ViewContext<Self>, snapshot: AppModel) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());
        let code_font_family = warpui::fonts::Cache::handle(ctx).update(ctx, |cache, _| {
            cache
                .load_system_font("monospace")
                .or_else(|_| cache.load_system_font("Monospace"))
                .unwrap_or(font_family)
        });

        Self {
            snapshot,
            scroll_state: ClippedScrollStateHandle::default(),
            font_family,
            code_font_family,
        }
    }

    /// Replace the render snapshot. Callers should invoke `ctx.notify()` after
    /// updating the view so WarpUI schedules a repaint.
    pub fn set_snapshot(&mut self, snapshot: AppModel) {
        self.snapshot = snapshot;
    }

    fn text(&self, text: impl Into<String>, size: f32, color: ColorU) -> Box<dyn Element> {
        Text::new(text.into(), self.font_family, size)
            .with_color(color)
            .finish()
    }

    fn markdown_or_text(&self, text: &str) -> Box<dyn Element> {
        match parse_markdown(text) {
            Ok(markdown) => FormattedTextElement::new(
                markdown,
                FONT_SIZE,
                self.font_family,
                self.code_font_family,
                colors::text(),
                Default::default(),
            )
            .with_line_height_ratio(1.35)
            .set_selectable(true)
            .finish(),
            Err(_) => FormattedTextElement::from_str(text.to_owned(), self.font_family, FONT_SIZE)
                .with_color(colors::text())
                .with_line_height_ratio(1.35)
                .set_selectable(true)
                .finish(),
        }
    }

    fn active_thread(&self) -> Option<(&SessionThread, Option<&SessionStatus>)> {
        let session_id = self.snapshot.active_session_id.as_ref()?;
        Some((
            self.snapshot.threads.get(session_id)?,
            self.snapshot.statuses.get(session_id),
        ))
    }

    fn render_message(&self, message: &MessageWithParts) -> Box<dyn Element> {
        if message.info.role == "user" {
            self.render_user_message(message)
        } else {
            self.render_assistant_message(message)
        }
    }

    fn render_user_message(&self, message: &MessageWithParts) -> Box<dyn Element> {
        let text = collect_text(message).unwrap_or_else(|| "(empty message)".to_string());
        let bubble = Container::new(self.text(text, FONT_SIZE, colors::user_text()))
            .with_uniform_padding(12.)
            .with_background_color(colors::user_bubble())
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(14.)))
            .finish();

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_child(
                ConstrainedBox::new(bubble)
                    .with_max_width(BUBBLE_MAX_WIDTH)
                    .finish(),
            )
            .finish()
    }

    fn render_assistant_message(&self, message: &MessageWithParts) -> Box<dyn Element> {
        let mut column = Flex::column().with_spacing(8.);

        for part in &message.parts {
            column = column.with_child(self.render_part(part));
        }

        if message.parts.is_empty() {
            column = column.with_child(self.text(
                "(assistant response pending)",
                FONT_SIZE,
                colors::muted(),
            ));
        }

        Container::new(column.finish())
            .with_horizontal_padding(4.)
            .with_vertical_padding(2.)
            .finish()
    }

    fn render_part(&self, part: &Part) -> Box<dyn Element> {
        let kind = part.kind.as_str();
        if kind == "step-start" || kind == "step_start" {
            return self.render_step_start(part);
        }

        if kind.contains("tool") || kind.contains("result") {
            return self.render_tool_or_result(part);
        }

        if let Some(text) = part.text.as_deref().filter(|text| !text.trim().is_empty()) {
            return self.markdown_or_text(text);
        }

        // Fallback for non-text assistant parts: show their type and compact JSON
        // metadata rather than dropping potentially important streamed data.
        self.render_metadata_block(kind, part)
    }

    fn render_step_start(&self, part: &Part) -> Box<dyn Element> {
        let label = part
            .text
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or("Step started");

        Container::new(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_child(self.text(format!("── {label}"), SMALL_FONT_SIZE, colors::muted()))
                .finish(),
        )
        .with_vertical_padding(8.)
        .finish()
    }

    fn render_tool_or_result(&self, part: &Part) -> Box<dyn Element> {
        let title = tool_title(part);
        let body = part
            .text
            .clone()
            .or_else(|| compact_json(part.state.as_ref()))
            .or_else(|| {
                compact_json_value(&Value::Object(part.extra.clone().into_iter().collect()))
            })
            .unwrap_or_else(|| "No details".to_string());

        self.block(
            colors::tool_bg(),
            colors::tool_border(),
            Flex::column()
                .with_spacing(6.)
                .with_child(self.text(title, SMALL_FONT_SIZE, colors::muted()))
                .with_child(
                    Text::new(body, self.code_font_family, 12.)
                        .with_color(colors::text())
                        .finish(),
                )
                .finish(),
        )
    }

    fn render_metadata_block(&self, kind: &str, part: &Part) -> Box<dyn Element> {
        let body = compact_json(part.state.as_ref())
            .or_else(|| {
                compact_json_value(&Value::Object(part.extra.clone().into_iter().collect()))
            })
            .unwrap_or_else(|| "No renderable content".to_string());

        self.block(
            colors::meta_bg(),
            colors::meta_border(),
            Flex::column()
                .with_spacing(6.)
                .with_child(self.text(format!("{kind} part"), SMALL_FONT_SIZE, colors::muted()))
                .with_child(
                    Text::new(body, self.code_font_family, 12.)
                        .with_color(colors::text())
                        .finish(),
                )
                .finish(),
        )
    }

    fn block(&self, bg: ColorU, border: ColorU, child: Box<dyn Element>) -> Box<dyn Element> {
        Container::new(child)
            .with_uniform_padding(10.)
            .with_background_color(bg)
            .with_border(Border::all(1.).with_border_color(border))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
            .finish()
    }

    fn render_busy_indicator(&self, status: &SessionStatus) -> Option<Box<dyn Element>> {
        let label = match status {
            SessionStatus::Busy => "Assistant is working…".to_string(),
            SessionStatus::Retry {
                attempt, message, ..
            } => {
                format!("Retrying (attempt {attempt}): {message}")
            }
            SessionStatus::Idle => return None,
        };

        Some(
            Container::new(self.text(label, FONT_SIZE, colors::muted()))
                .with_uniform_padding(10.)
                .with_background_color(colors::busy_bg())
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(10.)))
                .finish(),
        )
    }
}

impl Entity for ChatThreadView {
    type Event = ();
}

impl View for ChatThreadView {
    fn ui_name() -> &'static str {
        "WarpOpenCodeChatThread"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let mut messages = Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(14.);

        if let Some((thread, status)) = self.active_thread() {
            if thread.messages.is_empty() {
                messages = messages.with_child(self.text(
                    "No messages yet. Send a prompt to start the conversation.",
                    FONT_SIZE,
                    colors::muted(),
                ));
            } else {
                for message in &thread.messages {
                    messages = messages.with_child(self.render_message(message));
                }
            }

            if let Some(status) = status.and_then(|status| self.render_busy_indicator(status)) {
                messages = messages.with_child(status);
            }
        } else {
            messages = messages.with_child(self.text(
                "Select or create a session to view its thread.",
                FONT_SIZE,
                colors::muted(),
            ));
        }

        let content = Container::new(messages.finish())
            .with_uniform_padding(18.)
            .finish();

        ClippedScrollable::vertical(
            self.scroll_state.clone(),
            content,
            ScrollbarWidth::Auto,
            Fill::Solid(colors::scrollbar()),
            Fill::Solid(colors::scrollbar_active()),
            Fill::Solid(colors::scrollbar_track()),
        )
        .finish()
    }
}

fn collect_text(message: &MessageWithParts) -> Option<String> {
    let text = message
        .parts
        .iter()
        .filter_map(|part| part.text.as_deref())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

fn tool_title(part: &Part) -> String {
    let name = part
        .extra
        .get("tool")
        .or_else(|| part.extra.get("name"))
        .or_else(|| part.extra.get("callID"))
        .and_then(Value::as_str)
        .unwrap_or(part.kind.as_str());
    format!(
        "{} · {}",
        if part.kind.contains("result") {
            "Result"
        } else {
            "Tool"
        },
        name
    )
}

fn compact_json(value: Option<&Value>) -> Option<String> {
    value.and_then(compact_json_value)
}

fn compact_json_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Object(map) if map.is_empty() => None,
        other => serde_json::to_string_pretty(other).ok(),
    }
}

mod colors {
    use warpui::color::ColorU;

    pub fn text() -> ColorU {
        ColorU::new(229, 234, 242, 255)
    }
    pub fn muted() -> ColorU {
        ColorU::new(145, 153, 166, 255)
    }
    pub fn user_text() -> ColorU {
        ColorU::new(255, 255, 255, 255)
    }
    pub fn user_bubble() -> ColorU {
        ColorU::new(44, 92, 255, 255)
    }
    pub fn tool_bg() -> ColorU {
        ColorU::new(22, 27, 36, 255)
    }
    pub fn tool_border() -> ColorU {
        ColorU::new(55, 65, 81, 255)
    }
    pub fn meta_bg() -> ColorU {
        ColorU::new(18, 22, 30, 255)
    }
    pub fn meta_border() -> ColorU {
        ColorU::new(39, 45, 58, 255)
    }
    pub fn busy_bg() -> ColorU {
        ColorU::new(29, 34, 45, 255)
    }
    pub fn scrollbar() -> ColorU {
        ColorU::new(78, 87, 104, 120)
    }
    pub fn scrollbar_active() -> ColorU {
        ColorU::new(108, 119, 140, 180)
    }
    pub fn scrollbar_track() -> ColorU {
        ColorU::new(0, 0, 0, 0)
    }
}
