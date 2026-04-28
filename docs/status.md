# OpenWarp Status

OpenWarp is the standalone workspace for the `warp-opencode` binary: a native Rust frontend for OpenCode using Warp's GPU-rendered UI crates.

## Completed

- Researched WarpUI architecture and captured the findings in `docs/warpui-audit.md`.
- Extracted the OpenCode Hono API contract in `docs/opencode-api-contract.md`.
- Inventoried OpenCode's SolidJS UI states in `docs/ui-state-inventory.md`.
- Planned the Rust workspace in `docs/scaffold-plan.md`.
- Added the `warp-opencode` crate with API, PTY, state, and view modules.
- Added integration tests for session/message flow, permission event decoding, and streaming part deltas.

## Verification

Last verified locally:

```sh
cargo fmt --all
cargo test -p warp-opencode --tests
```

Both commands passed.

## Known Release Blocker

The crate directly depends only on `warpui` and `warpui_core`, but the current audited Warp revision pulls `markdown_parser`, `sum_tree`, and `warp_util` transitively through those crates. This must be resolved or explicitly approved before release if the project requires a strict MIT-only Warp dependency boundary.

## Next Work

- Wire the SSE stream into the running WarpUI application lifecycle.
- Replace placeholder view modules with interactive panels.
- Add a permissively licensed terminal renderer/ANSI parser for the PTY panel.
- Add reconnect/cancellation behavior for SSE and PTY tasks.
- Add UI/headless tests if WarpUI's test utilities are stable enough for this downstream crate.
