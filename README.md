# OpenWarp

OpenWarp is an experimental native frontend for OpenCode. The goal is to build `warp-opencode`, a standalone Rust binary that uses Warp's GPU-rendered `warpui`/`warpui_core` UI framework and connects to an already-running OpenCode server over HTTP, SSE, and WebSocket APIs.

OpenWarp is not a fork of Warp or OpenCode. It is a separate client that treats both projects as upstream dependencies and aims to provide a native Warp-style agentic terminal UI for OpenCode sessions.

## What This Is Trying To Do

- Use Warp's view/component model, layout system, rendering pipeline, and platform windowing as the native UI shell.
- Use OpenCode's existing backend server at `localhost:4096` for all agent/session logic.
- Stream OpenCode events incrementally through `/event` so assistant output, tool calls, permission prompts, questions, and PTY lifecycle updates render live.
- Provide a native UI for session navigation, chat threads, model/provider state, tool approvals, human-in-the-loop questions, status, and PTY panels.
- Avoid modifying either upstream repository.

## Current Status

This repository currently contains the research docs, workspace scaffold, typed Rust API/state/PTY layers, a minimal WarpUI root view, and integration tests for the API/state pieces.

Implemented:

- OpenCode HTTP client for health, sessions, messages, permissions, questions, providers, and PTY routes.
- SSE event decoding into typed `OpenCodeEvent` variants.
- PTY WebSocket transport helper.
- Reducer-style state store for sessions, message parts, streaming deltas, permissions, questions, providers, and PTYs.
- Minimal `warp-opencode` binary and WarpUI root view.
- Mock-server integration tests.

Still planned:

- Full interactive WarpUI panels for the session sidebar, chat thread, input bar, approval/question modals, PTY display, and status bar.
- SSE reconnect loop wired into the running UI.
- Terminal rendering and ANSI parsing from a permissively licensed implementation.
- Headless or UI-level view tests if the upstream WarpUI test APIs are practical for this crate.

## Repository Layout

```text
openwarp/
├── Cargo.toml
├── crates/warp-opencode/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── api/
│   │   ├── pty/
│   │   ├── state/
│   │   └── views/
│   └── tests/
└── docs/
    ├── warpui-audit.md
    ├── opencode-api-contract.md
    ├── ui-state-inventory.md
    └── scaffold-plan.md
```

The local `Warp/` checkout is intentionally ignored and not committed. Clone it locally when building until the dependency source is changed to a pinned git revision.

## Setup

Clone Warp next to this workspace root so the path dependencies resolve:

```sh
git clone https://github.com/warpdotdev/Warp.git Warp
```

Then build:

```sh
cargo build -p warp-opencode
```

Run tests:

```sh
cargo test -p warp-opencode --tests
```

## Run

Start OpenCode separately, then run:

```sh
cargo run -p warp-opencode -- --host localhost --port 4096
```

If the server uses `OPENCODE_SERVER_PASSWORD`:

```sh
cargo run -p warp-opencode -- --host localhost --port 4096 --username opencode --password "$OPENCODE_SERVER_PASSWORD"
```

## License Boundary

OpenWarp directly depends only on Warp's MIT-labeled `warpui` and `warpui_core` crates. Do not add Warp `app`, `warp_terminal`, `ui_components`, `markdown_parser`, `warp_util`, `sum_tree`, or other AGPL-labeled Warp crates as direct dependencies.

Current upstream note: the audited Warp revision has `warpui`/`warpui_core` marked MIT, but `cargo tree` shows transitive references to `markdown_parser`, `sum_tree`, and `warp_util`. Treat this as a release blocker until the upstream dependency/license boundary is clarified or those paths are removed/relicensed.

Before release, run:

```sh
cargo tree -p warp-opencode
cargo tree -p warp-opencode | grep -E 'warp_terminal|ui_components|markdown_parser|warp_util|sum_tree'
```

Review the exact resolved Warp revision with `cargo deny` or equivalent legal tooling.

## Platform Notes

macOS uses Warp's native Metal-backed path. Linux and Windows use Warp's WGPU/winit path and require normal GPU/windowing/font system libraries. The PTY panel talks to OpenCode's `/pty/:id/connect` WebSocket; it does not start local platform PTYs.
