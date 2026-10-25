#!/usr/bin/env bash
# Genuine nested Wayland GUI typing: Xvfb virtual hardware -> Weston X11
# backend -> Wayland wl_keyboard -> the production Typefinch egui surface.
# No physical hardware, Hyprland, privileges or global key interception.
set -Eeuo pipefail
: "${DISPLAY:?A disposable Xvfb DISPLAY is required}"
tmp="$(mktemp -d)"
weston_pid="" app_pid="" wm_pid=""
cleanup(){
  status=$?
  if ((status!=0));then
    for log in "$tmp/app.log" "$tmp/weston.log" "$tmp/weston.stdout";do
      [[ ! -s "$log" ]] || tail -n 75 "$log" >&2 || true
    done
  fi
  [[ -z "$app_pid" ]] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }
  [[ -z "$weston_pid" ]] || { kill "$weston_pid" 2>/dev/null || true; wait "$weston_pid" 2>/dev/null || true; }
  [[ -z "$wm_pid" ]] || { kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; }
  rm -rf "$tmp"
}
trap cleanup EXIT
mkdir -m 700 "$tmp/runtime"
mkdir -p "$tmp/config"
export XDG_RUNTIME_DIR="$tmp/runtime"
export XDG_CONFIG_HOME="$tmp/config"
export WAYLAND_DISPLAY=typefinch-wayland-input
export XDG_SESSION_TYPE=wayland
export WINIT_UNIX_BACKEND=wayland
export LIBGL_ALWAYS_SOFTWARE=1
export TYPEFINCH_QA_REPORT="$tmp/score.json"
export TYPEFINCH_QA_EDITOR_CARET_REPORT="$tmp/rtl-caret.json"
export TYPEFINCH_QA_IMPORT_REPORT="$tmp/rtl-import.json"
unset HYPRLAND_INSTANCE_SIGNATURE WAYLAND_SOCKET
printf '%s' 'abc' > "$tmp/source.txt"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
weston --backend=x11-backend.so --renderer=pixman --socket="$WAYLAND_DISPLAY" \
  --no-config --idle-time=0 --width=1024 --height=768 --log="$tmp/weston.log" \
  >"$tmp/weston.stdout" 2>&1 &
weston_pid=$!
for _ in $(seq 1 120);do
  [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] && break
  kill -0 "$weston_pid" 2>/dev/null || {
    echo "Nested Weston exited before Wayland socket" >&2;exit 1;
  }
  sleep 0.1
done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || {
  echo "Nested Weston Wayland socket not ready" >&2;exit 1;
}
parent=""
for _ in $(seq 1 100);do
  # Weston/X11 has differing title/class conventions across distributions.
  # Match its own PID first so Openbox, the Xvfb root, or another window
  # can never be mistaken for the compositor input bridge.
  parent="$(xdotool search --onlyvisible --pid "$weston_pid" 2>/dev/null|head -n 1||true)"
  if [[ -z "$parent" ]];then
    parent="$(xdotool search --onlyvisible --class '[Ww]eston' 2>/dev/null|head -n 1||true)"
  fi
  if [[ -z "$parent" ]];then
    parent="$(xdotool search --onlyvisible --name '[Ww]eston' 2>/dev/null|head -n 1||true)"
  fi
  [[ -n "$parent" ]] && break
  sleep 0.1
done
if [[ -z "$parent" ]];then
  echo "No Weston parent X11 window by PID/class/title" >&2
  for id in $(xdotool search --onlyvisible --name '.' 2>/dev/null||true);do
    printf 'Synthetic X11 window %s title=' "$id" >&2
    xdotool getwindowname "$id" >&2 || true
    printf ' class=' >&2
    xdotool getwindowclassname "$id" >&2 || true
  done
  exit 1
fi
./target/debug/typefinch >"$tmp/app.log" 2>&1 &
app_pid=$!
# A Weston 13 X11-backend race has repeatedly segfaulted while its first
# Wayland surface and F11 synthetic input were established <1 second after
# compositor startup. Let the disposable compositor settle before sending
# any input. Real typing and all score assertions remain mandatory.
sleep 0.75
kill -0 "$weston_pid" 2>/dev/null || {
  echo "Nested Weston died before native input stabilization" >&2;exit 1;
}
cfg="$XDG_CONFIG_HOME/typefinch/settings.json"
# The Wayland client can become live after Weston's X11 bridge is already
# focusable. A fixed sleep makes F11 a timing lottery on fast CI runners.
# Require the app's persisted response before injecting any private text.
custom_selected=0
for _ in $(seq 1 30);do
  kill -0 "$app_pid" 2>/dev/null || {
    echo "Typefinch exited before native Wayland keyboard input" >&2;exit 1;
  }
  xdotool windowactivate --sync "$parent" 2>/dev/null || true
  xdotool windowfocus "$parent" 2>/dev/null || true
  # Focus the *inner* Wayland surface, not merely the outer X11 window.
  xdotool mousemove --window "$parent" 500 340 click 1
  xdotool key --clearmodifiers F11
  sleep 0.3
  if [[ -f "$cfg" ]] && grep -q '"test_mode": "custom"' "$cfg";then
    custom_selected=1
    break
  fi
