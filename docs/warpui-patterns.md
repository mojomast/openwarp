# WarpUI Patterns

Source reviewed: local checkout at `/home/mojo/projects/openwarp/Warp`, focused on `crates/warpui_core/src`, `crates/warpui/src`, and practical `app/src` views. `warpui` re-exports `warpui_core` (`crates/warpui/src/lib.rs`).

## 1. View Trait Signatures

`View` is defined in `crates/warpui_core/src/core/view/mod.rs` and extends `Entity`.

Required:

```rust
impl Entity for MyView {
    type Event = ();
}

impl View for MyView {
    fn ui_name() -> &'static str;
    fn render(&self, app: &AppContext) -> Box<dyn Element>;
}
```

Optional `View` hooks with exact signatures:

```rust
fn on_focus(&mut self, _focus_ctx: &FocusContext, _ctx: &mut ViewContext<Self>) {}
fn accessibility_contents(&self, _ctx: &AppContext) -> Option<AccessibilityContent> { None }
fn active_cursor_position(&self, _ctx: &ViewContext<Self>) -> Option<CursorInfo> { None }
fn on_blur(&mut self, _blur_ctx: &BlurContext, _ctx: &mut ViewContext<Self>) {}
fn on_window_closed(&mut self, _ctx: &mut ViewContext<Self>) {}
fn on_window_transferred(
    &mut self,
    _source_window_id: WindowId,
    _target_window_id: WindowId,
    _ctx: &mut ViewContext<Self>,
) {}
fn keymap_context(&self, _: &AppContext) -> keymap::Context { Self::default_keymap_context() }
fn default_keymap_context() -> keymap::Context;
fn self_or_child_interacted_with(&self, _ctx: &mut ViewContext<Self>) {}
fn accessibility_data(&self, _ctx: &mut ViewContext<Self>) -> Option<AccessibilityData> { None }
```

Typed actions are separate:

```rust
impl TypedActionView for MyView {
    type Action = MyAction;
    fn handle_action(&mut self, _action: &Self::Action, _ctx: &mut ViewContext<Self>) {}
    fn action_accessibility_contents(
        &mut self,
        _action: &Self::Action,
        _ctx: &mut ViewContext<Self>,
    ) -> ActionAccessibilityContent { ActionAccessibilityContent::default() }
}
```

## 2. Text And Labels

Core text element:

```rust
Text::new(text, family_id, font_size).finish()
Text::new_inline(text, family_id, font_size).finish()
```

Important behavior:

- `Text::new(text: impl Into<Cow<'static, str>>, family_id: FamilyId, font_size: f32) -> Self` soft-wraps by default.
- `Text::new_inline(...)` sets `soft_wrap: false`; comments advise using `Text::new(...).soft_wrap(false)` instead.
- Common builders: `with_color(ColorU)`, `with_selection_color(ColorU)`, `with_line_height_ratio(f32)`, `with_style(Properties)`, `soft_wrap(bool)`, `with_selectable(bool)`, `with_highlights(...)`, `with_single_highlight(...)`.
- Labels usually need a loaded `FamilyId`, e.g. `warpui::fonts::Cache::handle(ctx).update(ctx, |cache, _| cache.load_system_font("Arial").unwrap())`.

UI component wrappers in `ui_components/text.rs`:

```rust
Span::new(text, UiComponentStyles).build().finish()
Paragraph::new(text, UiComponentStyles).build().finish()
WrappableText::new(text.into(), soft_wrap, UiComponentStyles).build().finish()
```

These require `UiComponent` in scope for `.build()` and `font_family_id` in `UiComponentStyles` must be set or `unwrap()` will panic.

## 3. Flex Rows, Columns, Sizing

Core constructors and builders:

```rust
Flex::row()
Flex::column()
Flex::new(Axis::Horizontal | Axis::Vertical)
.with_main_axis_size(MainAxisSize::Min | MainAxisSize::Max)
.with_main_axis_alignment(MainAxisAlignment::Start | SpaceBetween | SpaceEvenly | Center | End)
.with_cross_axis_alignment(CrossAxisAlignment::Start | Center | End | Stretch)
.with_spacing(f32)
.with_child(Box<dyn Element>)
.with_children(iter)
.finish()
```

Flexible children:

```rust
Expanded::new(flex: f32, child: Box<dyn Element>).finish()
Shrinkable::new(flex: f32, child: Box<dyn Element>).finish()
```

Sizing wrappers:

```rust
ConstrainedBox::new(child)
    .with_width(f32)
    .with_height(f32)
    .with_min_width(f32)
    .with_max_width(f32)
    .with_min_height(f32)
    .with_max_height(f32)
    .finish()
```

