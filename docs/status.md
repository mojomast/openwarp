# OpenWarp Status

OpenWarp is the standalone workspace for the `warp-opencode` binary: a native Rust frontend for OpenCode using Warp's GPU-rendered UI crates.

## Completed

- Researched WarpUI architecture and captured the findings in `docs/warpui-audit.md`.
- Extracted the OpenCode Hono API contract in `docs/opencode-api-contract.md`.
- Inventoried OpenCode's SolidJS UI states in `docs/ui-state-inventory.md`.
- Planned the Rust workspace in `docs/scaffold-plan.md`.
- Resolved the WarpUI license blocker in `docs/license-audit.md` by accepting AGPL for OpenWarp.
- Migrated WarpUI dependencies from local path dependencies to pinned git dependencies.
- Added the `warp-opencode` crate with API, PTY, state, and view modules.
- Added `docs/warpui-patterns.md` to document practical WarpUI view patterns.
- Implemented Phase 3 panels: session list, chat thread, input bar, tool approval overlay, question overlay, PTY shell, and status bar.
- Wired the root layout and store subscription flow so snapshots refresh child views.
- Added a reconnecting SSE lifecycle loop and wired it into `main.rs`.
- Added integration tests for session/message flow, permission event decoding, and streaming part deltas.
- Implemented Phase 4 PTY terminal grid primitives using `vte`, including ANSI parsing, SGR styles, cursor movement, screen clearing, scrollback, resize preservation, and xterm 256-color mapping.
- Updated the PTY panel to render the VTE-backed grid snapshot. Live WebSocket feeding into the panel remains a follow-up integration item.
- Replaced the input bar's append-only string with a `ropey`-backed cursor-aware `DraftBuffer`, including multiline movement, word deletion, line deletion, and caret display fallback.
- Added state behavior, PTY grid, draft buffer, and session round-trip tests.

## Verification

Last verified locally:

```sh
cargo fmt --all
cargo test -p warp-opencode --all
cargo clippy -p warp-opencode -- -D warnings
cargo build --release -p warp-opencode
```

All commands passed locally.

## License Status

OpenWarp is `AGPL-3.0-only`. The previous MIT-only boundary is intentionally abandoned for this experiment because upstream `warpui` and `warpui_core` directly depend on AGPL crates. See `docs/license-audit.md`.

## Next Work

- Wire the SSE stream into the running WarpUI application lifecycle.
- Add terminal renderer/ANSI parser for the PTY panel.
- Connect live PTY WebSocket output/input to the VTE-backed PTY panel.
- Add clipboard paste, IME, richer focus, and true caret rendering to the input bar.
- Add UI/headless tests if WarpUI's test utilities are stable enough for this downstream crate.
