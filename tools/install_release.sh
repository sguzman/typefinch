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
echo "Installed user-local Typefinch: $exe"
