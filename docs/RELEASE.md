# 🚀 Linux development previews

Typefinch is **not yet a validated MVP**. Successful GitHub Actions builds
publish a versioned `x86_64` Linux preview archive and SHA-256 checksum as
CI artifacts. These are not signed releases, and X11 CI does not certify live
Hyprland input or physical keyboard routing.

GitHub Actions downloads are ZIP containers. Unzip the `typefinch-linux-preview`
artifact first to obtain the `.tar.gz` archive and matching `.sha256`
file. Place these together in the same directory and verify them with
`sha256sum -c typefinch-<version>-linux-x86_64.tar.gz.sha256`.

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

### Check a private backup before recovery

Use `typefinch --verify-backup SETTINGS.json SESSIONS.sqlite3` with the two
files generated together by F8. This checks that the settings contain persistent
profile IDs, that the SQLite snapshot passes an integrity check, that its schema
is supported and that stored sessions reference profiles included in the settings.
The command never opens the live profile or replaces either file. It rejects
corrupt settings and mismatched backup pairs. A successful validation does
**not** perform a restore; preserve the original files while planning recovery.

For a non-destructive recovery copy, run
`typefinch --restore-backup-to-new-dir SETTINGS.json SESSIONS.sqlite3 NEW_DIRECTORY`.
This first validates both source files, then creates a **new directory only**
and copies the pair to `settings.json` and `sessions.sqlite3` within it.
If the destination already exists, including if it is the live Typefinch
directory, the operation fails without overwriting anything. The copied
directory is not automatically activated. On any I/O error, a partial newly
created destination is left for inspection, rather than deleting user files.
Review it before choosing a new XDG configuration location for that copy.
Never merge or overwrite a live Typefinch directory while the app is running.

An existing unmanaged `bin/typefinch` is never overwritten. App settings
and the SQLite database are **not** rolled back with the binary: use the in-app
F8 full backup before experimenting with newer builds.

The tracked `Cargo.lock` pins the dependency resolver's crate versions and
checksums, and `rust-toolchain.toml` specifies Rust 1.99.1, the actual
toolchain verified by CI on 2026-10-24. Normal CI builds/tests and preview
packaging now use Cargo's `--locked` mode, and the workflow fails if a
command rewrites the committed lockfile. This controls source dependencies,
not the host C library, graphics drivers, linker or every build timestamp;
a bit-identical executable across different machines is **not yet proven**.

Maintainers can build with `cargo build --release --locked` and run
`lua5.4 tools/package_release.lua VERSION COMMIT_SHA` followed by
`bash tests/release_smoke.sh`. The explicit file allowlist excludes private
profiles, exports and arbitrary local files. It also includes the exact
`epaint 0.35.0` MIT and Apache-2.0 license files and upstream provenance
under `share/doc/typefinch/third-party/epaint-0.35.0/`, because Typefinch
currently includes a narrowly patched egui RTL shaping dependency.
`tests/release_smoke.sh` verifies these three files against the checked-in
third-party originals, alongside the existing checksum, install, upgrade,
rollback and unmanaged-file refusal tests. Installation also places these
notices under the user's `share/doc/typefinch/third-party/` prefix;
preflight refuses symlinked notice files, their parent directories,
and managed executable paths before writing a binary. Tar metadata and gzip headers
are normalized for repeatable archives *given identical binary contents*;
the Rust executable is not independently proven reproducible.

**Acceptance evidence:** [CI 40238040039](https://github.com/sguzman/typefinch/actions/runs/40238040039)
passed the original pinned-egui optimized build, deterministic archive
hash check and user-local install/upgrade/rollback. The subsequent
installer hardening additionally requires vendored upstream notices to be
installed byte-for-byte and refuses symlinked notice destinations before
writing the executable. These new safeguards require their own fresh
`[package]` run; prior artifacts do not retroactively establish them.

No GitHub Releases or tags are created by the CI preview job. Publication
and MVP acceptance remain separate release gates.

## Expensive preview-build gate

Fast Rust/X11 CI runs for every push to `main`. Optimized packaging is
intentionally opt-in: include `[package]` in a commit message or run the
workflow manually from GitHub Actions. This prevents the release compiler
from rebuilding the complete dependency graph on every development commit.
The archive is attached to that verified workflow run only if its packaging
and install/rollback tests pass.

## Identifying an installed preview

Newer binary builds support `typefinch --version` and `typefinch --help`
without opening a window. Compare `--version` to the archive's
`METADATA.txt`. The first package validated by CI precedes this CLI change;
the feature applies to subsequent builds, not retroactively to that archive.
