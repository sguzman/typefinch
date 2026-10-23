# 🚀 Linux development previews

Typefinch is **not yet a validated MVP**. Successful GitHub Actions builds
publish a versioned `x86_64` Linux preview archive and SHA-256 checksum as
CI artifacts. These are not signed releases, and X11 CI does not certify live
Hyprland input or physical keyboard routing.

From an extracted `typefinch-<version>-linux-x86_64` folder, use
`./install.sh` to install under `$HOME/.local`, or
`./install.sh --prefix "$HOME/.local"` to specify the destination.

Verify the downloadable archive with its adjacent `.sha256` using
`sha256sum -c` before extraction. The installer requires no sudo and
does not change system keyboard layouts, daemons, or local profile storage.
A newer extracted archive can replace the managed executable. Run
`./install.sh --rollback` (with the same prefix) to restore the immediately
previous installed executable and version marker. Only one prior binary is
retained; no rollback is available before the first upgrade.

An existing unmanaged `bin/typefinch` is never overwritten. App settings
and the SQLite database are **not** rolled back with the binary: use the in-app
F8 full backup before experimenting with newer builds.

Maintainers can build with `cargo build --release --locked` and run
`lua5.4 tools/package_release.lua VERSION COMMIT_SHA` followed by
`bash tests/release_smoke.sh`. The explicit file allowlist excludes private
profiles, exports and arbitrary local files. Tar metadata and gzip headers
are normalized for repeatable archives *given identical binary contents*;
the Rust executable is not independently proven reproducible.

No GitHub Releases or tags are created by the CI preview job. Publication
and MVP acceptance remain separate release gates.

## Expensive preview-build gate

Fast Rust/X11 CI runs for every push to `main`. Optimized packaging is
intentionally opt-in: include `[package]` in a commit message or run the
workflow manually from GitHub Actions. This prevents the release compiler
from rebuilding the complete dependency graph on every development commit.
The archive is attached to that verified workflow run only if its packaging
and install/rollback tests pass.
