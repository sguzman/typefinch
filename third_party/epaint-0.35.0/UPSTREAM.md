# epaint 0.35.0 — reproducible local upstream backport

This is a single third-party dependency of Typefinch, not an additional app
or general monorepo. It is pinned to the egui release tag `0.35.0`
(upstream Git commit `f72eaf6be1d137b2f568f7c21d4569ab6304b2b4`).

Source: https://github.com/emilk/egui/tree/0.35.0/crates/epaint

License: MIT OR Apache-2.0. Full original license texts are included in
`LICENSE-MIT` and `LICENSE-APACHE`. No fonts are bundled.

The sole Rust source modification is in `src/text/text_layout.rs`:
the RTL decreasing-cluster-offset correction in the upstream,
still-unmerged https://github.com/emilk/egui/pull/8698 for issue
https://github.com/emilk/egui/issues/8576. This fixes surplus shaped
glyph slots that push Arabic/Hebrew cursor indices past source characters.
The original patch was exercised against native X11 and nested Weston.

`Cargo.toml` expands upstream workspace-inherited dependency versions
for standalone Cargo path patching. Other Rust files match upstream tag
`0.35.0` exactly.

Remove the vendor override only after a released upstream epaint fixes
glyph count with all Typefinch Unicode, IME, X11, and Wayland
regressions green. Do not substitute an untested newer crate.
