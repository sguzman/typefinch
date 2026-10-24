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
    # Select the *first Arabic/Hebrew logical grapheme* (including any
    # diacritics), not the final ASCII digit in the mixed-script fixture.
    # The visual arrow direction at an RTL boundary can differ by platform.
    partial_status=''
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
      [[ "$partial_status" != "1:1" ]] || break
    done
    [[ "$partial_status" == "1:1" ]] || {
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
    xdotool key --clearmodifiers ctrl+Home
    sleep 0.25
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