Practical limitation: `MainAxisSize::Max` and `Expanded`/`Shrinkable` need finite constraints on the flex axis. Debug builds log/assert if used under infinite constraints.

## 4. Scrollable Regions

For arbitrary element trees, app views commonly use `ClippedScrollable`:

```rust
let state = ClippedScrollStateHandle::default();
ClippedScrollable::vertical(
    state,
    child,
    ScrollbarWidth::Auto,
    nonactive_thumb_fill,
    active_thumb_fill,
    track_fill,
).finish()
```

Exact constructors:

```rust
ClippedScrollable::vertical(
    state: ClippedScrollStateHandle,
    child: Box<dyn Element>,
    scrollbar_size: ScrollbarWidth,
    nonactive_scrollbar_thumb_background: Fill,
    active_scrollbar_thumb_background: Fill,
    scrollbar_track_background: Fill,
) -> Scrollable

ClippedScrollable::horizontal(...) -> Scrollable
ClippedScrollable::vertical_centered(...) -> Scrollable
```

Manual scrollable API:

```rust
Scrollable::vertical(
    state: ScrollStateHandle,
    child: Box<dyn ScrollableElement>,
    scrollbar_size: ScrollbarWidth,
    nonactive_scrollbar_thumb_background: Fill,
    active_scrollbar_thumb_background: Fill,
    scrollbar_track_background: Fill,
) -> Self
```

`NewScrollable` supports one or both axes:

```rust
NewScrollable::vertical(config: SingleAxisConfig, nonactive: Fill, active: Fill, track: Fill)
NewScrollable::horizontal(config: SingleAxisConfig, nonactive: Fill, active: Fill, track: Fill)
NewScrollable::horizontal_and_vertical(config: DualAxisConfig, nonactive: Fill, active: Fill, track: Fill)
```

Limitations:

- `ClippedScrollable` is slower because it lays out the full child tree and clips it.
- `NewScrollableElement` requires custom elements to implement `axis`, `scroll_data`, and `scroll`; plain views should use `ClippedScrollable` unless performance requires manual scrolling.

## 5. Click Event Handling

Low-level event wrapper:

```rust
EventHandler::new(child)
    .on_left_mouse_down(|ctx, app, position| DispatchEventResult::StopPropagation)
    .on_left_mouse_up(|ctx, app, position| DispatchEventResult::StopPropagation)
    .on_keydown(|ctx, app, keystroke| DispatchEventResult::StopPropagation)
    .finish()
```

Exact callback forms from `EventHandler`:

```rust
FnMut(&mut EventContext, &AppContext, Vector2F) -> DispatchEventResult
FnMut(&mut EventContext, &AppContext, &Keystroke) -> DispatchEventResult
FnMut(&mut EventContext, &AppContext, &Vector2F, &ModifiersState) -> DispatchEventResult
FnMut(&mut EventContext, &AppContext, &KeyCode, &KeyState) -> DispatchEventResult
```

Higher-level `Hoverable` has click helpers:

```rust
Hoverable::new(mouse_state, |state| child)
    .on_click(|ctx, app, position| { ... })
    .on_mouse_down(|ctx, app, position| { ... })
    .on_double_click(|ctx, app, position| { ... })
    .finish()
```

Dispatch typed actions inside callbacks with:

```rust
ctx.dispatch_typed_action(MyAction::Clicked);
ctx.dispatch_typed_action(&MyAction::Clicked);
```

## 6. Keyboard Input And Key Bindings

Element-local keyboard input uses `EventHandler::on_keydown`:

```rust
EventHandler::new(child)
    .on_keydown(|ctx, _app, keystroke| {
        // inspect &Keystroke
        DispatchEventResult::StopPropagation
    })
    .finish()
```

View-level keybinding context comes from `View::keymap_context(&self, app: &AppContext) -> keymap::Context`. Default context contains `Self::ui_name()`.

Binding constructors in `keymap.rs` include:

```rust
FixedBinding::new(keystrokes: impl AsRef<str>, action: impl Action, context_predicate: ContextPredicate) -> Self
FixedBinding::new_per_platform(PerPlatformKeystroke { mac, linux_and_windows }, action, context_predicate) -> Self
```

Typed actions must satisfy `Action: Any + Debug + Send + Sync`; there is a blanket impl for matching types.

Limitation for this app: no local keymap registration code has been added in `crates/warp-opencode` yet. Downstream code must either add fixed/editable bindings at app setup if those APIs are exposed in the chosen integration point, or use `EventHandler::on_keydown` as a placeholder for direct key handling.

