//! Live PTY panel backed by the WebSocket PTY stream.

use crate::api::ApiClient;
use crate::pty::{
    xterm_256_color, Cell, CellColor, PtyClient, PtySession, PtyState, DEFAULT_FOREGROUND,
};
use crate::state::AppModel;
use warpui::color::ColorU;
use warpui::elements::{
    ConstrainedBox, Container, DispatchEventResult, Element, EventHandler, Flex, ParentElement,
    Text,
};
use warpui::fonts::FamilyId;
use warpui::{AppContext, Entity, TypedActionView, View, ViewContext};

const DEFAULT_ROWS: usize = 12;
const DEFAULT_COLS: usize = 96;

#[derive(Debug, Clone)]
pub enum PtyPanelAction {
    Send(String),
    PageUp,
    PageDown,
}

pub struct PtyPanelView {
    font_family: FamilyId,
    client: PtyClient,
    model: AppModel,
    pty_id: Option<String>,
    session: Option<PtySession>,
    state_snapshot: PtyState,
    scroll_offset: usize,
    focused: bool,
}

impl PtyPanelView {
    pub fn new(font_family: FamilyId, api: ApiClient, model: AppModel) -> Self {
        Self {
            font_family,
            client: PtyClient::new(api),
            model,
            pty_id: None,
            session: None,
            state_snapshot: PtyState::new(DEFAULT_ROWS, DEFAULT_COLS),
            scroll_offset: 0,
            focused: true,
        }
    }

    pub fn set_snapshot(&mut self, model: AppModel, ctx: &mut ViewContext<Self>) {
        let next_pty_id = model.ptys.values().next().map(|pty| pty.id.clone());
        self.model = model;
        self.set_pty_id(next_pty_id, ctx);
        ctx.notify();
    }

    pub fn set_pty_id(&mut self, pty_id: Option<String>, ctx: &mut ViewContext<Self>) {
        if self.pty_id == pty_id {
            return;
        }

        if let Some(session) = self.session.take() {
            session.abort();
        }

        self.pty_id = pty_id.clone();
        self.scroll_offset = 0;
        self.state_snapshot = PtyState::new(DEFAULT_ROWS, DEFAULT_COLS);

        let Some(pty_id) = pty_id else {
            ctx.notify();
            return;
        };

        let client = self.client.clone();
        ctx.spawn(
            async move {
                PtySession::connect(client, pty_id.clone(), DEFAULT_ROWS, DEFAULT_COLS)
                    .await
                    .map(|session| (pty_id, session))
            },
            |view, result, ctx| match result {
                Ok((pty_id, session)) if view.pty_id.as_deref() == Some(pty_id.as_str()) => {
                    view.state_snapshot = session.state_rx.borrow().clone();
                    let mut state_rx = session.state_rx.clone();
                    view.session = Some(session);
                    view.start_state_watcher(pty_id, &mut state_rx, ctx);
                    ctx.notify();
                }
                Ok((_pty_id, session)) => session.abort(),
                Err(error) => {
                    view.state_snapshot
                        .feed(format!("\x1b[1;31mPTY connection failed:\x1b[0m {error}"));
                    ctx.notify();
                }
            },
        );
    }

    fn start_state_watcher(
        &self,
        pty_id: String,
        state_rx: &mut tokio::sync::watch::Receiver<PtyState>,
        ctx: &mut ViewContext<Self>,
    ) {
        let mut state_rx = state_rx.clone();
        let spawner = ctx.spawner();
        tokio::spawn(async move {
            while state_rx.changed().await.is_ok() {
                let snapshot = state_rx.borrow().clone();
                let pty_id = pty_id.clone();
                let _ = spawner
                    .spawn(move |view, ctx| {
                        if view.pty_id.as_deref() == Some(pty_id.as_str()) {
                            view.state_snapshot = snapshot;
                            ctx.notify();
                        }
                    })
                    .await;
            }
        });
    }

