# egui 0.35.0 — pinned third-party physical-key extension

Upstream source: https://github.com/emilk/egui/tree/0.35.0/crates/egui
Pinned release: 0.35.0, upstream commit f72eaf6be1d137b2f568f7c21d4569ab6304b2b4.
License: MIT OR Apache-2.0 (full original license texts included).

Only src/data/key.rs is changed, adding distinct physical JIS IntlRo,
IntlYen, Convert, NonConvert, KanaMode, and Lang5 Key variants and their
Key::ALL/name/from_name conversion entries. The paired winit bridge is
vendored separately as third_party/egui-winit-0.35.0.

This closes a dependency API omission, not a hardware permission gate.
Upstream winit already distinguishes the Linux keycode positions, but
egui 0.35 drops their physical identities. Typefinch must never infer
the physical position from an OS-translated character or XKB display.
See isolated CI 40240529267 (254 Rust tests + native X11 JIS 4/4)
for the experimental proof. Actual Hyprland hardware remains unverified.

Other Rust source files are unmodified upstream 0.35.0. Cargo.toml only
expands workspace dependency versions to support a standalone path patch.
Do not remove this override until a newer tested egui release provides
physical RO/YEN and control events. No fonts are copied.