## 7. Modal And Overlay Rendering

Use `Stack` for overlays:

```rust
let mut stack = Stack::new();
stack.add_child(base);
stack.add_positioned_child(child, OffsetPositioning::offset_from_parent(...));
stack.add_overlay_child(overlay);
stack.add_positioned_overlay_child(overlay, positioning);
stack.finish()
```

Exact `Stack` positioning methods:

```rust
fn add_positioned_child(&mut self, child: Box<dyn Element>, positioning: OffsetPositioning)
fn add_overlay_child(&mut self, child: Box<dyn Element>)
fn add_positioned_overlay_child(&mut self, child: Box<dyn Element>, positioning: OffsetPositioning)
```

Practical modal pattern from app views:

- Render a scrim `Container` with semi-transparent `Fill`.
- Render modal content in a `Stack`.
- Center with `OffsetPositioning::offset_from_parent(vec2f(0., 0.), ParentOffsetBounds::WindowByPosition, ParentAnchor::Center, ChildAnchor::Center)`.
- Add close buttons as positioned children.

Limitations: `Stack` event dispatch differs by build mode default (`Waterfall` in debug, `Broadcast` otherwise). Set `with_event_dispatch_mode(...)` if behavior must be explicit.

## 8. AppStore-Style Subscriptions And Render Loop

Local app store (`crates/warp-opencode/src/state/mod.rs`) is external to WarpUI:

```rust
pub struct AppStore {
    model: Arc<RwLock<AppModel>>,
    changes: broadcast::Sender<()>,
}

pub fn subscribe(&self) -> broadcast::Receiver<()>;
pub async fn snapshot(&self) -> AppModel;
```

WarpUI invalidation API:

```rust
ViewContext::notify(&mut self)
ViewContext::spawner(&mut self) -> ViewSpawner<T>
ViewSpawner::spawn<R: Send + 'static>(&self, work: impl FnOnce(&mut T, &mut ViewContext<T>) -> R + Send + 'static) -> Result<R, ViewDropped>
```

Recommended integration pattern:

- Store a renderable snapshot on the view, not only `AppStore`.
- In `RootView::new`, get `let spawner = ctx.spawner()` and a `store.subscribe()` receiver.
- Run a background task that waits on `receiver.recv().await`.
- On each change, obtain `store.snapshot().await`, then call `spawner.spawn(move |view, ctx| { view.snapshot = snapshot; ctx.notify(); })`.

OpenWarp pattern: `RootView` stores `AppStore`, subscribes to changes with `ctx.spawner()`, and updates child snapshots from the UI thread. `PtyPanelView` uses the same pattern for live PTY `watch::Receiver<PtyState>` updates: a Tokio task waits for state changes, then calls the view spawner to replace the render snapshot and notify.

## 9. Minimal View Template

```rust
use warpui::color::ColorU;
use warpui::fonts::FamilyId;
use warpui::{
    elements::{Container, Element, Flex, ParentElement, Text},
    AppContext, Entity, View, ViewContext,
};
use warpui::SingletonEntity as _;

pub struct MyView {
    font_family: FamilyId,
}

impl MyView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let font_family = warpui::fonts::Cache::handle(ctx)
            .update(ctx, |cache, _| cache.load_system_font("Arial").unwrap());
        Self { font_family }
    }
}

impl Entity for MyView {
    type Event = ();
}

impl View for MyView {
    fn ui_name() -> &'static str { "MyView" }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Container::new(
            Flex::column()
                .with_spacing(8.)
                .with_child(Text::new("Hello", self.font_family, 14.).finish())
                .finish(),
        )
        .with_uniform_padding(16.)
        .with_background_color(ColorU::new(12, 14, 20, 255))
        .finish()
    }
}
```

## 10. Markdown Parser And Formatted Markdown Rendering

Parser exports from `crates/markdown_parser/src/lib.rs`:

```rust
parse_markdown(markdown: &str) -> anyhow::Result<FormattedText>
parse_markdown_with_gfm_tables(markdown: &str) -> anyhow::Result<FormattedText>
parse_markdown_to_raw_text(markdown: &str) -> anyhow::Result<String>
```

Core formatted text element constructors:

```rust
FormattedTextElement::new(
    formatted_text: FormattedText,
    font_size: f32,
    family_id: FamilyId,
    code_block_family_id: FamilyId,
    text_color: ColorU,
    highlight_index: HighlightedHyperlink,
) -> Self

FormattedTextElement::new_arc(...)
FormattedTextElement::from_str(text, family_id, font_size)
```

