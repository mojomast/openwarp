//! Tool approval modal rendering.

use crate::api::schema::{PermissionId, PermissionRequest};
use crate::state::AppModel;
use crate::views::UiAction;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::{
    elements::{
        Border, ChildAnchor, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
        DispatchEventResult, Element, EventDispatchMode, EventHandler, Flex, Hoverable,
        MainAxisAlignment, MainAxisSize, MouseState, OffsetPositioning, ParentAnchor,
        ParentElement, ParentOffsetBounds, Radius, Stack, Text,
    },
    prelude::vec2f,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolApprovalAction {
    Allow(PermissionId),
    Deny(PermissionId),
}

pub fn render_tool_approval_overlay(
    base: Box<dyn Element>,
    model: &AppModel,
    font_family: FamilyId,
) -> Box<dyn Element> {
    let Some(request) = first_pending_permission(model) else {
        return base;
    };

    let mut stack = Stack::new().with_event_dispatch_mode(EventDispatchMode::Waterfall);
    stack.add_child(base);

    // Full-window scrim. The event handler prevents clicks from leaking through
    // the modal overlay into the underlying chat/session UI.
    let scrim = EventHandler::new(
        Container::new(
            Flex::column()
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_background_color(ColorU::new(0, 0, 0, 150))
        .finish(),
    )
    .on_left_mouse_down(|_, _, _| DispatchEventResult::StopPropagation)
    .on_left_mouse_up(|_, _, _| DispatchEventResult::StopPropagation)
    .finish();
    stack.add_overlay_child(scrim);

    stack.add_positioned_overlay_child(
        modal_for_request(request, font_family),
        OffsetPositioning::offset_from_parent(
            vec2f(0., 0.),
            ParentOffsetBounds::WindowByPosition,
            ParentAnchor::Center,
            ChildAnchor::Center,
        ),
    );

    stack.finish()
}

fn first_pending_permission(model: &AppModel) -> Option<&PermissionRequest> {
    let mut requests: Vec<_> = model.permissions.values().collect();
    requests.sort_by(|left, right| left.id.cmp(&right.id));
    requests.into_iter().next()
}

fn modal_for_request(request: &PermissionRequest, font_family: FamilyId) -> Box<dyn Element> {
    let body = Flex::column()
        .with_main_axis_size(MainAxisSize::Min)
        .with_spacing(12.)
        .with_child(text(
            "Tool approval required",
            font_family,
            20.,
            palette::text(),
        ))
        .with_child(text(
            format!("Permission: {}", request.permission),
            font_family,
            14.,
            palette::text_muted(),
        ))
        .with_child(detail_row("Session", &request.session_id, font_family))
        .with_child(detail_row(
            "Patterns",
            &request.patterns.join(", "),
            font_family,
        ))
        .with_child(json_panel("Metadata", &request.metadata, font_family))
        .with_child(json_panel("Args", &permission_args(request), font_family))
        .with_child(
            Flex::row()
                .with_main_axis_size(MainAxisSize::Max)
                .with_main_axis_alignment(MainAxisAlignment::End)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(10.)
                .with_child(action_button(
                    "Allow",
                    UiAction::AllowPermission(request.id.clone()),
                    ColorU::new(38, 92, 64, 255),
                    ColorU::new(48, 126, 82, 255),
                    font_family,
                ))
                .with_child(action_button(
                    "Always Allow",
                    UiAction::AlwaysAllowPermission(request.id.clone()),
                    ColorU::new(92, 71, 28, 255),
                    ColorU::new(132, 98, 34, 255),
                    font_family,
                ))
                .with_child(action_button(
                    "Deny",
                    UiAction::DenyPermission(request.id.clone()),
                    ColorU::new(80, 36, 40, 255),
                    ColorU::new(116, 48, 54, 255),
                    font_family,
                ))
                .finish(),
        )
        .finish();

    ConstrainedBox::new(
        Container::new(body)
            .with_uniform_padding(20.)
            .with_background_color(palette::surface())
            .with_border(Border::all(1.).with_border_color(palette::border()))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(12.)))
            .finish(),
    )
    .with_width(700.)
    .finish()
}

fn detail_row(label: &str, value: &str, font_family: FamilyId) -> Box<dyn Element> {
    Flex::row()
        .with_main_axis_size(MainAxisSize::Max)
        .with_spacing(8.)
        .with_child(
            ConstrainedBox::new(text(label, font_family, 12., palette::text_muted()))
                .with_width(78.)
                .finish(),
        )
        .with_child(text(
            empty_fallback(value),
            font_family,
            12.,
            palette::text(),
        ))
        .finish()
}

fn json_panel(label: &str, value: &Value, font_family: FamilyId) -> Box<dyn Element> {
    let pretty =
        serde_json::to_string_pretty(value).unwrap_or_else(|_| String::from("<unprintable>"));
    Container::new(
        Flex::column()
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(6.)
            .with_child(text(label, font_family, 12., palette::text_muted()))
            .with_child(text(pretty, font_family, 12., palette::code()))
            .finish(),
    )
    .with_uniform_padding(12.)
    .with_background_color(palette::code_bg())
    .with_border(Border::all(1.).with_border_color(palette::border_subtle()))
    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
    .finish()
}

fn permission_args(request: &PermissionRequest) -> Value {
    request
        .metadata
        .get("args")
        .or_else(|| request.metadata.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn action_button(
    label: &'static str,
    action: UiAction,
    background: ColorU,
    hover_background: ColorU,
    font_family: FamilyId,
) -> Box<dyn Element> {
    let mouse = Arc::new(Mutex::new(MouseState::default()));
    Hoverable::new(mouse, move |state| {
        let fill = if state.is_hovered() {
            hover_background
        } else {
            background
        };
        Container::new(text(label, font_family, 13., palette::text()))
            .with_horizontal_padding(18.)
            .with_vertical_padding(9.)
            .with_background_color(fill)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
            .finish()
    })
    .on_click(move |ctx, _, _| {
        ctx.dispatch_typed_action(action.clone());
    })
    .finish()
}

fn text(
    value: impl Into<String>,
    font_family: FamilyId,
    size: f32,
    color: ColorU,
) -> Box<dyn Element> {
    Text::new(value.into(), font_family, size)
        .with_color(color)
        .with_line_height_ratio(1.25)
        .finish()
}

fn empty_fallback(value: &str) -> String {
    if value.is_empty() {
        "—".to_string()
    } else {
        value.to_string()
    }
}

mod palette {
    use warpui::color::ColorU;

    pub fn surface() -> ColorU {
        ColorU::new(18, 22, 30, 255)
    }
    pub fn code_bg() -> ColorU {
        ColorU::new(10, 13, 18, 255)
    }
    pub fn border() -> ColorU {
        ColorU::new(72, 82, 104, 255)
    }
    pub fn border_subtle() -> ColorU {
        ColorU::new(42, 50, 66, 255)
    }
    pub fn text() -> ColorU {
        ColorU::new(235, 239, 246, 255)
    }
    pub fn text_muted() -> ColorU {
        ColorU::new(158, 168, 184, 255)
    }
    pub fn code() -> ColorU {
        ColorU::new(206, 218, 235, 255)
    }
}
