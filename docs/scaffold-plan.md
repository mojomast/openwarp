# OpenWarp Scaffold Plan

This plan defines OpenWarp: a standalone Rust workspace containing the `warp-opencode` binary. OpenWarp is AGPL-licensed because upstream `warpui` and `warpui_core` directly depend on AGPL Warp crates at the pinned revision. See `docs/license-audit.md`.

Important license note: the project has chosen to accept AGPL for the experiment rather than vendor and strip `markdown_parser`, `sum_tree`, and `warp_util` from WarpUI.

## 1. Cargo Workspace Structure

```text
openwarp/
├── Cargo.toml
├── crates/
│   └── warp-opencode/
│       ├── Cargo.toml
│       └── src/
│           ├── api/
│           ├── pty/
│           ├── state/
│           └── views/
└── docs/
    └── scaffold-plan.md
```

Root `Cargo.toml` owns dependency versions and pins `warpui` and `warpui_core` to `warpdotdev/warp` commit `c325d146ab314971e1577f168cf45f03118c3ac5`. `crates/warp-opencode/Cargo.toml` is the only application member for now.

The workspace now includes the initial implementation source files. The module structure below remains the intended organization for continued development.

## 2. Crate Dependency Graph

```text
warp-opencode
├── warpui                 # MIT-labeled Warp UI crate, pinned git dependency
├── warpui_core            # MIT-labeled Warp core crate, pinned git dependency
├── markdown_parser        # AGPL direct dependency of upstream WarpUI crates
├── sum_tree               # AGPL direct dependency of upstream WarpUI crates
├── warp_util              # AGPL direct dependency of upstream WarpUI crates
├── reqwest                # HTTP JSON API client, rustls TLS
├── eventsource-client     # /event and /global/event SSE streams
├── tokio                  # background runtime for network and stream tasks
├── tokio-tungstenite      # /pty/:ptyID/connect WebSocket transport
├── futures-util           # Stream/Sink utilities for SSE and WebSocket tasks
├── serde, serde_json      # opencode API schemas and event payloads
├── bytes                  # PTY/WebSocket byte buffers
├── url, base64            # endpoint construction and Basic/auth_token support
├── anyhow, thiserror      # application and API error boundaries
└── tracing                # structured diagnostics
```

Do not add these additional Warp crates as dependencies unless the AGPL impact is deliberate:

```text
Warp/app
Warp/crates/warp_terminal
Warp/crates/ui_components
```

Terminal rendering should still be implemented independently. `warp_terminal` is not currently part of the dependency graph and should remain reference-only unless a future decision explicitly accepts that additional dependency.

## 3. Planned Module Structure

```text
src/main.rs
src/api/
  mod.rs
  client.rs
  events.rs
  schema.rs
  session.rs
  provider.rs
  pty.rs
  permission.rs
  question.rs
src/sse_loop.rs
src/views/
  mod.rs
  draft_buffer.rs
  root.rs
  session_list.rs
  chat_thread.rs
  input_bar.rs
  tool_approval.rs
  question_prompt.rs
  pty_panel.rs
  status_bar.rs
src/state/
  mod.rs
src/pty/
  mod.rs
  client.rs
  colors.rs
  buffer.rs
  pty_state.rs
  terminal.rs
  terminal_model.rs
```

`main.rs` creates `warpui::platform::AppBuilder`, starts the SSE loop, adds the root window, and wires the initial `AppState` model. `api/` owns HTTP, SSE, and schema types derived from `docs/opencode-api-contract.md`. `views/` owns WarpUI `View` implementations, rendering composition, and the `DraftBuffer` input model. `state/` owns canonical app stores, selected IDs, optimistic message state, and event reducers. `pty/` owns the opencode PTY WebSocket client, VTE terminal grid, xterm color mapping, replay cursor, local buffer, and terminal model adapter.

## 4. State Management Approach

WarpUI is entity/handle based, not React-like. The app should use a small set of long-lived models stored in `AppContext` and update them from background network tasks through WarpUI main-thread callbacks.

Recommended shape:

```text
AppState
├── connection: server URL, auth mode, health, selected directory
├── sessions: map SessionID -> Session plus ordered sidebar rows
├── messages: map SessionID -> ordered MessageWithParts and part indexes
├── status: map SessionID -> idle/busy/retry
├── approvals: pending PermissionRequest and QuestionRequest by session
├── providers: provider/model catalog and selected model/agent
└── terminals: PtyInfo, active terminal ID, cursor, buffer metadata
```

Initial bootstrap should call `GET /global/health`, `GET /path`, `GET /session`, `GET /session/status`, `GET /permission`, `GET /question`, and `GET /provider`. The instance SSE stream (`GET /event`) is the live synchronization source for session, message, part, status, permission, question, and PTY lifecycle events.

Event handling should use reducer-style methods on state models:

```text
ApiEvent -> decode -> AppEvent -> AppState::apply_event -> ViewContext::notify
```

For WarpUI compatibility:

- Use `ViewContext::spawn` or a captured `spawner` to run network tasks off the UI thread and apply results on the main thread.
- Mutate models only inside `AppContext`/`ModelHandle::update`/`ViewHandle::update` callbacks.
- Call `ViewContext::notify` for explicit invalidation after store mutations.
- Use `Tracked<T>` sparingly for small local UI state; avoid it for large message or terminal buffers because any mutable access invalidates dependent views.
- Keep SSE/WebSocket tasks cancellation-aware and tied to the active server/session/PTY identity so stale events do not update the current view.
- Treat `message.part.delta` as an incremental patch into the local part buffer, then let later `message.part.updated` snapshots replace canonical data.

## 5. Build Instructions And Cross-Platform Notes

Build from the workspace root. No local Warp checkout is required:

```sh
cargo build -p warp-opencode
cargo run -p warp-opencode -- --host localhost --port 4096
```

Useful verification commands:

```sh
cargo tree -p warp-opencode
cargo tree -p warp-opencode | grep -E 'markdown_parser|sum_tree|warp_util'
```

The second command documents the known AGPL Warp crates accepted by `docs/license-audit.md`.

Linux notes:

- WarpUI uses the winit/WGPU path and system font discovery. Install common graphics/font dependencies for the target distro.
- Wayland/X11 behavior depends on the local WarpUI backend dependencies in the checked-out Warp revision.

macOS notes:

- WarpUI uses native Metal/text dependencies from the Warp checkout.
- Do not enable optional debug/frame-capture features in release builds unless explicitly needed.

Windows notes:

- WarpUI uses the Windows backend dependencies from the Warp checkout.
- PTY transport to opencode remains WebSocket-based, so no Windows ConPTY binding is needed for the standalone client unless local terminal execution is added later.

Network/auth notes:

- HTTP uses `reqwest` with `rustls-tls` to avoid platform OpenSSL setup.
- If `OPENCODE_SERVER_PASSWORD` is enabled, send Basic auth headers for HTTP/SSE. For WebSocket environments where headers are not available, add `auth_token=<base64(username:password)>` to the query string as documented by opencode.
- SSE `/event` is the primary live source. `POST /session/:sessionID/message` returns a final JSON object, not token chunks.

PTY notes:

- Create terminals with `POST /pty/` and connect to `/pty/:ptyID/connect?cursor=<cursor>` using `tokio-tungstenite`.
- Text WebSocket frames are raw terminal output/input. Binary control frames starting with `0x00` contain cursor JSON.
- Keep terminal transport, VTE parsing, and grid state separate from WarpUI. The current PTY panel renders the grid through WarpUI primitives; live WebSocket feeding is the next PTY integration step.
