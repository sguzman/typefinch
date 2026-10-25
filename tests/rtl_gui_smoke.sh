#!/usr/bin/env bash
# Real X11 import/active-caret/error QA for isolated synthetic RTL passages.
# Each script gets its own fresh Typefinch process and XDG_CONFIG_HOME.
# No real keyboard, private text, hardware or IME acceptance implied.
set -Eeuo pipefail
export LIBGL_ALWAYS_SOFTWARE=1 WINIT_UNIX_BACKEND=x11 XDG_SESSION_TYPE=x11
tmp="$(mktemp -d)"
app_pid="" wm_pid="" window=""
cleanup(){
  status=$?
  if (( status != 0 ));then
    for log in "$tmp"/*/app.log;do
      [[ ! -s "$log" ]] || tail -n 60 "$log" >&2 || true
    done
  fi
  if [[ -n "$app_pid" ]];then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$wm_pid" ]];then
    kill "$wm_pid" 2>/dev/null || true
    wait "$wm_pid" 2>/dev/null || true
  fi
  if [[ -n "${GITHUB_WORKSPACE:-}" ]];then
    for img in "$tmp"/rtl-*.png;do
      [[ ! -s "$img" ]] || cp "$img" "$GITHUB_WORKSPACE/"
    done
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT
printf '%s' 'سَلَام بالعربية 123' > "$tmp/arabic.txt"
printf '%s' 'שָׁלוֹם בעברית 123' > "$tmp/hebrew.txt"
# An RTL-base line, a Latin-base line, and Hebrew/Arabic with ASCII digits
# and punctuation. Keep this text synthetic and out of persisted settings.
printf '%s' 'سَلَام (ABC 123), שָׁלוֹם!
English XYZ 42
שלום, مرحبا! 2026' > "$tmp/mixed.txt"
# Long original synthetic fixture, with enough connected RTL words to test
# responsive wrapping, a forced newline and a narrow reference viewport.
for _ in $(seq 1 24);do printf '%s' 'שלום עולם ' >> "$tmp/hebrew-long.txt";done
printf '\nסוף 123' >> "$tmp/hebrew-long.txt"
openbox --sm-disable >"$tmp/wm.log" 2>&1 &
wm_pid=$!
sleep 0.4
start_app(){
  local name="$1"
  export XDG_CONFIG_HOME="$tmp/$name/config"
  export TYPEFINCH_QA_IMPORT_REPORT="$tmp/$name/import.json"
  export TYPEFINCH_QA_EDITOR_CARET_REPORT="$tmp/$name/editor-caret.json"
  mkdir -p "$XDG_CONFIG_HOME"
  ./target/debug/typefinch >"$tmp/$name/app.log" 2>&1 &
  app_pid=$!
  window=""
  for _ in $(seq 1 200);do
    window="$(xdotool search --onlyvisible --name 'Typefinch' 2>/dev/null | head -n 1 || true)"
    [[ -z "$window" ]] || break
    if ! kill -0 "$app_pid" 2>/dev/null;then
      cat "$tmp/$name/app.log" >&2;exit 1
    fi
    sleep 0.25
  done
  [[ -n "$window" ]] || { echo "No native RTL QA window for $name" >&2;exit 1; }
  focus_window
}
focus_window(){
  xdotool windowactivate --sync "$window" 2>/dev/null || true
  xdotool windowfocus --sync "$window"
  sleep 0.25
}
stop_app(){
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=""
  window=""
  sleep 0.3
}
for name in arabic hebrew hebrew-long mixed;do
  mkdir -p "$tmp/$name"
  start_app "$name"
  import -window "$window" "$tmp/rtl-$name-baseline.png"
  xdotool key --clearmodifiers F11
  sleep 0.45
  grep -q '"test_mode": "custom"' "$XDG_CONFIG_HOME/typefinch/settings.json" || {
    echo "$name did not switch into Custom mode" >&2;exit 1;
  }
  xdotool type --clearmodifiers --delay 25 "$tmp/$name.txt"
  xdotool key --clearmodifiers Return
  sleep 0.65
  # Strong fixture-identity assertion. The app emits only a synthetic script
  # category and reference grapheme count, never text or a private file path.
  expected_script="${name%%-*}"
  lua5.4 - "$TYPEFINCH_QA_IMPORT_REPORT" "$expected_script" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"No import audit file")
local r=assert(json.decode(f:read("*a")))
f:close()
assert(r.state=="custom-import" and r.synthetic==true,"Import not confirmed")
assert(r.script==arg[2], "Wrong language fixture imported: "..tostring(r.script))
assert(type(r.graphemes)=="number" and r.graphemes>=10,
  "Imported reference missing or truncated")
print("Native RTL import identity PASS: "..r.script.."; "..r.graphemes.." graphemes")
LUA
  if [[ "$name" == arabic || "$name" == hebrew ]];then
    # Keyboard-only switch into the ordinary writable egui editor. The
    # visual COSMIC preview caret must move when its primary cursor moves.
    focus_window
    xdotool key --clearmodifiers ctrl+e
    sleep 0.4
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.3
    import -window "$window" "$tmp/rtl-$name-editor-start.png"
    [[ -s "$TYPEFINCH_QA_EDITOR_CARET_REPORT" ]] || {
      echo "Native RTL editor has no caret audit: $name" >&2;exit 1;
    }
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-start.json"
    xdotool key --clearmodifiers ctrl+End
    sleep 0.3
    import -window "$window" "$tmp/rtl-$name-editor-end.png"
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-end.json"
    xdotool key --clearmodifiers ctrl+a
    sleep 0.35
    import -window "$window" "$tmp/rtl-$name-editor-selection.png"
    # Compare only the shaped preview glyphs, not the egui source editor.
    # These disposable test screenshots use a fixed 1060x760 X11 window;
    # a changed source-field selection alone must not satisfy this check.
    for state in end selection;do
      convert "$tmp/rtl-$name-editor-$state.png" \
        -crop 195x58+120+467 +repage \
        "$tmp/$name/editor-$state-preview.png"
    done
    selected_pixels="$(compare -metric AE \
      "$tmp/$name/editor-end-preview.png" \
      "$tmp/$name/editor-selection-preview.png" null: 2>&1 || true)"
    [[ "$selected_pixels" =~ ^[0-9]+$ && "$selected_pixels" -gt 80 ]] || {
      echo "RTL shaped preview did not visibly highlight selection: $name ($selected_pixels)" >&2
      exit 1
    }
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-selection.json"
    lua5.4 - "$tmp/$name/editor-start.json" "$tmp/$name/editor-end.json" "$tmp/$name/editor-selection.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"No RTL editor audit")
  local data=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(data.synthetic==true and data.state=="rtl-editor-caret")
  assert(data.caret_positioned and data.index>=data.visible_start
    and data.index<=data.visible_end,"Shaped preview caret outside visible window")
  return data
end
local first,last=read(assert(arg[1])),read(assert(arg[2]))
local selection=read(assert(arg[3]))
assert(selection.total==last.total and selection.selected_graphemes==last.total,
  "Ctrl+A did not select all logical RTL graphemes")
assert(first.selected_graphemes==0 and last.selected_graphemes==0)
assert(first.index==0,"Ctrl+Home did not reset the native editor cursor")
assert(last.total==first.total and last.total>=10,"RTL editor source changed unexpectedly")
assert(last.index==last.total,
  "Ctrl+End did not reach the actual logical end in the shaped preview")
print("Native RTL editor cursor audit PASS: 0 to "..last.index.." graphemes")
LUA
    editor_delta="$(compare -metric AE "$tmp/rtl-$name-editor-start.png" "$tmp/rtl-$name-editor-end.png" null: 2>&1 || true)"
    [[ "$editor_delta" =~ ^[0-9]+$ && "$editor_delta" -gt 30 ]] || {
      echo "Native RTL source editor caret/preview did not move: $name / $editor_delta" >&2
      exit 1
    }
    echo "Native $name writable editor cursor and shaped preview moved ($editor_delta pixels)"
    # Select the actual first marked Arabic/Hebrew script grapheme,
    # not a trailing ASCII digit. The native editor now snaps a *single*
    # Shift+Arrow selection to a whole grapheme boundary.
    partial_status=''
    selected_direction=''
    for direction in Right Left; do
      xdotool key --clearmodifiers ctrl+Home
      sleep 0.15
      xdotool key --clearmodifiers "shift+$direction"
      sleep 0.3
      partial_status="$(lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
print(tostring(d.selected_graphemes)..":"..tostring(d.index))
LUA
)"
      if [[ "$partial_status" == "1:1" ]];then
        selected_direction="$direction"
        break
      fi
    done
    [[ "$partial_status" == "1:1" ]] || {
      import -window "$window" "rtl-$name-editor-first-grapheme-debug.png" 2>/dev/null || true
      echo "Native $name first-script-grapheme selection failed: $partial_status" >&2
      exit 1
    }
    import -window "$window" "$tmp/rtl-$name-editor-partial.png"
    convert "$tmp/rtl-$name-editor-partial.png" \
      -crop 195x58+120+467 +repage "$tmp/$name/editor-partial-preview.png"
    convert "$tmp/rtl-$name-editor-start.png" \
      -crop 195x58+120+467 +repage "$tmp/$name/editor-start-preview.png"
    partial_pixels="$(compare -metric AE \
      "$tmp/$name/editor-start-preview.png" \
      "$tmp/$name/editor-partial-preview.png" null: 2>&1 || true)"
    [[ "$partial_pixels" =~ ^[0-9]+$ && "$partial_pixels" -gt 8 ]] || {
      echo "Partial $name selection did not visibly change shaped preview ($partial_pixels)" >&2
      exit 1
    }
    echo "Native $name first RTL script grapheme selection PASS: one cluster; $partial_pixels preview pixels changed"
    # The opposite Shift+Arrow must contract that exact marked cluster
    # back to its anchor. Without direction-aware snapping the editor
    # can get stuck selecting the same grapheme indefinitely.
    if [[ "$selected_direction" == Right ]];then reverse=Left;else reverse=Right;fi
    xdotool key --clearmodifiers "shift+$reverse"
    sleep 0.3
    retracted_status="$(lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
print(tostring(d.selected_graphemes)..":"..tostring(d.index))
LUA
)"
    [[ "$retracted_status" == "0:0" ]] || {
      echo "Native $name RTL Shift+Arrow could not retract selected grapheme: $retracted_status" >&2
      exit 1
    }
    echo "Native $name reverse RTL selection PASS: selected one full cluster and retracted to anchor"
    # Ordinary (unshifted) Arrow navigation must also skip diacritic
    # interiors with a single physical keypress, without selecting text.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.15
    xdotool key --clearmodifiers "$selected_direction"
    sleep 0.3
    plain_forward="$(lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
print(tostring(d.selected_graphemes)..":"..tostring(d.index))
LUA
)"
    [[ "$plain_forward" == "0:1" ]] || {
      echo "Native $name plain Arrow did not advance one whole RTL grapheme: $plain_forward" >&2
      exit 1
    }
    xdotool key --clearmodifiers "$reverse"
    sleep 0.3
    plain_reverse="$(lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(assert(arg[1]),"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
print(tostring(d.selected_graphemes)..":"..tostring(d.index))
LUA
)"
    [[ "$plain_reverse" == "0:0" ]] || {
      echo "Native $name plain Arrow did not return to previous whole grapheme: $plain_reverse" >&2
      exit 1
    }
    echo "Native $name plain Arrow grapheme navigation PASS: 0 -> 1 -> 0"
    # Drag through the *shaped read-only preview*, not the native source
    # field. Its visual hit-tests must transfer a grapheme-safe selection
    # into the actual editable egui TextEdit and visibly color the preview.
    # Coordinates are from independently inspected 1060x760 CI screenshots:
    # connected script glyphs occupy x=140..300, y=506..536.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    import -window "$window" "$tmp/rtl-$name-preview-drag-before.png"
    xdotool mousemove --window "$window" 175 521 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 265 521
    sleep 0.25
    xdotool mouseup 1
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-drag.json"
    import -window "$window" "$tmp/rtl-$name-preview-drag-after.png"
    lua5.4 - "$tmp/$name/preview-drag.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Missing native shaped-preview drag report")
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
assert(d.total>=10 and d.selected_graphemes>=2
  and d.selected_graphemes<d.total,
  "Shaped-preview pointer drag failed to select a partial Unicode range")
assert(d.index>=0 and d.index<=d.total,
  "Shaped-preview selection caret outside logical source bounds")
print("Native shaped RTL preview drag PASS: "..d.selected_graphemes.." graphemes")
LUA
    for state in before after;do
      convert "$tmp/rtl-$name-preview-drag-$state.png" \
        -crop 280x48+18+500 +repage \
        "$tmp/$name/preview-drag-$state-crop.png"
    done
    preview_pixels="$(compare -metric AE "$tmp/$name/preview-drag-before-crop.png" \
      "$tmp/$name/preview-drag-after-crop.png" null: 2>&1 || true)"
    [[ "$preview_pixels" =~ ^[0-9]+$ && "$preview_pixels" -gt 80 ]] || {
      echo "Shaped $name preview selection not visibly painted: $preview_pixels" >&2
      exit 1
    }
    # Reverse the visual gesture across the same glyphs. The primary
    # caret must follow the new pointer endpoint rather than retaining the
    # first drag's primary CCursor, and the underlying source stays intact.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool mousemove --window "$window" 265 521 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 175 521
    sleep 0.25
    xdotool mouseup 1
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-drag-reverse.json"
    import -window "$window" "$tmp/rtl-$name-preview-drag-reverse.png"
    lua5.4 - "$tmp/$name/preview-drag.json" \
      "$tmp/$name/preview-drag-reverse.json" <<'LUA'
local json=require("dkjson")
local function read(file)
  local f=assert(io.open(file,"rb"),"Missing native shaped-preview drag audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local forward,reverse=read(arg[1]),read(arg[2])
assert(reverse.total==forward.total and reverse.selected_graphemes>=2
  and reverse.selected_graphemes<reverse.total,
  "Reverse shaped-preview gesture did not retain a partial selection")
assert(reverse.selected_graphemes==forward.selected_graphemes,
  "Identical reverse visual range selected a different number of graphemes")
assert(reverse.index~=forward.index,
  "Reverse preview drag did not move the primary native caret endpoint")
print("Native RTL preview forward/reverse selection PASS: "..
  forward.selected_graphemes.." graphemes; active caret "..
  forward.index.." -> "..reverse.index)
LUA
    # The selected range must remain owned by the *editable* TextEdit:
    # replace it with one Latin grapheme, Undo, and verify exact restoration
    # of every Arabic/Hebrew combining mark. Counts alone are insufficient.
    xdotool type --clearmodifiers --delay 35 x
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-replaced.json"
    xdotool key --clearmodifiers ctrl+z
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-undo.json"
    lua5.4 - "$tmp/$name/preview-drag-reverse.json" \
      "$tmp/$name/preview-replaced.json" "$tmp/$name/preview-undo.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing preview replacement audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local selected,replaced,restored=read(arg[1]),read(arg[2]),read(arg[3])
assert(selected.selected_graphemes>=2,
  "Expected a multi-grapheme reverse preview selection")
assert(replaced.total==selected.total-selected.selected_graphemes+1
  and replaced.selected_graphemes==0,
  "The shaped-preview selection failed to replace in the native TextEdit")
assert(restored.total==selected.total and restored.selected_graphemes==0,
  "Native Undo did not restore preview-selected grapheme count")
print("Native shaped-preview selected-range edit/Undo counts PASS")
LUA
    xdotool key --clearmodifiers ctrl+a
    xdotool key --clearmodifiers ctrl+c
    sleep 0.35
    timeout 5s xclip -selection clipboard -o > "$tmp/$name/preview-restored.txt"
    cmp -s "$tmp/$name.txt" "$tmp/$name/preview-restored.txt" || {
      echo "Native shaped-preview edit/Undo changed original Unicode bytes" >&2
      exit 1
    }
    echo "Native shaped-preview selection editing and byte-exact Undo PASS"
    # Hebrew at x=175 lands on the *last* logical grapheme and selects
    # the entire fixture. Its connected glyphs begin farther right than
    # Arabic's; x=220 is an independently demonstrated partial-range hit.
    shift_click_x=175
    if [[ "$name" == hebrew ]];then shift_click_x=220;fi
    # Shift+click in the shaped *image* must extend the native TextEdit's
    # existing source anchor. A second Shift+click moves only the active
    # endpoint; a plain click collapses it. No IME or key capture is used.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool keydown Shift_L
    xdotool mousemove --window "$window" "$shift_click_x" 521 click 1
    sleep 0.3
    xdotool keyup Shift_L
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-shift-first.json"
    import -window "$window" "$tmp/rtl-$name-preview-shift-first.png"
    xdotool keydown Shift_L
    xdotool mousemove --window "$window" 265 521 click 1
    sleep 0.3
    xdotool keyup Shift_L
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-shift-second.json"
    import -window "$window" "$tmp/rtl-$name-preview-shift-second.png"
    lua5.4 - "$tmp/$name/preview-shift-first.json" \
      "$tmp/$name/preview-shift-second.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native shaped Shift-click audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local first,second=read(arg[1]),read(arg[2])
assert(first.total>8 and first.selected_graphemes>=2
  and first.selected_graphemes<first.total,
  "First shaped-preview Shift-click did not extend a native source selection")
assert(second.total==first.total and second.selected_graphemes<first.selected_graphemes,
  "Second shaped-preview Shift-click did not retract toward the original anchor")
print("Native shaped-preview Shift-click selection PASS: "..
  first.selected_graphemes.." -> "..second.selected_graphemes.." graphemes")
LUA
    xdotool mousemove --window "$window" 220 521 click 1
    sleep 0.4
    lua5.4 - "$TYPEFINCH_QA_EDITOR_CARET_REPORT" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"))
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret"
  and d.selected_graphemes==0 and d.caret_positioned,
  "Plain shaped-preview click failed to collapse Shift-click selection")
print("Native shaped-preview plain click collapses Shift-selection PASS")
LUA
    # The extended selection belongs to native TextEdit, not the preview
    # alone. Replace then Undo and compare all original marked Unicode.
    # The outer x=175 Arabic hit selects 16/17 graphemes, which would
    # remove *every RTL glyph* during replacement and legitimately disable
    # the RTL-only QA report. Both scripts' verified x=220 midpoint hits
    # leave marked RTL text in the source during the edit assertion.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool keydown Shift_L
    xdotool mousemove --window "$window" 220 521 click 1
    sleep 0.3
    xdotool keyup Shift_L
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-shift-before-edit.json"
    import -window "$window" "$tmp/rtl-$name-preview-shift-before-edit.png"
    xdotool type --clearmodifiers --delay 35 z
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-shift-after-edit.json"
    import -window "$window" "$tmp/rtl-$name-preview-shift-after-edit.png"
    xdotool key --clearmodifiers ctrl+z
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-shift-after-undo.json"
    lua5.4 - "$tmp/$name/preview-shift-before-edit.json" \
      "$tmp/$name/preview-shift-after-edit.json" \
      "$tmp/$name/preview-shift-after-undo.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native Shift-click edit report")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local first,edited,restored=read(arg[1]),read(arg[2]),read(arg[3])
print(string.format("Native Shift-click edit audit: original=%d selected=%d after=%d selected=%d undo=%d selected=%d",
  first.total,first.selected_graphemes,edited.total,
  edited.selected_graphemes,restored.total,restored.selected_graphemes))
assert(first.selected_graphemes>=2
  and edited.total==first.total-first.selected_graphemes+1
  and edited.selected_graphemes==0,
  "Native Shift-click did not select an editable Unicode range")
assert(restored.total==first.total and restored.selected_graphemes==0,
  "Native Undo did not restore Shift-click selected source length")
print("Native RTL Shift-click selection replacement and Undo counts PASS")
LUA
    xdotool key --clearmodifiers ctrl+a
    xdotool key --clearmodifiers ctrl+c
    sleep 0.35
    timeout 5s xclip -selection clipboard -o > "$tmp/$name/preview-shift-restored.txt"
    cmp -s "$tmp/$name.txt" "$tmp/$name/preview-shift-restored.txt" || {
      echo "Native Shift-click selection Undo changed the original Unicode bytes" >&2
      exit 1
    }
    echo "Native RTL shaped-preview Shift-click editing byte-exact Undo PASS"
    # Native XTest double-click on a shaped Arabic/Hebrew word. COSMIC's
    # visual hit must select one whole lexical word in source TextEdit,
    # without absorbing adjacent spaces/punctuation or splitting marks.
    # The word hit position is a previously verified glyph-region point.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.25
    xdotool mousemove --window "$window" 220 521
    # Direct XTest events can be coalesced into a single egui frame when
    # --repeat is used. Separate the clicks by real GUI frames but stay
    # inside egui's double-click interval (<300 ms).
    xdotool click 1
    sleep 0.16
    xdotool click 1
    sleep 0.5
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-double-word.json"
    import -window "$window" "$tmp/rtl-$name-preview-double-word.png"
    lua5.4 - "$tmp/$name/preview-double-word.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Missing native word double-click audit")
local d=assert(json.decode(assert(f:read("*a"))));f:close()
print("Native shaped RTL word-click audit: "..tostring(d.selected_graphemes)..
  "/"..tostring(d.total).." selected graphemes")
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned
  and d.total>8 and d.selected_graphemes>=2
  and d.selected_graphemes<d.total,
  "Double-click on a shaped script glyph must select one source word")
print("Native shaped RTL word double-click PASS: "..
  d.selected_graphemes.." selected graphemes")
LUA
    xdotool type --clearmodifiers --delay 35 z
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-double-replaced.json"
    xdotool key --clearmodifiers ctrl+z
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-double-undone.json"
    lua5.4 - "$tmp/$name/preview-double-word.json" \
      "$tmp/$name/preview-double-replaced.json" \
      "$tmp/$name/preview-double-undone.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing double-click edit report")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local selected,changed,restored=read(arg[1]),read(arg[2]),read(arg[3])
assert(changed.total==selected.total-selected.selected_graphemes+1
  and changed.selected_graphemes==0,
  "Double-click source word replacement must insert one grapheme")
assert(restored.total==selected.total and restored.selected_graphemes==0,
  "Undo must restore original source grapheme count")
print("Native RTL shaped word double-click replacement/Undo counts PASS")
LUA
    xdotool key --clearmodifiers ctrl+a
    xdotool key --clearmodifiers ctrl+c
    sleep 0.3
    timeout 5s xclip -selection clipboard -o > "$tmp/$name/preview-double-restored.txt"
    cmp -s "$tmp/$name.txt" "$tmp/$name/preview-double-restored.txt" || {
      echo "Double-click edit/Undo changed original marked RTL source bytes" >&2
      exit 1
    }
    echo "Native RTL word double-click byte-exact Undo PASS"
    # A preview gesture may end just outside its bitmap. The renderer must
    # clamp the active visual endpoint, retain an actual grapheme-safe editor
    # selection, and never guess or mutate source text.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    # Start inside the connected script glyphs, not the Latin digit edge.
    # A drag from the digit edge to the far right legitimately selects the
    # entire Hebrew source, which is not this partial-range acceptance case.
    xdotool mousemove --window "$window" 220 521 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 355 521
    sleep 0.25
    xdotool mouseup 1
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/preview-edge.json"
    import -window "$window" "$tmp/rtl-$name-preview-edge.png"
    lua5.4 - "$tmp/$name/preview-edge.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Missing native RTL edge-drag audit")
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned
  and d.total>8 and d.selected_graphemes>=2
  and d.selected_graphemes<d.total,
  "Releasing outside the visual preview must retain a bounded source selection")
print("Native RTL shaped-preview out-of-bounds release PASS: "..
  d.selected_graphemes.." selected graphemes")
LUA
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.25
    # Real source mutation, not only caret painting. Select the first marked
    # script grapheme, Backspace it, then undo. Both the original and the
    # restored passage must have exactly the same grapheme counts; no fixture
    # text or input path enters the QA report or persisted profile.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.15
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-before-delete.json"
    xdotool key --clearmodifiers "shift+$selected_direction"
    sleep 0.25
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-before-backspace.json"
    xdotool key --clearmodifiers BackSpace
    sleep 0.5
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-after-backspace.json"
    import -window "$window" "$tmp/rtl-$name-editor-after-backspace.png"
    xdotool key --clearmodifiers ctrl+z
    sleep 0.5
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/editor-after-undo.json"
    import -window "$window" "$tmp/rtl-$name-editor-after-undo.png"
    lua5.4 - "$tmp/$name/editor-before-delete.json" \
      "$tmp/$name/editor-before-backspace.json" \
      "$tmp/$name/editor-after-backspace.json" \
      "$tmp/$name/editor-after-undo.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing synthetic RTL source editing audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  assert(type(d.total)=="number" and type(d.index)=="number")
  return d
end
local original,selected,deleted,restored=read(arg[1]),read(arg[2]),read(arg[3]),read(arg[4])
assert(original.selected_graphemes==0 and original.index==0,
  "RTL mutation did not start at the logical source boundary")
assert(selected.total==original.total and selected.selected_graphemes==1
  and selected.index==1, "Could not select one marked RTL grapheme")
assert(deleted.total==original.total-1 and deleted.index==0
  and deleted.selected_graphemes==0,
  "Backspace did not delete one entire marked RTL grapheme")
assert(restored.total==original.total and restored.selected_graphemes==0,
  "Undo did not restore the original native marked-grapheme source length")
print("Native marked RTL grapheme editing PASS: "..original.total.." -> "
  ..deleted.total.." -> "..restored.total.." after selection, Backspace and Undo")
LUA
    # Length recovery alone cannot prove preservation of Arabic harakat
    # or Hebrew niqqud. Compare the native editor's Ctrl+C output against
    # the exact disposable UTF-8 source, without logging the copied text.
    xdotool key --clearmodifiers ctrl+a
    xdotool key --clearmodifiers ctrl+c
    sleep 0.35
    timeout 5s xclip -selection clipboard -o > "$tmp/$name/editor-restored.txt"
    cmp -s "$tmp/$name.txt" "$tmp/$name/editor-restored.txt" || {
      echo "Native $name marked-grapheme edit/Undo damaged original Unicode bytes" >&2
      exit 1
    }
    echo "Native $name marked-grapheme Backspace/Undo byte-exact source PASS"
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.25
  fi
  if [[ "$name" == hebrew-long ]];then
    # Actual egui/COSMIC editor viewport of a long, marked RTL passage.
    # A fixed 160-grapheme preview overflows narrow windows; require
    # width-bounded source slices and a positioned caret at both ends.
    focus_window
    xdotool key --clearmodifiers ctrl+e
    sleep 0.4
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/long-editor-start.json"
    import -window "$window" "$tmp/rtl-hebrew-long-editor-start.png"
    xdotool key --clearmodifiers ctrl+End
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/long-editor-end.json"
    import -window "$window" "$tmp/rtl-hebrew-long-editor-end.png"
    lua5.4 - "$tmp/$name/long-editor-start.json" \
      "$tmp/$name/long-editor-end.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native RTL long viewport audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  return d
end
local before,after=read(arg[1]),read(arg[2])
assert(before.total>160 and before.index==0 and before.visible_start==0,
  "Long Hebrew editor did not start at logical source beginning")
assert(after.total==before.total and after.index==after.total
  and after.visible_end==after.total and after.visible_start>0,
  "Long RTL native editor failed to follow logical end caret")
assert(after.visible_end-after.visible_start<=125
  and before.visible_end-before.visible_start<=125,
  "Long RTL editor texture exceeds width-sensitive preview window")
assert(type(after.caret_y_px)=="number" and after.caret_y_px>=0
  and after.caret_y_px<240,
  "Long Hebrew native caret is clipped outside the shaped texture")
print("Native long RTL editor bounded viewport PASS: "..
  before.visible_start.."–"..before.visible_end.." -> "..
  after.visible_start.."–"..after.visible_end..
  " of "..after.total.." graphemes")
LUA
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.25
  fi
  if [[ "$name" == mixed ]];then
    # Three actual script directions over three logical paragraphs. This
    # checks native editor caret positioning against the shaped preview,
    # not only Rust-level shaping or a nonblank screenshot.
    focus_window
    xdotool key --clearmodifiers ctrl+e
    sleep 0.35
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-caret-start.json"
    import -window "$window" "$tmp/rtl-mixed-editor-start.png"
    xdotool key --clearmodifiers ctrl+End
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-caret-end.json"
    import -window "$window" "$tmp/rtl-mixed-editor-end.png"
    lua5.4 - "$tmp/$name/mixed-caret-start.json" "$tmp/$name/mixed-caret-end.json" <<'LUA'
local json=require("dkjson")
local function read(file)
  local f=assert(io.open(file,"rb"),"Missing mixed BiDi caret audit")
  local r=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(r.synthetic and r.state=="rtl-editor-caret" and r.caret_positioned)
  assert(type(r.caret_y_px)=="number" and r.caret_y_px>=0)
  return r
end
local first,last=read(assert(arg[1])),read(assert(arg[2]))
assert(first.index==0 and last.index==last.total and first.total==last.total,
  "Mixed BiDi native editor start/end did not reach logical boundaries")
assert(last.caret_y_px>first.caret_y_px+15,
  "Mixed RTL/LTR source end caret did not move to a later shaped paragraph")
assert(first.selected_graphemes==0 and last.selected_graphemes==0,
  "Caret movement must not create a selection")
print("Native mixed BiDi editor caret PASS: logical start/end; visual row moved "
  ..first.caret_y_px.." -> "..last.caret_y_px.." px")
LUA
    # Expand a keyboard selection through the Arabic-containing first
    # paragraph, the Latin paragraph and toward the Hebrew/Arabic line.
    # Test actual native TextEdit state and independent shaped-preview paint.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool key --clearmodifiers shift+Down
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-selection-first.json"
    import -window "$window" "$tmp/rtl-mixed-selection-first.png"
    xdotool key --clearmodifiers shift+Down
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-selection-second.json"
    import -window "$window" "$tmp/rtl-mixed-selection-second.png"
    xdotool key --clearmodifiers shift+Up
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-selection-reverse.json"
    import -window "$window" "$tmp/rtl-mixed-selection-reverse.png"
    lua5.4 - "$tmp/$name/mixed-caret-start.json" \
      "$tmp/$name/mixed-selection-first.json" \
      "$tmp/$name/mixed-selection-second.json" \
      "$tmp/$name/mixed-selection-reverse.json" <<'LUA'
local json=require("dkjson")
local function read(file)
  local f=assert(io.open(file,"rb"),"Missing native BiDi selection audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  assert(type(d.selected_graphemes)=="number" and type(d.caret_y_px)=="number")
  return d
end
local start,first,second,reversed=read(arg[1]),read(arg[2]),read(arg[3]),read(arg[4])
assert(first.total==start.total and second.total==start.total
  and reversed.total==start.total,"Mixed BiDi source changed during selection")
assert(first.selected_graphemes>5 and first.selected_graphemes<start.total-5,
  "Shift+Down failed to form a partial Arabic/Latin range")
assert(second.selected_graphemes>first.selected_graphemes
  and second.selected_graphemes<start.total,
  "Second Shift+Down failed to expand selection into next mixed run")
assert(reversed.selected_graphemes<second.selected_graphemes,
  "Reverse Shift+Up failed to shrink the mixed-direction selection")
assert(first.caret_y_px>start.caret_y_px
  and second.caret_y_px>first.caret_y_px,
  "Shaped caret did not cross both mixed paragraphs")
print("Native mixed-script selection PASS: "..first.selected_graphemes.." -> "
  ..second.selected_graphemes.." -> "..reversed.selected_graphemes..
  " graphemes, caret rows "..start.caret_y_px.." -> "
  ..first.caret_y_px.." -> "..second.caret_y_px)
LUA
    convert "$tmp/rtl-mixed-editor-start.png" \
      -crop 285x94+12+487 +repage "$tmp/$name/mixed-preview-base.png"
    convert "$tmp/rtl-mixed-selection-second.png" \
      -crop 285x94+12+487 +repage "$tmp/$name/mixed-preview-selection.png"
    mixed_pixels="$(compare -metric AE \
      "$tmp/$name/mixed-preview-base.png" \
      "$tmp/$name/mixed-preview-selection.png" null: 2>&1 || true)"
    [[ "$mixed_pixels" =~ ^[0-9]+$ && "$mixed_pixels" -gt 80 ]] || {
      echo "Native mixed selection not visibly represented in shaped preview: $mixed_pixels" >&2
      exit 1
    }
    # Hit-test the *actual* multiline source control with two native mouse
    # clicks on the Latin and mixed Hebrew/Arabic paragraphs. Coordinates
    # refer to the fixed 1060x760 disposable CI window and its text editor,
    # never a production user desktop or a global input capture surface.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool mousemove --window "$window" 28 419 click 1
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-mouse-latin.json"
    import -window "$window" "$tmp/rtl-mixed-mouse-latin.png"
    xdotool mousemove --window "$window" 28 437 click 1
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-mouse-hebrew.json"
    import -window "$window" "$tmp/rtl-mixed-mouse-hebrew.png"
    lua5.4 - "$tmp/$name/mixed-mouse-latin.json" \
      "$tmp/$name/mixed-mouse-hebrew.json" <<'LUA'
local json=require("dkjson")
local function read(file)
  local f=assert(io.open(file,"rb"),"Missing native mixed-script mouse audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  assert(type(d.index)=="number" and type(d.caret_y_px)=="number")
  assert(d.selected_graphemes==0,"Mouse click must not leave keyboard selection")
  return d
end
local latin,hebrew=read(arg[1]),read(arg[2])
assert(latin.total==hebrew.total and latin.total==54,
  "Native source changed while mouse-hit-testing RTL/LTR paragraphs")
assert(latin.index>=22 and latin.index<37 and latin.caret_y_px>=25
  and latin.caret_y_px<55,
  "Latin paragraph mouse click failed native logical caret hit-testing")
assert(hebrew.index>=37 and hebrew.index<=54
  and hebrew.caret_y_px>=55,
  "Hebrew/Arabic paragraph mouse click failed native logical caret hit-testing")
assert(hebrew.index>latin.index and hebrew.caret_y_px>latin.caret_y_px,
  "Mixed-script mouse clicks did not move the shaped caret to the next paragraph")
print("Native mixed BiDi mouse caret PASS: "..latin.index.." -> "..hebrew.index..
  " graphemes, visual rows "..latin.caret_y_px.." -> "..hebrew.caret_y_px)
LUA
    # Test real pointer *drag* ranges across different BiDi paragraph
    # directions. Click-only caret motion is already covered above;
    # mousedown/move/mouseup must preserve the logical range and move
    # the independent shaped selection/caret.
    xdotool mousemove --window "$window" 28 419 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 28 437
    sleep 0.2
    xdotool mouseup 1
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-drag-down.json"
    import -window "$window" "$tmp/rtl-mixed-drag-down.png"
    xdotool mousemove --window "$window" 28 437 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 28 419
    sleep 0.2
    xdotool mouseup 1
    sleep 0.35
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-drag-up.json"
    import -window "$window" "$tmp/rtl-mixed-drag-up.png"
    lua5.4 - "$tmp/$name/mixed-drag-down.json" "$tmp/$name/mixed-drag-up.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native BiDi drag audit")
  local r=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(r.synthetic and r.state=="rtl-editor-caret" and r.caret_positioned)
  assert(r.total==54 and type(r.selected_graphemes)=="number")
  return r
end
local down,up=read(arg[1]),read(arg[2])
assert(down.index>=37 and down.index<=54 and down.caret_y_px>=55
  and down.selected_graphemes>=8 and down.selected_graphemes<54,
  "Native Latin-to-Hebrew/Arabic pointer drag failed")
assert(up.index>=22 and up.index<37 and up.caret_y_px>=25 and up.caret_y_px<55
  and up.selected_graphemes>=8 and up.selected_graphemes<54,
  "Native Hebrew/Arabic-to-Latin reverse pointer drag failed")
print("Native mixed BiDi drag selection PASS: "..down.selected_graphemes.." / "..
  up.selected_graphemes.." graphemes, visual rows "..down.caret_y_px.." / "..up.caret_y_px)
LUA
    # Cross visual rows in the shaped, noneditable preview itself.
    # The fixture has mixed RTL/Latin first line at y≈515 and a Latin
    # second line at y≈545. COSMIC must hand this real drag to the
    # native source editor at correct logical newline-aware offsets.
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
    xdotool mousemove --window "$window" 220 516 mousedown 1
    sleep 0.15
    xdotool mousemove --window "$window" 80 548
    sleep 0.25
    xdotool mouseup 1
    sleep 0.45
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-preview-cross-row.json"
    import -window "$window" "$tmp/rtl-mixed-preview-cross-row.png"
    lua5.4 - "$tmp/$name/mixed-preview-cross-row.json" <<'LUA'
local json=require("dkjson")
local f=assert(io.open(arg[1],"rb"),"Missing native cross-row preview audit")
local d=assert(json.decode(assert(f:read("*a"))));f:close()
assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned
  and d.total==54 and d.selected_graphemes>=8
  and d.selected_graphemes<d.total and d.index>=22 and d.index<=54
  and d.caret_y_px>20,
  "Shaped preview cross-row RTL/Latin drag failed to select in the source editor")
print("Native mixed BiDi shaped-preview cross-row drag PASS: "..
  d.selected_graphemes.." graphemes, active caret "..d.index)
LUA
    convert "$tmp/rtl-mixed-editor-start.png" -crop 285x94+12+487 +repage \
      "$tmp/$name/mixed-preview-cross-row-base.png"
    convert "$tmp/rtl-mixed-preview-cross-row.png" -crop 285x94+12+487 +repage \
      "$tmp/$name/mixed-preview-cross-row-highlight.png"
    cross_row_pixels="$(compare -metric AE "$tmp/$name/mixed-preview-cross-row-base.png" \
      "$tmp/$name/mixed-preview-cross-row-highlight.png" null: 2>&1 || true)"
    [[ "$cross_row_pixels" =~ ^[0-9]+$ && "$cross_row_pixels" -gt 80 ]] || {
      echo "Shaped mixed cross-row preview failed to visibly highlight selection" >&2
      exit 1
    }
    # Replace an actual pointer-selected range spanning Latin and RTL
    # paragraphs, then undo. The earlier test only inspected selection
    # counts; it could pass even if an edit silently corrupted equal-length
    # graphemes. Exercise the real egui editor and verify exact recovery.
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-before-replace.json"
    xdotool type --clearmodifiers --delay 35 "Z"
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-after-replace.json"
    import -window "$window" "$tmp/rtl-mixed-editor-replaced.png"
    xdotool key --clearmodifiers ctrl+z
    sleep 0.4
    cp "$TYPEFINCH_QA_EDITOR_CARET_REPORT" "$tmp/$name/mixed-after-undo.json"
    import -window "$window" "$tmp/rtl-mixed-editor-undo.png"
    lua5.4 - "$tmp/$name/mixed-before-replace.json" \
      "$tmp/$name/mixed-after-replace.json" \
      "$tmp/$name/mixed-after-undo.json" <<'LUA'
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"),"Missing native mixed-direction editing audit")
  local d=assert(json.decode(assert(f:read("*a"))));f:close()
  assert(d.synthetic and d.state=="rtl-editor-caret" and d.caret_positioned)
  assert(type(d.total)=="number" and type(d.selected_graphemes)=="number")
  return d
end
local selected,changed,undone=read(arg[1]),read(arg[2]),read(arg[3])
assert(selected.total==54 and selected.selected_graphemes>=8,
  "BiDi replacement did not begin with a real multi-grapheme drag selection")
assert(changed.total==selected.total-selected.selected_graphemes+1
  and changed.selected_graphemes==0,
  "Replacing a mixed-direction range did not insert one Latin grapheme")
assert(undone.total==selected.total and undone.selected_graphemes==0,
  "Undo did not restore the mixed-direction source length")
print("Native mixed-script pointer selection, replacement and Undo counts PASS")
LUA
    # The fixture is entirely synthetic and lives in an isolated X11 CI
    # session. Clipboard comparison is performed only here, with no source
    # bytes printed or published. A length-only check cannot detect a
    # changed Arabic/Hebrew combining mark or equal-length substitution.
    xdotool key --clearmodifiers ctrl+a
    xdotool key --clearmodifiers ctrl+c
    sleep 0.35
    timeout 5s xclip -selection clipboard -o > "$tmp/$name/mixed-restored.txt"
    cmp -s "$tmp/mixed.txt" "$tmp/$name/mixed-restored.txt" || {
      echo "Native mixed BiDi edit/Undo changed the original Unicode source" >&2
      exit 1
    }
    echo "Native mixed BiDi replacement/Undo byte-exact recovery PASS"
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.2
  fi
  focus_window
  import -window "$window" "$tmp/rtl-$name.png"
  [[ -s "$tmp/rtl-$name.png" ]] || { echo "Missing $name screenshot" >&2;exit 1; }
  delta="$(compare -metric AE "$tmp/rtl-$name-baseline.png" "$tmp/rtl-$name.png" null: 2>&1 || true)"
  [[ "$delta" =~ ^[0-9]+$ && "$delta" -gt 1500 ]] || {
    echo "Loaded RTL $name did not change the frame: $delta" >&2;exit 1;
  }
  focus_window
  xdotool key --clearmodifiers F5
  sleep 0.3
  xdotool key --clearmodifiers F2
  sleep 0.25
  import -window "$window" "$tmp/rtl-$name-running.png"
  [[ -s "$tmp/rtl-$name-running.png" ]] || {
    echo "Missing RTL active caret screenshot for $name" >&2;exit 1;
  }
  active="$(compare -metric AE "$tmp/rtl-$name.png" "$tmp/rtl-$name-running.png" null: 2>&1 || true)"
  [[ "$active" =~ ^[0-9]+$ && "$active" -gt 100 ]] || {
    echo "Active RTL frame unchanged for $name: $active" >&2;exit 1;
  }
  # An ASCII x should mismatch the first *logical* reference grapheme; it
  # makes a real in-app highlight without claiming RTL physical key input.
  xdotool type --clearmodifiers --delay 35 "x"
  sleep 0.2
  import -window "$window" "$tmp/rtl-$name-error.png"
  [[ -s "$tmp/rtl-$name-error.png" ]] || {
    echo "Missing RTL error screenshot for $name" >&2;exit 1;
  }
  changed="$(compare -metric AE "$tmp/rtl-$name-running.png" "$tmp/rtl-$name-error.png" null: 2>&1 || true)"
  [[ "$changed" =~ ^[0-9]+$ && "$changed" -gt 100 ]] || {
    echo "Reference error produced no visible difference for $name: $changed" >&2;exit 1;
  }
  xdotool key --clearmodifiers Escape
  stop_app
  echo "Native $name isolated reference, caret and error snapshots PASS ($delta / $active / $changed pixels)"
done
different="$(compare -metric AE "$tmp/rtl-arabic.png" "$tmp/rtl-hebrew.png" null: 2>&1 || true)"
[[ "$different" =~ ^[0-9]+$ && "$different" -gt 400 ]] || {
  echo "Distinct imported RTL scripts rendered identical frames: $different" >&2;exit 1;
}
echo 'Independent Arabic/Hebrew/mixed-direction native X11 fixtures (including long RTL wrapping) PASS; real IME/Hyprland QA pending'