done
if ((custom_selected!=1));then
  # The image contains only a disposable synthetic fixture. Retain it for
  # failed CI diagnosis; never include a real profile or passage.
  import -window "$parent" typefinch-wayland-virtual.png 2>/dev/null || true
  echo "Wayland F11 did not select Custom after 30 focused attempts" >&2
  exit 1
fi
xdotool type --clearmodifiers --delay 25 "$tmp/source.txt"
xdotool key --clearmodifiers Return
sleep 0.55
xdotool key --clearmodifiers F5
sleep 0.3
xdotool key --clearmodifiers F2
sleep 0.2
xdotool type --clearmodifiers --delay 40 abc
sleep 0.8
lua5.4 - "$TYPEFINCH_QA_REPORT" "$tmp/source.txt" "$cfg" <<'LUA'
local json=require("dkjson")
local report,source,settings=assert(arg[1]),assert(arg[2]),assert(arg[3])
local f=assert(io.open(report,"rb"),"No scored native Wayland typing result")
local r=assert(json.decode(assert(f:read("*a"))));f:close()
assert(r.synthetic and r.state=="finished" and r.entered==3
  and r.correct==3 and r.accuracy==100,
  "Native Weston wl_keyboard did not deliver the complete test")
local db=os.getenv("XDG_CONFIG_HOME").."/typefinch/sessions.sqlite3"
local cmd=string.format("sqlite3 -json %q 'SELECT test_mode,input_mode,layout,entered,correct FROM sessions'",db)
local p=assert(io.popen(cmd,"r"))
local rows=assert(json.decode(assert(p:read("*a"))));assert(p:close())
assert(#rows==1 and rows[1].test_mode=="custom" and rows[1].input_mode=="OS"
  and rows[1].layout=="os:unknown" and rows[1].entered==3 and rows[1].correct==3,
  "Native Wayland provenance or score must remain aggregate/unattributed")
local cf=assert(io.open(settings,"rb"))
local body=assert(cf:read("*a"));cf:close()
local sf=assert(io.open(source,"rb"))
local passage=assert(sf:read("*a"));sf:close()
assert(not body:find(source,1,true),"Local source path persisted")
assert(not body:find(passage,1,true),"Private source text persisted")
print("Nested Weston real Wayland wl_keyboard PASS: 3/3 OS-input graphemes, unattributed seat, no text persistence")
LUA
import -window "$parent" typefinch-wayland-virtual.png
echo "Nested Weston production Wayland GUI typed and saved one clean Custom session"
# Reuse one live Wayland client. Weston 13's X11 backend can crash while
# rapidly tearing down/recreating this synthetic surface between scripts;
# the production editor itself resets and imports each new private source.
for script in arabic hebrew;do
  if [[ "$script" == arabic ]];then
    printf '%s' 'سَلَام بالعربية 123' >"$tmp/$script-source.txt"
  else
    printf '%s' 'שָׁלוֹם בעברית 123' >"$tmp/$script-source.txt"
  fi
  kill -0 "$app_pid" 2>/dev/null || {
    echo "Wayland $script native editor process exited prematurely" >&2;exit 1;
  }
  xdotool windowactivate --sync "$parent" 2>/dev/null || true
  xdotool windowfocus "$parent" 2>/dev/null || true
  xdotool key --clearmodifiers F11
  sleep 0.3
  xdotool key --clearmodifiers ctrl+a
  rm -f "$TYPEFINCH_QA_IMPORT_REPORT" "$TYPEFINCH_QA_EDITOR_CARET_REPORT"
  xdotool type --clearmodifiers --delay 25 "$tmp/$script-source.txt"
  xdotool key --clearmodifiers Return
  sleep 0.6
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$script" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Native Wayland RTL import not confirmed")
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="custom-import" and d.script==arg[2]
  and d.graphemes>8,"Wrong Arabic/Hebrew Wayland source or failed import")
