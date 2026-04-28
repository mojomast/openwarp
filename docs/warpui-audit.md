# WarpUI Audit Spec

Source reviewed: `warpdotdev/Warp` at the locally cloned `Warp/` checkout. Primary focus was `crates/warpui` and `crates/warpui_core`; terminal/PTTY references were inspected only where needed in `app/` and `crates/warp_terminal`.

## License Boundaries

- MIT crates: `crates/warpui` and `crates/warpui_core` explicitly declare `license = "MIT"`.
- AGPL workspace default: the root workspace declares `license = "AGPL-3.0-only"`; crates that use `license.workspace = true` are AGPL unless overridden.
- AGPL examples/references: `app`, `crates/warp_terminal`, `crates/markdown_parser`, `crates/ui_components`, `crates/warp_util`, and `crates/sum_tree` inherit the AGPL workspace license.
- Do not recommend importing AGPL crates for a permissive implementation. Use AGPL code only as behavioral reference.
- Important consequence: `FormattedTextElement` in MIT `warpui_core` depends on AGPL `markdown_parser` by crate metadata. Treat Markdown support as not cleanly MIT-only unless `markdown_parser` is relicensed or replaced.

## 1. App Entrypoint And Lifecycle

- Public entrypoint is `warpui::platform::AppBuilder` from MIT `crates/warpui/src/platform/app.rs`.
- Typical startup from examples: construct `AppBuilder::new(AppCallbacks::default(), Box<dyn AssetProvider>, None)` and call `run(|ctx| { ctx.add_window(AddWindowOptions::default(), RootView::new); })`.
- `AppBuilder::run` wraps initialization, applies optional custom key trigger conversion, and delegates to the current platform backend or headless backend.
- On Linux/Windows/wasm, `crates/warpui/src/windowing/winit/app.rs` creates a `winit` user-event loop, initializes the WGPU instance, constructs the `warpui_core::App`, then drives `EventLoop::handle_event`.
- Init happens on `Event::NewEvents(StartCause::Init)`: `AppCallbackDispatcher::initialize_app` calls the user init closure through `App::update`, then validates bindings.
- Tick/frame behavior is event-driven, not a fixed explicit `tick()` API. Winit `UserEvent`s, window events, timers, redraw requests, and invalidations drive updates. `ControlFlow::Wait` is used by default.
- Rendering happens when platform windows receive `RedrawRequested`; invalidations are collected by `AppContext::update_windows` and presenter/window callbacks request redraws.
- Delayed repaint uses foreground timers (`manage_delayed_repaint_timers`) that set `redraw_requested` and call `update_windows`.
- Shutdown uses `CustomEvent::Terminate`, `CloseWindow`, `WindowEvent::CloseRequested`, and `AppCallbacks` hooks: `on_should_terminate_app`, `on_should_close_window`, `on_will_terminate`, and `on_window_will_close`.
- Forced/content-transfer terminations bypass cancellable approval. On non-macOS, destroyed last window exits the event loop.

## 2. View Trait Interface, Required Methods, Composition

- `View` is in MIT `warpui_core::core::view` and extends `Entity`.
- Required methods: `fn ui_name() -> &'static str` and `fn render(&self, app: &AppContext) -> Box<dyn Element>`.
- `Entity` supplies an associated `type Event`; many examples use `type Event = ()`.
- Optional lifecycle/hooks include `on_focus`, `on_blur`, `on_window_closed`, `on_window_transferred`, `active_cursor_position`, `keymap_context`, `self_or_child_interacted_with`, `accessibility_contents`, and `accessibility_data`.
- Composition is handle-based. `ViewContext::add_view` and `add_typed_action_view` create child views; `elements::ChildView::new(&handle)` embeds a child view in an element tree.
- `ViewContext` provides view-local APIs: `handle`, `window_id`, `view_id`, `focus`, `focus_self`, `emit`, `notify`, `observe`, `subscribe_to_model`, `subscribe_to_view`, `spawn`, and `spawner`.
- `TypedActionView` adds typed action handling through `type Action: Action` and `handle_action`.

## 3. Layout System

