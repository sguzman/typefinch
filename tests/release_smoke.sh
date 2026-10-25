#!/usr/bin/env bash
set -euo pipefail
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
archive="dist/typefinch-${version}-linux-x86_64.tar.gz"
[[ -s "$archive" ]] || exit 1
(cd dist && sha256sum --check "$(basename "$archive").sha256")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
tar -tzf "$archive" >"$tmp/members"
[[ "$(grep -cv '/$' "$tmp/members")" -eq 8 ]]
! grep -E '^/|(^|/)\.\.(/|$)' "$tmp/members"
tar -xzf "$archive" -C "$tmp"
root="$tmp/typefinch-${version}-linux-x86_64"
# Original epaint notices are required byte-for-byte in every preview archive.
for item in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
  notice="$root/share/doc/typefinch/third-party/epaint-0.35.0/$item"
  [[ -s "$notice" ]] || { echo "Missing pinned epaint notice: $item" >&2;exit 1; }
  cmp "third_party/epaint-0.35.0/$item" "$notice"
done
prefix="$tmp/prefix"
sh "$root/install.sh" --prefix "$prefix"
test -x "$prefix/bin/typefinch"
cmp "$root/bin/typefinch" "$prefix/bin/typefinch"
for notice in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
  cmp "$root/share/doc/typefinch/third-party/epaint-0.35.0/$notice" \
      "$prefix/share/doc/typefinch/third-party/epaint-0.35.0/$notice"
done
"$prefix/bin/typefinch" --version | grep -Fx "typefinch $version"
original="$(sha256sum "$prefix/bin/typefinch" | cut -d' ' -f1)"
printf '#!/bin/sh\necho upgraded\n' >"$root/bin/typefinch"
chmod 755 "$root/bin/typefinch"
sh "$root/install.sh" --prefix "$prefix"
updated="$(sha256sum "$prefix/bin/typefinch" | cut -d' ' -f1)"
[[ "$updated" != "$original" ]]
sh "$root/install.sh" --prefix "$prefix" --rollback
restored="$(sha256sum "$prefix/bin/typefinch" | cut -d' ' -f1)"
[[ "$restored" == "$original" ]]
"$prefix/bin/typefinch" --version | grep -Fx "typefinch $version"
foreign="$tmp/foreign"
mkdir -p "$foreign/bin"
printf '#!/bin/sh\necho foreign\n' >"$foreign/bin/typefinch"
chmod 755 "$foreign/bin/typefinch"
if sh "$root/install.sh" --prefix "$foreign"; then echo "Overwrote foreign executable" >&2; exit 1; fi
# Refuse symlinked managed directories and notice files before writing
# the binary. Nothing may be redirected outside the opted-in prefix.
for form in directory file ancestor readme bin;do
  blocked="$tmp/symlink-notice-$form"
  target="$blocked/share/doc/typefinch/third-party/epaint-0.35.0"
  mkdir -p "$(dirname "$target")" "$tmp/outside"
  case "$form" in
    directory) ln -s "$tmp/outside" "$target" ;;
    file) mkdir -p "$target"
      ln -s "$tmp/outside/notices" "$target/LICENSE-MIT" ;;
    ancestor) ln -s "$tmp/outside" "$(dirname "$target")" ;;
    readme) mkdir -p "$blocked/share/doc/typefinch"
      ln -s "$tmp/outside/readme" "$blocked/share/doc/typefinch/README.md" ;;
    bin) mkdir -p "$blocked"
      ln -s "$tmp/outside" "$blocked/bin" ;;
  esac
  if sh "$root/install.sh" --prefix "$blocked";then
    echo "Installed through a symlinked managed path" >&2;exit 1
  fi
  [ ! -e "$blocked/bin/typefinch" ] || { echo "Partially installed executable" >&2;exit 1; }
done
echo "Versioned Linux packaging, vendor notices, install/upgrade/rollback and collision tests PASS"
