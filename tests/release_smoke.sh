#!/usr/bin/env bash
set -euo pipefail
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)"
archive="dist/typefinch-${version}-linux-x86_64.tar.gz"
[[ -s "$archive" ]] || exit 1
(cd dist && sha256sum --check "$(basename "$archive").sha256")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
tar -tzf "$archive" >"$tmp/members"
[[ "$(grep -cv '/$' "$tmp/members")" -eq 14 ]]
! grep -E '^/|(^|/)\.\.(/|$)' "$tmp/members"
tar -xzf "$archive" -C "$tmp"
root="$tmp/typefinch-${version}-linux-x86_64"
# Each pinned source ships its exact original MIT/Apache texts and provenance.
for vendor in epaint-0.35.0 egui-0.35.0 egui-winit-0.35.0;do
  for item in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
    notice="$root/share/doc/typefinch/third-party/$vendor/$item"
    [[ -s "$notice" ]] || { echo "Missing vendor notice: $vendor/$item" >&2;exit 1; }
    cmp "third_party/$vendor/$item" "$notice"
  done
done
prefix="$tmp/prefix"
sh "$root/install.sh" --prefix "$prefix"
test -x "$prefix/bin/typefinch"
cmp "$root/bin/typefinch" "$prefix/bin/typefinch"
for vendor in epaint-0.35.0 egui-0.35.0 egui-winit-0.35.0;do
  for notice in LICENSE-MIT LICENSE-APACHE UPSTREAM.md;do
    cmp "$root/share/doc/typefinch/third-party/$vendor/$notice" \
        "$prefix/share/doc/typefinch/third-party/$vendor/$notice"
  done
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
    ancestor) rmdir "$(dirname "$target")"
      ln -s "$tmp/outside" "$(dirname "$target")" ;;
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
# Independently exercise the extra vendor destinations against symlinks.
for vendor in egui-0.35.0 egui-winit-0.35.0;do
  for form in directory file;do
    blocked="$tmp/symlink-$vendor-$form"
    target="$blocked/share/doc/typefinch/third-party/$vendor"
    mkdir -p "$(dirname "$target")" "$tmp/outside"
    if [[ "$form" == directory ]];then
      ln -s "$tmp/outside" "$target"
    else
      mkdir -p "$target"
      ln -s "$tmp/outside/notices" "$target/LICENSE-MIT"
    fi
    if sh "$root/install.sh" --prefix "$blocked";then
      echo "Installed through symlinked pinned vendor" >&2;exit 1
    fi
    [[ ! -e "$blocked/bin/typefinch" ]] ||
      { echo "Partially installed executable" >&2;exit 1; }
  done
done
# Reject a linked installation prefix (or any of its ancestors).
escape_target="$tmp/install-prefix-external"
mkdir -p "$escape_target"
ln -s "$escape_target" "$tmp/prefix-link"
for candidate in "$tmp/prefix-link" "$tmp/prefix-link/deep/path";do
  if sh "$root/install.sh" --prefix "$candidate";then
    echo "Installer followed symlinked prefix ancestor" >&2;exit 1
  fi
  [[ ! -e "$escape_target/bin/typefinch" ]] ||
    { echo "Prefix link redirected an executable write" >&2;exit 1; }
done
for candidate in "relative/typefinch" "$tmp/../$(basename "$tmp")/escaped" \
                 "$tmp/./escaped";do
  if sh "$root/install.sh" --prefix "$candidate";then
    echo "Installer accepted ambiguous/relative destination" >&2;exit 1
  fi
done
# Check rollback metadata and .new destinations without overwriting data.
printf 'never-overwrite\n' >"$escape_target/sentinel"
for target in "share/typefinch/previous.version" \
              "bin/typefinch.new" \
              "bin/typefinch.rollback-new" \
              "share/typefinch/previous-binary.new";do
  prefix_guard="$tmp/prefix-guard-$(echo "$target" | tr '/' '-')"
  mkdir -p "$prefix_guard/$(dirname "$target")"
  if [[ "$target" == *previous.version ]];then
    ln -s "$escape_target/sentinel" "$prefix_guard/$target"
  else
    printf 'staged sentinel\n' >"$prefix_guard/$target"
  fi
  if sh "$root/install.sh" --prefix "$prefix_guard";then
    echo "Installer accepted occupied staging/symlink metadata" >&2;exit 1
  fi
  [[ ! -e "$prefix_guard/bin/typefinch" ]] ||
    { echo "Staging guard caused partial installation" >&2;exit 1; }
  if [[ "$target" != *previous.version ]];then
    [[ "$(cat "$prefix_guard/$target")" == "staged sentinel" ]] ||
      { echo "Installer overwrote an existing staging file" >&2;exit 1; }
  fi
done
[[ "$(cat "$escape_target/sentinel")" == "never-overwrite" ]] ||
  { echo "Installer overwrote external rollback target" >&2;exit 1; }
echo "Versioned Linux packaging, three vendor notices, install/upgrade/rollback and symlink/collision tests PASS"