- Layout operates through MIT `Element` trait: `layout(SizeConstraint, LayoutContext, AppContext) -> Vector2F`, `after_layout`, `paint`, `size`, `origin`, and `dispatch_event`.
- `SizeConstraint` is min/max vector based. Elements receive constraints during layout and report concrete sizes.
- `Flex` is Flutter-inspired. `Flex::row()` and `Flex::column()` support `with_main_axis_size`, `with_main_axis_alignment`, `with_cross_axis_alignment`, `with_spacing`, reverse orientation, and child extension.
- `MainAxisSize::Min` minimizes along the main axis; `MainAxisSize::Max` expands to the incoming max. Debug assertions warn if `Max` or flexible children are used under infinite main-axis constraints.
- Flexible children use wrappers: `Expanded::new(flex, child)` uses tight fit; `Shrinkable::new(flex, child)` uses loose fit.
- `CrossAxisAlignment` supports `Start`, `Center`, `End`, and `Stretch`.
- `Container` provides margin, padding, overdraw, background fill/gradient, borders, corner radius, drop shadow, and foreground overlay.
- `ConstrainedBox` clamps inherited constraints with explicit min/max/width/height.
- Scroll containers exist in two APIs. Legacy `Scrollable` wraps a `ScrollableElement` for one axis. `NewScrollable` wraps `NewScrollableElement` and supports horizontal, vertical, or both axes with independent scrollbar appearance/config.
- Scroll units are pixels internally; non-precise wheel line deltas are converted with a `40px` per line constant.
- `Clipped`, `ClippedScrollable`, `Align`, `Stack`, `SavePosition`, `Percentage`, `Resizable`, `UniformList`, `ViewportedList`, and `Table` are additional MIT layout/composition tools.

## 4. Text Rendering

- MIT `Text` element renders styled text using `fonts::Cache`, `text_layout::LayoutCache`, and platform text layout systems.
- `Text::new` soft-wraps by default; `Text::new_inline` disables wrapping and is documented as deprecated-like in favor of `Text::new(...).soft_wrap(false)`.
- Fonts are addressed by `FamilyId`, `FontId`, `Properties { style, weight }`, `Style`, and `Weight`. `fonts::Cache` loads system fonts and tracks glyph metrics, advances, bounds, rasterization, and fallback.
- `TextStyle` supports foreground color, syntax color, underline, strikethrough, and related style runs. Foreground color overrides syntax color.
- `Highlight` and `HighlightedRange` apply per-character style spans. Ranges should be sorted; helpers merge overlapping/contiguous ranges.
- `FormattedTextElement` supports formatted lines, headings, inline/code-block styling, hyperlinks, alignment, selection, and click/hover handlers. It is in MIT `warpui_core` source but depends on AGPL `markdown_parser`; do not treat it as MIT-only usable without replacing that dependency.
- ANSI terminal parsing is not in MIT `warpui_core`. `crates/warp_terminal` has AGPL terminal model/ANSI/vte code and should be reference-only for permissive implementations.
- Syntax highlighting appears as style support (`TextStyle::syntax_color`) rather than a MIT parser/highlighter pipeline. A new MIT-only implementation should bring its own parser/highlighter and translate ranges into `Text` styles.

## 5. Event Handling, Keyboard, Mouse, Focus

- Platform events are converted to `warpui_core::Event` in MIT `warpui` winit event loop.
- Mouse conversion covers cursor move, left drag, left/right/middle down, left up, wheel, touch-to-mouse, long press/right click, drag/drop files, and momentum scroll.
- Keyboard conversion maps winit keyboard input into WarpUI key events, modifier changes, typed characters, and IME preedit/commit (`SetMarkedText`, `ClearMarkedText`, `TypedCharacters`).
- Events dispatch through window callbacks into `AppContext::handle_window_event`.
- Keybindings are checked first for `Event::KeyDown` against the focused view responder chain. If unhandled, normal element dispatch runs.
- Element event dispatch starts at the root element. Parents are expected to forward to children; elements return whether propagation should stop.
- Focus is view-level. `AppContext` tracks focused view per window; `ViewContext::focus` and `focus_self` enqueue focus effects.
- `View::on_focus` and `View::on_blur` receive `FocusContext`/`BlurContext` indicating self vs descendant transitions.
- `keymap_context` contributes context sets/maps used to determine valid shortcuts. The terminal reference uses this heavily for modal/keybinding state.

## 6. State Management

- Core state is manual, entity/handle based: `AppContext` owns models and views; `ModelHandle`/`ViewHandle` provide typed access.
- Updates happen inside `App::update`, `AppContext` methods, `ModelHandle::update`, and `ViewHandle::update`; effects are queued and flushed after nested updates complete.
- Manual invalidation uses `ViewContext::notify`, which queues `Effect::ViewNotification`; model changes can queue `Effect::ModelNotification`.
- Events are explicit. `ViewContext::emit` emits a typed `Entity::Event`; subscribers must subscribe per instance.
- Observations are invalidation notifications without event payload; subscriptions receive emitted payloads.
- Automatic tracking exists through MIT `Tracked<T>`. Reads during `View::render` are recorded in a thread-local dependency cache; mutable deref marks dependent views invalid.
- `Tracked` is main-thread only, not `Send`/`Sync`, and invalidates on mutable access rather than true diffing. Interior mutability changes are not detected.
- There is no React-like signal graph outside `Tracked`; the rest is explicit handles, updates, notifications, and presenter invalidations.

## 7. Async Tasks Integration

