# License Audit

This audit records the resolved license decision for OpenWarp's use of Warp UI crates.

Source inspected: `warpdotdev/warp` at commit `c325d146ab314971e1577f168cf45f03118c3ac5`.

## Confirmed Facts

- `crates/warpui/Cargo.toml` declares `license = "MIT"`.
- `crates/warpui/Cargo.toml` directly depends on `markdown_parser.workspace = true` and `sum_tree.workspace = true`.
- `crates/warpui_core/Cargo.toml` declares `license = "MIT"`.
- `crates/warpui_core/Cargo.toml` directly depends on `markdown_parser.workspace = true`, `sum_tree.workspace = true`, and `warp_util.workspace = true`.
- `crates/markdown_parser/Cargo.toml` uses `license.workspace = true`.
- `crates/sum_tree/Cargo.toml` uses `license.workspace = true`.
- `crates/warp_util/Cargo.toml` uses `license.workspace = true`.
- Warp's root `[workspace.package]` declares `license = "AGPL-3.0-only"`.

Therefore `markdown_parser`, `sum_tree`, and `warp_util` resolve to `AGPL-3.0-only`.

## Crates In The OpenWarp WarpUI Dependency Boundary

Explicit MIT crates:

- `warpui`
- `warpui_core`

AGPL crates directly required by those MIT-labeled crates:

- `markdown_parser`
- `sum_tree`
- `warp_util`

## Conclusion

OpenWarp cannot link upstream `warpui` and `warpui_core` as-is while remaining MIT-only. The contamination is direct, not merely incidental: `warpui` and `warpui_core` explicitly reference AGPL workspace crates as non-optional dependencies.

## Decision

OpenWarp accepts AGPL for the experiment.

This is the pragmatic path because OpenWarp is a public open-source experiment whose main goal is to quickly validate a native Warp-style frontend for OpenCode. Vendoring and replacing the AGPL dependencies would create a high-maintenance fork before the product direction is validated.

Implemented mitigation:

- OpenWarp workspace license changed to `AGPL-3.0-only`.
- `LICENSE` added with the AGPL-3.0 text.
- `README.md` updated to disclose the license decision and the direct AGPL dependency cause.
- Warp dependencies migrated to pinned git dependencies at `c325d146ab314971e1577f168cf45f03118c3ac5`.

Rejected alternatives for now:

- Vendor and strip AGPL dependencies from `warpui`/`warpui_core`.
- Replace WarpUI with a different GUI toolkit.
- Build a wrapper that avoids linking `warpui`/`warpui_core`.

These alternatives can be revisited if OpenWarp needs permissive licensing later.
