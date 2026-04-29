//! Minimal PTY panel overlay shell.

use crate::state::AppModel;
use warpui::color::ColorU;
use warpui::elements::{ConstrainedBox, Container, Element, Flex, ParentElement, Text};
use warpui::fonts::FamilyId;

pub fn render_pty_panel(model: &AppModel, font_family: FamilyId) -> Box<dyn Element> {
    let title = model
        .ptys
        .values()
        .next()
        .map(|pty| format!("PTY: {} ({})", pty.title, pty.status))
        .unwrap_or_else(|| "PTY: no active terminal".to_string());

    ConstrainedBox::new(
        Container::new(
            Flex::column()
                .with_spacing(8.)
                .with_child(label(title, font_family, 14., ColorU::new(226, 232, 240, 255)))
                .with_child(label(
                    "Terminal rendering is not implemented yet; transport is available in pty::client.",
                    font_family,
                    12.,
                    ColorU::new(148, 163, 184, 255),
                ))
                .finish(),
        )
        .with_uniform_padding(16.)
        .with_background_color(ColorU::new(8, 12, 18, 245))
        .finish(),
    )
    .with_height(260.)
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