LUA
  xdotool key --clearmodifiers ctrl+e
  sleep 0.35
  xdotool key --clearmodifiers ctrl+Home
  sleep 0.25
  [[ -s "$TYPEFINCH_QA_EDITOR_CARET_REPORT" ]] || {
    echo "Wayland $script native editor has no caret report" >&2;exit 1;
  }
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-click-before.json"
  # Use the ACTUAL shaped COSMIC preview to seek a non-initial logical
  # caret with a normal native wl_pointer click. The read-only preview
  # must not modify text, create a selection or intercept typing; it
  # hands a grapheme-aligned cursor to egui's existing source TextEdit.
  xdotool mousemove --window "$parent" 258 539 click 1
  sleep 0.5
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-click-after.json"
  lua5.4 - "$tmp/$script-click-before.json" "$tmp/$script-click-after.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"))
  local data=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(data.synthetic and data.state=="rtl-editor-caret" and data.caret_positioned)
  return data
end
local before,after=read(arg[1]),read(arg[2])
assert(before.index==0 and before.selected_graphemes==0)
assert(after.total==before.total and after.index>0 and after.index<after.total
  and after.selected_graphemes==0,
  "Real COSMIC shaped-preview click did not move the native caret without changing source")
print("Native Weston RTL visual hit-test and source caret move PASS: 0 -> "..after.index)
LUA
  import -window "$parent" "typefinch-wayland-rtl-$script-preview-click.png"
  # The visual click must also leave the *native* source field writable:
  # type an ordinary Latin character at the hit-tested grapheme boundary,
  # verify the insertion, and Undo the edit before running other checks.
  xdotool type --clearmodifiers --delay 35 x
  inserted=0
  for _ in $(seq 1 45);do
    if [[ -s "$TYPEFINCH_QA_EDITOR_CARET_REPORT" ]] &&
      lua5.4 - "$tmp/$script-click-after.json" "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA' >/dev/null 2>&1
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"))
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret")
  return d
end
local prior,updated=read(arg[1]),read(arg[2])
assert(updated.total==prior.total+1 and updated.index==prior.index+1
  and updated.selected_graphemes==0,
  "Visual click must position actual editable caret before insertion")
LUA
    then inserted=1;break;fi
    sleep 0.12
  done
  ((inserted==1)) || {
    echo "Native Wayland $script shaped click did not allow insertion at selected grapheme" >&2
    exit 1
  }
  xdotool key --clearmodifiers ctrl+z
  restored=0
  for _ in $(seq 1 45);do
    if [[ -s "$TYPEFINCH_QA_EDITOR_CARET_REPORT" ]] &&
      lua5.4 - "$tmp/$script-click-before.json" "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA' >/dev/null 2>&1
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"))
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret")
  return d
end
local original,restored=read(arg[1]),read(arg[2])
assert(restored.total==original.total and restored.selected_graphemes==0,
  "Undo after visual caret seek must preserve original source")
LUA
    then restored=1;break;fi
    sleep 0.12
  done
  ((restored==1)) || { echo "Native Wayland $script preview-click Undo failed" >&2;exit 1; }
  echo "Native Weston $script shaped-preview seek, actual edit and Undo PASS"
  # Real wl_pointer press/motion/release across COSMIC's visual glyphs:
  # egui's source TextEdit must receive a whole-grapheme selection without
  # losing text or converting the preview into a separate input field.
  # Coordinates are grounded in the reviewed nested 1024x768 Weston frame.
  xdotool key --clearmodifiers ctrl+Home
  sleep 0.25
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-preview-drag-before.json"
  import -window "$parent" "typefinch-wayland-rtl-$script-preview-drag-before.png"
  xdotool mousemove --window "$parent" 228 539 mousedown 1
  sleep 0.15
  xdotool mousemove --window "$parent" 325 539
  sleep 0.25
  xdotool mouseup 1
  sleep 0.55
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-preview-drag-after.json"
  import -window "$parent" "typefinch-wayland-rtl-$script-preview-drag-after.png"
  lua5.4 - "$tmp/$script-preview-drag-before.json" \
    "$tmp/$script-preview-drag-after.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native Wayland preview drag report")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local before,after=read(arg[1]),read(arg[2])
assert(before.selected_graphemes==0 and before.total>8,
  "Wayland shaped-preview drag started from an invalid source state")
assert(after.total==before.total and after.selected_graphemes>=2
  and after.selected_graphemes<before.total
  and after.index>=0 and after.index<=before.total,
  "Wayland shaped-preview drag must create a partial selection without changing text")