- MIT async support lives in `warpui_core::async`.
- Native foreground executor schedules local futures onto the platform main thread via a `DispatchDelegate`; test foreground uses `async_executor::LocalExecutor`.
- Native background executor is a multi-thread Tokio runtime, one thread in tests/integration, otherwise `num_cpus::get()`.
- `ViewContext::spawn` runs a `Send` background future and invokes an `on_resolve` callback on the main thread with `&mut View` and `&mut ViewContext`.
- `ViewContext::spawn_abortable` adds an abort handle and abort callback.
- `ViewContext::spawn_stream_local` polls a local stream on the main thread and calls per-item/done callbacks with mutable view context.
- `ViewContext::spawner` gives background code a handle to enqueue closures back onto the main thread for a specific view without keeping that view alive.
- Platform event loops also use custom user events (`RunTask`, `UpdateUIApp`, timers, notification callbacks) to bridge background work to the UI thread.

## 8. PTY/Terminal Surface Embedding In WarpUI View

- No MIT-only terminal surface abstraction was found in `warpui` or `warpui_core`.
- AGPL reference implementation is `app/src/terminal/view.rs` plus `app/src/terminal/model/*` and `crates/warp_terminal`.
- The reference `TerminalView` is a normal `View` whose `render` builds a large WarpUI element tree with `Flex`, `Stack`, `ChildView`, `TerminalSizeElement`, scrollables, block-list elements, and `AltScreenElement`.
- PTY integration is outside WarpUI core. `TerminalView::write_to_pty`, `shutdown_pty`, `PtyController`, and terminal manager utilities wire model/view/controller communication in AGPL app code.
- Terminal rendering has two modes in the reference: block-list rendering for Warp blocks and alt-screen rendering via `AltScreenElement`. Alt screen may be wrapped in vertical/horizontal scrollable containers for shared-session viewers.
- Size synchronization is done by wrapping the terminal element in a measuring element (`TerminalSizeElement`) that sends size updates; model/session size updates then resize terminal dimensions.
- For a permissive implementation, build a separate MIT-compatible terminal model/PTTY layer and expose it as a custom `Element` or a `View` that composes MIT WarpUI elements. Do not import `app` or `warp_terminal` code.

## 9. Minimal MIT-Only Hello World Skeleton

This is a specification skeleton, not implementation code to add to this workspace.

- Dependencies should be limited to MIT `warpui`/`warpui_core` plus third-party crates with compatible licenses for assets/errors.
- Define an `AssetProvider`; examples use `rust_embed`, but an empty provider can be used if no assets are loaded.
- In `main`, create `warpui::platform::AppBuilder::new(AppCallbacks::default(), Box::new(assets), None)`.
- Call `run(|ctx| { ctx.add_window(AddWindowOptions::default(), RootView::new); })`.
- `RootView::new(ctx: &mut ViewContext<Self>)` should load a system font through `warpui::fonts::Cache::handle(ctx).update(ctx, |cache, _| cache.load_system_font("Arial"))` or use an already known family if available.
- Implement `Entity for RootView { type Event = (); }`.
- Implement `View for RootView` with `ui_name` and `render`, returning a simple `Text` wrapped in `Align`/`Container`/`Flex` as needed.
- Avoid `FormattedTextElement` unless its AGPL `markdown_parser` dependency is replaced or license posture is acceptable.

## 10. Gotchas And Undocumented APIs From Tests/Examples

- `Text::new` now soft-wraps by default; examples often use `Text::new_inline` for single-line labels.
- `Flex::MainAxisSize::Max` and flexible children require finite constraints along the main axis; debug builds include call-site diagnostics for misuse.
- `Tracked<T>` is convenient but not a precise diff system. Mutable access alone invalidates; interior mutability is invisible.
- `ViewContext::notify` only dirties that specific view instance. Dirty children must notify independently unless invalidated via autotracking/model observation.
- View events do not bubble. Subscribers must explicitly subscribe to a specific view/model handle.
- `FormattedTextElement` has a larger default line height (`1.4`) than `Text` (`1.2`), and comments note layout mismatches in constrained rows.
- `FormattedTextElement::from_str` disables mouse interaction by default.
- Scrollbar/wheel APIs use pixels internally, but non-precise wheel events are line-based and converted with a hard-coded 40px multiplier.
- Winit focus events are coalesced into a custom `ActiveWindowChanged` event on the next event-loop tick because raw focus events can fire multiple times between Warp windows.
- On mobile wasm, touch events are converted to mouse/scroll gestures and soft keyboard display is deferred until tap completion to avoid showing during scroll/drag.
- Tests use headless/test delegates and `App::test`; these APIs are useful for behavior exploration but may be behind `test-util` features.
- Examples are in the MIT `warpui` crate, but check their dependencies before copying patterns into a license-sensitive product.
