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
- Added integration tests for session/message flow, permission event decoding, and streaming part deltas.

## Verification

Last verified locally:

```sh
cargo fmt --all
cargo test -p warp-opencode --tests
```

Both commands passed.

## License Status

OpenWarp is `AGPL-3.0-only`. The previous MIT-only boundary is intentionally abandoned for this experiment because upstream `warpui` and `warpui_core` directly depend on AGPL crates. See `docs/license-audit.md`.

## Next Work

- Wire the SSE stream into the running WarpUI application lifecycle.
- Replace placeholder view modules with interactive panels.
- Add a permissively licensed terminal renderer/ANSI parser for the PTY panel.
- Add reconnect/cancellation behavior for SSE and PTY tasks.
- Add UI/headless tests if WarpUI's test utilities are stable enough for this downstream crate.