print("Native Weston shaped RTL preview drag PASS: "..after.selected_graphemes.." selected graphemes")
LUA
  for state in before after;do
    convert "typefinch-wayland-rtl-$script-preview-drag-$state.png" \
      -crop 345x52+18+513 +repage "$tmp/$script-preview-drag-$state-crop.png"
  done
  preview_pixels="$(compare -metric AE "$tmp/$script-preview-drag-before-crop.png" \
    "$tmp/$script-preview-drag-after-crop.png" null: 2>&1 || true)"
  [[ "$preview_pixels" =~ ^[0-9]+$ && "$preview_pixels" -gt 80 ]] || {
    echo "Weston $script shaped-preview drag selection not visibly painted: $preview_pixels" >&2
    exit 1
  }
  xdotool key --clearmodifiers ctrl+Home
  sleep 0.3
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-start.json"
  direction=''
  for side in Right Left;do
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.15
    xdotool key --clearmodifiers "shift+$side"
    sleep 0.25
    status="$(lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret")
print(tostring(d.selected_graphemes)..":"..tostring(d.index))
LUA
)"
    [[ "$status" != "1:1" ]] || { direction="$side";break; }
  done
  [[ -n "$direction" ]] || {
    echo "Wayland $script Shift+Arrow failed whole grapheme: $status" >&2;exit 1;
  }
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-selected.json"
  import -window "$parent" "typefinch-wayland-rtl-$script.png"
  if [[ "$direction" == Right ]];then reverse=Left;else reverse=Right;fi
  xdotool key --clearmodifiers "shift+$reverse"
  sleep 0.25
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-back.json"
  lua5.4 - "$tmp/$script-start.json" "$tmp/$script-selected.json" \
    "$tmp/$script-back.json" "$cfg" "$tmp/$script-source.txt" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"))
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local start,selected,back=read(arg[1]),read(arg[2]),read(arg[3])
assert(start.total>8 and start.index==0 and start.selected_graphemes==0)
assert(selected.total==start.total and selected.index==1 and selected.selected_graphemes==1)
assert(back.total==start.total and back.index==0 and back.selected_graphemes==0)
local cf=assert(io.open(arg[4],"rb"));local settings=assert(cf:read("*a"));cf:close()
local sf=assert(io.open(arg[5],"rb"));local passage=assert(sf:read("*a"));sf:close()
assert(not settings:find(arg[5],1,true) and not settings:find(passage,1,true),
  "Native Wayland source or local path leaked into settings")
print("Native Weston Wayland RTL keyboard selection PASS: 0 -> 1 -> 0 with private source")
LUA
  # Verify the production native Wayland editor actually mutates the source,
  # not only the shaped preview: one entire marked grapheme must disappear,
  # and native Undo must restore it. Persist only counts, never source bytes.
  xdotool key --clearmodifiers ctrl+Home
  sleep 0.2
  xdotool key --clearmodifiers "shift+$direction"
  sleep 0.3
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-before-delete.json"
  xdotool key --clearmodifiers BackSpace
  sleep 0.4
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-after-delete.json"
  xdotool key --clearmodifiers ctrl+z
  sleep 0.4
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-after-undo.json"
  lua5.4 - "$tmp/$script-start.json" "$tmp/$script-before-delete.json"     "$tmp/$script-after-delete.json" "$tmp/$script-after-undo.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing Wayland RTL mutation report")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local start,selected,deleted,restored=read(arg[1]),read(arg[2]),read(arg[3]),read(arg[4])
assert(start.total>8 and start.index==0 and start.selected_graphemes==0,
  "Wayland RTL mutation did not start at first grapheme")
assert(selected.total==start.total and selected.index==1 and selected.selected_graphemes==1,
  "Wayland RTL mutation did not select precisely one full marked grapheme")
assert(deleted.total==start.total-1 and deleted.index==0 and deleted.selected_graphemes==0,
  "Wayland native Backspace did not remove exactly one grapheme")
assert(restored.total==start.total and restored.selected_graphemes==0,
  string.format("Wayland native Undo mismatch: initial=%d selected=%d deleted=%d restored=%d restored_selection=%d restored_index=%d",
    start.total,selected.total,deleted.total,restored.total,
    restored.selected_graphemes,restored.index))
print("Native Wayland marked RTL grapheme Backspace/Undo PASS: "..
  start.total.." -> "..deleted.total.." -> "..restored.total)
