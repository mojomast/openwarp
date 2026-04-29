//! Snapshot PTY panel overlay.

use crate::pty::{xterm_256_color, Cell, CellColor, PtyState, DEFAULT_FOREGROUND};
use crate::state::AppModel;
use warpui::color::ColorU;
use warpui::elements::{ConstrainedBox, Container, Element, Flex, ParentElement, Text};
use warpui::fonts::FamilyId;

pub fn render_pty_panel(model: &AppModel, font_family: FamilyId) -> Box<dyn Element> {
    let active = model.ptys.values().next();
    let title = active
        .map(|pty| format!("PTY: {} ({})", pty.title, pty.status))
        .unwrap_or_else(|| "PTY: no active terminal".to_string());

    // The websocket transport exists in `pty::client`, but RootView does not yet
    // own a live PTY subscription. Render a local VTE-backed grid so the panel,
    // color mapping, and future integration API are exercised without coupling
    // this view to async transport state.
    let mut state = PtyState::new(8, 96);
    if let Some(pty) = active {
        state.feed(format!(
            "\x1b[1;36m{}\x1b[0m  {}\r\n\x1b[90m{}\x1b[0m\r\n\r\nWaiting for live PTY stream integration…",
            pty.command, pty.args.join(" "), pty.cwd
        ));
    } else {
        state.feed("No active terminal. Toggle PTY after opencode creates one.");
    }

    let mut lines = Flex::column().with_spacing(1.);
    for row in state.grid().render_region(0, state.grid().rows()) {
        lines = lines.with_child(render_row(
            row.into_iter().map(|cell| cell.cell),
            font_family,
        ));
    }

    ConstrainedBox::new(
        Container::new(
            Flex::column()
                .with_spacing(8.)
                .with_child(label(
                    title,
                    font_family,
                    14.,
                    ColorU::new(226, 232, 240, 255),
                ))
                .with_child(lines.finish())
                .finish(),
        )
        .with_uniform_padding(16.)
        .with_background_color(ColorU::new(8, 12, 18, 245))
        .finish(),
    )
    .with_height(260.)
    .finish()
}

fn render_row(cells: impl Iterator<Item = Cell>, font_family: FamilyId) -> Box<dyn Element> {
    let mut row = Flex::row();
    let mut current_color: Option<ColorU> = None;
    let mut buffer = String::new();

    for cell in cells {
        let color = color_for_cell(&cell);
        if current_color.is_some_and(|existing| existing != color) && !buffer.is_empty() {
            row = row.with_child(label(
                std::mem::take(&mut buffer),
                font_family,
                12.,
                current_color.unwrap(),
            ));
        }
        current_color = Some(color);
        buffer.push(cell.ch);
    }

    if !buffer.is_empty() {
        row = row.with_child(label(
            buffer,
            font_family,
            12.,
            current_color.unwrap_or_else(default_fg),
        ));
    }

    row.finish()
}

fn color_for_cell(cell: &Cell) -> ColorU {
    let color = if cell.inverse { cell.bg } else { cell.fg };
    let (r, g, b) = match color {
        CellColor::Default => DEFAULT_FOREGROUND,
        CellColor::Indexed(index) => xterm_256_color(index),
        CellColor::Rgb(r, g, b) => (r, g, b),
    };
    if cell.bold {
        ColorU::new(
            r.saturating_add(24),
            g.saturating_add(24),
            b.saturating_add(24),
            255,
        )
    } else {
        ColorU::new(r, g, b, 255)
    }
}

fn default_fg() -> ColorU {
    let (r, g, b) = DEFAULT_FOREGROUND;
    ColorU::new(r, g, b, 255)
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
