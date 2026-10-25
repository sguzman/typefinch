# egui-winit 0.35.0 — pinned JIS physical-key bridge

Upstream source: https://github.com/emilk/egui/tree/0.35.0/crates/egui-winit
Pinned upstream commit f72eaf6be1d137b2f568f7c21d4569ab6304b2b4.
License: MIT OR Apache-2.0 (original license texts included).

Only src/lib.rs is modified. Winit's physical IntlRo, IntlYen,
Convert, NonConvert, KanaMode, and Lang5 events now map into separate
egui::Key values. The corresponding egui Key enum is pinned in
third_party/egui-0.35.0. This is a narrow Typefinch T305 improvement
for Japanese physical positions, not permission to intercept global
keyboard input or attribute a compositor seat to individual hardware.

The isolated native X11 CI run 40240529267 demonstrated 254 passing
Rust tests, 4/4 real X11 physical-position scoring on JIS RO/YEN
Base/Shift, and non-scored Japanese conversion/mode controls.
Live physical Hyprland acceptance is still open.

Other Rust source files are unmodified upstream 0.35.0. Cargo.toml
expands upstream workspace-inherited versions for standalone Cargo
path patching. No font files are included.