LUA
  # Exercise whole-range keyboard selection across connected RTL glyphs and
  # the trailing left-to-right digits. The actual source TextEdit, rather than
  # a synthetic text-layout mock, must preserve both selection endpoints.
  xdotool key --clearmodifiers ctrl+Home
  sleep 0.2
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-whole-start.json"
  import -window "$parent" "$tmp/$script-whole-before.png"
  xdotool key --clearmodifiers ctrl+shift+End
  sleep 0.3
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-whole-selected.json"
  import -window "$parent" "typefinch-wayland-rtl-$script-whole.png"
  # Inspecting only metadata would miss a stale/unpainted shaped preview.
  # Synthetic, self-contained screenshots are safe for the CI artifact.
  pixels="$(compare -metric AE "$tmp/$script-whole-before.png" \
    "typefinch-wayland-rtl-$script-whole.png" null: 2>&1 || true)"
  [[ "$pixels" =~ ^[0-9]+$ && "$pixels" -gt 200 ]] || {
    echo "Wayland full-range RTL selection did not visibly repaint: pixels=$pixels" >&2
    exit 1
  }
  xdotool key --clearmodifiers ctrl+shift+Home
  sleep 0.3
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-whole-retracted.json"
  lua5.4 - "$tmp/$script-whole-start.json"     "$tmp/$script-whole-selected.json"     "$tmp/$script-whole-retracted.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native Wayland full-range RTL audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local start,selected,back=read(arg[1]),read(arg[2]),read(arg[3])
assert(start.total>8 and start.index==0 and start.selected_graphemes==0,
  "Wayland full-range RTL selection lacks a logical start boundary")
assert(selected.total==start.total and selected.index==start.total
  and selected.selected_graphemes==start.total,
  "Wayland Ctrl+Shift+End did not select the entire mixed RTL/LTR source")
assert(back.total==start.total and back.index==0 and back.selected_graphemes==0,
  "Wayland Ctrl+Shift+Home failed to retract to the RTL selection anchor")
print("Native Wayland whole-range mixed-direction selection PASS: 0 -> "..
  selected.selected_graphemes.." -> 0")
LUA
  # Real Weston wl_pointer dragging in the native *editable* Arabic/Hebrew
  # TextEdit. Click and drag within the rendered marked-script first line;
  # read only logical counts, never the selected source bytes. Both drag
  # directions should select complete grapheme clusters and leave the
  # source unchanged. This tests a limited same-line pointer range, not
  # arbitrary mixed BiDi visual hit testing or production Hyprland.
  xdotool mousemove --window "$parent" 30 439 mousedown 1
  sleep 0.12
  xdotool mousemove --window "$parent" 92 439
  sleep 0.25
  xdotool mouseup 1
  sleep 0.25
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-pointer-forward.json"
  import -window "$parent" "typefinch-wayland-rtl-$script-pointer-forward.png"
  xdotool mousemove --window "$parent" 92 439 mousedown 1
  sleep 0.12
  xdotool mousemove --window "$parent" 30 439
  sleep 0.25
  xdotool mouseup 1
  sleep 0.25
  cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$script-pointer-backward.json"
  import -window "$parent" "typefinch-wayland-rtl-$script-pointer-backward.png"
  lua5.4 - "$tmp/$script-start.json" "$tmp/$script-pointer-forward.json" \
      "$tmp/$script-pointer-backward.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local file=assert(io.open(path,"rb"),"Native pointer-drag audit missing")
  local data=assert(json.decode(assert(file:read("*a"))));file:close()
  assert(data.synthetic and data.state=="rtl-editor-caret"
    and data.caret_positioned and type(data.index)=="number"
    and type(data.selected_graphemes)=="number")
  return data
end
local baseline,forward,backward=read(arg[1]),read(arg[2]),read(arg[3])
assert(baseline.total>8 and forward.total==baseline.total
  and backward.total==baseline.total,
  "Native pointer gesture mutated the source instead of selecting it")
assert(forward.selected_graphemes>=1 and forward.selected_graphemes<forward.total
  and backward.selected_graphemes>=1 and backward.selected_graphemes<backward.total,
  string.format("Native RTL pointer drag did not select a nonempty bounded range: %d/%d",
    forward.selected_graphemes,backward.selected_graphemes))
assert(forward.index>=0 and forward.index<=forward.total and
  backward.index>=0 and backward.index<=backward.total)
print("Native Wayland marked-script real pointer-drag selection PASS: "..
  forward.selected_graphemes.." forward / "..backward.selected_graphemes..
  " reverse logical graphemes, full source preserved")
LUA
done
echo "Nested Weston Wayland Arabic/Hebrew editing, full-range and pointer-selection regressions PASS"