    fn title(&self) -> String {
        self.pty_id
            .as_deref()
            .and_then(|id| self.model.ptys.get(id))
            .map(|pty| format!("PTY: {} ({})", pty.title, pty.status))
            .unwrap_or_else(|| "PTY: no active terminal".to_string())
    }

    fn visible_rows(&self) -> usize {
        self.state_snapshot.grid().rows()
    }
}

impl Entity for PtyPanelView {
    type Event = ();
}

impl TypedActionView for PtyPanelView {
    type Action = PtyPanelAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PtyPanelAction::Send(input) => {
                if let Some(session) = &self.session {
                    let _ = session.input_tx.try_send(input.clone());
                }
            }
            PtyPanelAction::PageUp => {
                let max = self.state_snapshot.grid().scrollback_len();
                self.scroll_offset = (self.scroll_offset + self.visible_rows()).min(max);
                ctx.notify();
            }
            PtyPanelAction::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(self.visible_rows());
                ctx.notify();
            }
        }
    }
}

impl View for PtyPanelView {
    fn ui_name() -> &'static str {
        "WarpOpenCodePtyPanel"
    }

    fn render(&self, _: &AppContext) -> Box<dyn Element> {
        let mut lines = Flex::column().with_spacing(1.);
        for row in self
            .state_snapshot
            .grid()
            .render_region(0, self.state_snapshot.grid().rows())
        {
            lines = lines.with_child(render_row(
                row.into_iter().map(|cell| cell.cell),
                self.font_family,
            ));
        }

        let content = ConstrainedBox::new(
            Container::new(
                Flex::column()
                    .with_spacing(8.)
                    .with_child(label(
                        self.title(),
                        self.font_family,
                        14.,
                        if self.focused {
                            ColorU::new(226, 232, 240, 255)
                        } else {
                            ColorU::new(148, 163, 184, 255)
                        },
                    ))
                    .with_child(lines.finish())
                    .finish(),
            )
            .with_uniform_padding(16.)
            .with_background_color(ColorU::new(8, 12, 18, 245))
            .finish(),
        )
        .with_height(260.)
        .finish();

        EventHandler::new(content)
            .with_always_handle()
            .on_keydown(|ctx, _app, keystroke| {
                let key = keystroke.key.as_str();
                let action = match key {
                    "up" | "arrowup" => PtyPanelAction::Send("\x1b[A".to_string()),
                    "down" | "arrowdown" => PtyPanelAction::Send("\x1b[B".to_string()),
                    "right" | "arrowright" => PtyPanelAction::Send("\x1b[C".to_string()),
                    "left" | "arrowleft" => PtyPanelAction::Send("\x1b[D".to_string()),
                    "enter" => PtyPanelAction::Send("\r".to_string()),
                    "backspace" => PtyPanelAction::Send("\x7f".to_string()),
                    "pageup" => PtyPanelAction::PageUp,
                    "pagedown" => PtyPanelAction::PageDown,
                    "c" if keystroke.ctrl => PtyPanelAction::Send("\x03".to_string()),
                    "d" if keystroke.ctrl => PtyPanelAction::Send("\x04".to_string()),
                    "z" if keystroke.ctrl => PtyPanelAction::Send("\x1a".to_string()),
                    "l" if keystroke.ctrl => PtyPanelAction::Send("\x0c".to_string()),
                    key if is_plain_printable_key(key, keystroke) => {
                        PtyPanelAction::Send(key.to_string())
                    }
                    _ => return DispatchEventResult::PropagateToParent,
                };
                ctx.dispatch_typed_action(action);
                DispatchEventResult::StopPropagation
            })
            .finish()
    }
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

fn is_plain_printable_key(key: &str, keystroke: &warpui::keymap::Keystroke) -> bool {
    !keystroke.ctrl
        && !keystroke.alt
        && !keystroke.cmd
        && !keystroke.meta
        && key.chars().count() == 1
}
