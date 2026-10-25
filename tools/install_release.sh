#!/bin/sh
# User-local Typefinch preview installer: never sudo or alter OS input.
set -eu
prefix="${HOME}/.local"; rollback=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix) [ "$#" -ge 2 ] || exit 2; prefix="$2"; shift 2 ;;
    --rollback) rollback=1; shift ;;
    -h|--help) echo "Usage: $0 [--prefix DIR] [--rollback]"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done
[ -n "$prefix" ] && [ "$prefix" != "/" ] || { echo "Unsafe prefix" >&2; exit 2; }
case "$prefix" in *"/../"*|*"//"*) echo "Ambiguous prefix" >&2; exit 2 ;; esac
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
bin="$prefix/bin"; data="$prefix/share/typefinch"
exe="$bin/typefinch"; previous="$data/previous-binary"
marker="$data/installed.version"; prior="$data/previous.version"
vendor_target="$prefix/share/doc/typefinch/third-party/epaint-0.35.0"
# Check every managed path component BEFORE creating directories or replacing
# binaries. Refuse redirection through a pre-existing symlinked ancestor.
for managed_path in "$bin" "$prefix/share" "$data" \
    "$prefix/share/doc" "$prefix/share/doc/typefinch" \
    "$prefix/share/doc/typefinch/third-party" "$vendor_target" \
    "$prefix/share/doc/typefinch/README.md" \
    "$prefix/share/doc/typefinch/LICENSE"; do
  [ ! -L "$managed_path" ] ||
    { echo "Refusing symlinked managed installation path" >&2; exit 1; }
done
mkdir -p "$bin" "$data"
if [ -L "$exe" ] || [ -L "$marker" ] || [ -L "$previous" ]; then
  echo "Refusing symlink install targets" >&2; exit 1
fi
if [ "$rollback" -eq 1 ]; then
  [ -f "$previous" ] && [ -f "$prior" ] || { echo "No prior managed version" >&2; exit 1; }
  install -m 755 "$previous" "$exe.rollback-new"
  mv -f "$exe.rollback-new" "$exe"
  mv -f "$prior" "$marker"
  rm -f "$previous"
  echo "Restored previous Typefinch executable: $exe"
  exit 0
fi
[ -f "$base/bin/typefinch" ] && [ -f "$base/METADATA.txt" ] ||
  { echo "Incomplete preview archive" >&2; exit 1; }
vendor_source="$base/share/doc/typefinch/third-party/epaint-0.35.0"
for notice in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
  [ -s "$vendor_source/$notice" ] || { echo "Incomplete upstream epaint notices" >&2; exit 1; }
  [ ! -L "$vendor_target" ] && [ ! -L "$vendor_target/$notice" ] ||
    { echo "Refusing symlinked notice destination" >&2; exit 1; }
done
if [ -e "$exe" ] && [ ! -f "$marker" ]; then
  echo "Refusing to overwrite unmanaged executable: $exe" >&2; exit 1
fi
if [ -e "$exe" ]; then
  install -m 755 "$exe" "$previous.new"
  mv -f "$previous.new" "$previous"
  cp "$marker" "$prior"
fi
install -m 755 "$base/bin/typefinch" "$exe.new"
mv -f "$exe.new" "$exe"
install -m 644 "$base/METADATA.txt" "$marker"
mkdir -p "$prefix/share/doc/typefinch"
install -m 644 "$base/share/doc/typefinch/README.md" "$prefix/share/doc/typefinch/README.md"
install -m 644 "$base/share/doc/typefinch/LICENSE" "$prefix/share/doc/typefinch/LICENSE"
mkdir -p "$vendor_target"
for notice in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
  install -m 644 "$vendor_source/$notice" "$vendor_target/$notice"
done
echo "Installed user-local Typefinch: $exe"
