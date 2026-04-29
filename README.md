# OpenWarp

OpenWarp is an AGPL-licensed experimental native frontend for OpenCode. The goal is to build `warp-opencode`, a standalone Rust binary that uses Warp's GPU-rendered `warpui`/`warpui_core` UI framework and connects to an already-running OpenCode server over HTTP, SSE, and WebSocket APIs.

OpenWarp is not a fork of Warp or OpenCode. It is a separate client that treats both projects as upstream dependencies and aims to provide a native Warp-style agentic terminal UI for OpenCode sessions.

## What This Is Trying To Do

- Use Warp's view/component model, layout system, rendering pipeline, and platform windowing as the native UI shell.
- Use OpenCode's existing backend server at `localhost:4096` for all agent/session logic.
- Stream OpenCode events incrementally through `/event` so assistant output, tool calls, permission prompts, questions, and PTY lifecycle updates render live.
- Provide a native UI for session navigation, chat threads, model/provider state, tool approvals, human-in-the-loop questions, status, and PTY panels.
- Avoid modifying either upstream repository.

## Current Status

This repository currently contains the research docs, workspace scaffold, typed Rust API/state/PTY layers, Phase 3 WarpUI panels, SSE lifecycle wiring, and integration tests for the API/state/SSE pieces.

Implemented:

- OpenCode HTTP client for health, sessions, messages, permissions, questions, providers, and PTY routes.
- SSE event decoding into typed `OpenCodeEvent` variants.
- PTY WebSocket transport helper.
- Reducer-style state store for sessions, message parts, streaming deltas, permissions, questions, providers, and PTYs.
- WarpUI session list, chat thread, input bar, tool approval overlay, question overlay, status bar, and root layout wiring.
- Reconnecting SSE loop wired into `main.rs`.
- VTE-based PTY terminal grid primitives and snapshot panel rendering.
- Rope-backed cursor-aware input draft buffer.
- Mock-server integration tests.

Still planned:

- Live PTY WebSocket feeding into the rendered terminal grid.
- Clipboard paste, IME, and deeper focus/keybinding polish for the input bar.
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
    ├── license-audit.md
    ├── warpui-audit.md
    ├── opencode-api-contract.md
    ├── ui-state-inventory.md
    └── scaffold-plan.md
```

## Setup

Warp dependencies are pinned to `warpdotdev/warp` commit `c325d146ab314971e1577f168cf45f03118c3ac5`; no local Warp checkout is required.

Build:

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

## License

OpenWarp is licensed as `AGPL-3.0-only`.

Reason: `warpui` and `warpui_core` are MIT-labeled crates, but at the pinned Warp revision they directly depend on `markdown_parser`, `sum_tree`, and `warp_util`, which inherit Warp's workspace `AGPL-3.0-only` license. OpenWarp accepts AGPL for this experiment rather than maintaining a fork with those dependencies stripped or replaced.

See `docs/license-audit.md` for the full finding and decision.

Useful audit command:

```sh
cargo tree -p warp-opencode
cargo tree -p warp-opencode | grep -E 'markdown_parser|sum_tree|warp_util'
```

## Platform Notes

macOS uses Warp's native Metal-backed path. Linux and Windows use Warp's WGPU/winit path and require normal GPU/windowing/font system libraries. The PTY panel talks to OpenCode's `/pty/:id/connect` WebSocket; it does not start local platform PTYs.