Useful builders:

```rust
.with_line_height_ratio(f32)
.with_heading_to_font_size_multipliers(HeadingFontSizeMultipliers { ... })
.with_color(ColorU)
.with_inline_code_properties(font_color, bg_color)
.with_hyperlink_font_color(ColorU)
.register_default_click_handlers(|HyperlinkUrl { url }, ctx, app| { ... })
.register_default_click_handlers_with_action_support(|HyperlinkLens, ctx, app| { ... })
.set_selectable(bool)
.disable_mouse_interaction()
.with_no_text_wrapping()
.with_clip(ClipConfig)
.finish()
```

Assistant-message rendering pattern:

```rust
FormattedTextElement::new(
    parse_markdown(message_text).unwrap_or_else(|_| FormattedText::new([FormattedTextLine::Line(vec![
        FormattedTextFragment::plain_text(message_text.to_owned()),
    ])])),
    13.,
    ui_font,
    code_font,
    text_color,
    highlighted_link.clone(),
)
.register_default_click_handlers_with_action_support(|link, evt, app| match link {
    HyperlinkLens::Url(url) => evt.open_url(url),
    HyperlinkLens::Action(action) => { /* downcast and dispatch if used */ }
})
.set_selectable(true)
.finish()
```

Limitations:

- `FormattedTextElement` is in `warpui_core`, but it depends on `markdown_parser`. Prior audit notes `markdown_parser` inherits the upstream workspace license, so confirm license posture before treating this as permissive-only.
- The local `crates/warp-opencode` workspace already declares `markdown_parser` in `Cargo.toml`, but assistant-message rendering has not been implemented.
- Images, embedded mappings, and GFM tables parse into `FormattedTextLine` variants, but downstream UI may need placeholders or custom handling depending on the desired assistant-message behavior.
- For parse failures, downstream code should fall back to plain `Text` or `FormattedTextElement::from_str` rather than panicking on streamed assistant content.

## 11. Testing And Headless Support

Local Warp checkout reviewed: `/home/mojo/projects/openwarp/Warp` at the paths below.

WarpUI has two distinct test-oriented layers:

- `warpui` exposes a headless platform backend via `warpui::platform::app::AppBuilder::new_headless(...)` (`Warp/crates/warpui/src/platform/app.rs`). It constructs the headless platform (`Warp/crates/warpui/src/platform/headless/*`) and reuses the test font DB, so it can run without native windows.
- The `test-util` Cargo feature (`warpui` feature `test-util`, forwarding to `warpui_core/test-util`) enables test-only helpers such as spawned-future tracking (`AppContext::await_spawned_future`) and delegate inspection (`get_cursor_shape`). Upstream enables this feature for its own dev-dependencies.
- Integration-style `TestDriver`/`Builder` APIs live under `warpui_core::integration` (`Warp/crates/warpui_core/src/integration/driver.rs`) and are intended to be handed to `AppBuilder::{new,new_headless}` so the driver runs after app initialization.

Practical pattern for future view tests:

```rust
let driver = warpui::integration::Builder::new(work_dir)
    .with_timeout(Duration::from_secs(5))
    .with_on_finish(|app, window_id, data| Box::pin(async move {
        // Inspect app/window state here if public APIs expose what the test needs.
    }))
    .build("test_name", true);

warpui::platform::app::AppBuilder::new_headless(callbacks, assets, Some(driver))
    .run(|ctx| {
        // create windows/views here
    })?;
```

Current OpenWarp limitation: `crates/warp-opencode` depends on `warpui`/`warpui_core` from the workspace without enabling `test-util`, and the app does not yet provide a small public harness that creates `RootView` in a headless `AppBuilder` with asset callbacks. The current public APIs are enough for state/API/SSE tests, but not for a stable, low-boilerplate `view_snapshot_tests.rs`. Those view snapshot tests were therefore skipped for Phase 4 Workstream 3 rather than adding brittle tests coupled to upstream private platform details.

## 12. Known Limitations

- IME: the audited Warp platform layer receives IME preedit/commit events, but the downstream `EventHandler::on_keydown` path currently used by OpenWarp does not expose stable composition callbacks such as `on_ime_compose` or `on_ime_commit`. OpenWarp therefore handles committed printable key text and clipboard paste, but does not render in-progress IME composition text yet.
- PTY mouse wheel: OpenWarp currently supports Page Up/Page Down scroll actions in the PTY panel. A direct mouse-wheel handler was not wired because the local `EventHandler` API documentation used by this crate only exposed click, hover, and key callbacks during this pass.
