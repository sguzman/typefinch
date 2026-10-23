#!/usr/bin/env lua5.4
-- Assertions for Typefinch's real native X11 smoke workflow.
-- Uses Lua and the SQLite CLI; does not collect typed text or private settings.
local json = require("dkjson")
local mode = assert(arg[1], "missing assertion mode")
local function read(path)
    local file, err = io.open(path, "rb")
    assert(file, err)
    local content = assert(file:read("*a"))
    file:close()
    return content
end
local function save(path, content)
    local file, err = io.open(path, "wb")
    assert(file, err)
    assert(file:write(content))
    assert(file:close())
end
local function quoted(s) return "'" .. s:gsub("'", "'\\''") .. "'" end
local function dbquery(sql)
    local root = assert(os.getenv("XDG_CONFIG_HOME"))
    local path = root .. "/typefinch/sessions.sqlite3"
    local cmd = "sqlite3 -json " .. quoted(path) .. " " .. quoted(sql)
    local pipe = assert(io.popen(cmd, "r"))
    local raw = assert(pipe:read("*a"))
    local ok, kind, code = pipe:close()
    assert(ok and (not code or code == 0), "sqlite3 failed: " .. cmd)
    return assert(json.decode(raw))
end
local function report(path)
    return assert(json.decode(read(path)))
end
local function equals(a,b,desc)
    assert(a == b, (desc or "value") .. ": " .. tostring(a) .. " ~= " .. tostring(b))
end
if mode == "first" then
    local r = report(assert(arg[2]))
    assert(r.synthetic and r.state == "finished")
    equals(r.tests, 1, "first test count")
    assert(r.entered >= 8 and r.correct >= 5 and r.wpm > 0)
    assert(r.elapsed_ms >= 1500 and r.elapsed_ms <= 2500)
    local rows = dbquery("SELECT entered,correct,scoring_version,test_mode FROM sessions")
    equals(#rows,1)
    assert(rows[1].entered>=8 and rows[1].scoring_version==1 and rows[1].test_mode=="words")
    print(string.format("X11 GUI smoke PASS: %d entered, %d correct, %.1f WPM",r.entered,r.correct,r.wpm))
elseif mode == "paste" then
    local r = report(assert(arg[2]))
    equals(r.tests,2)
    equals(r.entered,#"little finches")
    equals(r.correct,#"little finches")
    local rows=dbquery("SELECT entered,correct FROM sessions ORDER BY id")
    equals(#rows,2)
    equals(rows[2].entered,#"little finches")
    equals(rows[2].correct,#"little finches")
    print("Clipboard paste suppression PASS: pasted text was not scored")
elseif mode == "correction" then
    local r=report(assert(arg[2]))
    assert(r.state=="finished")
    equals(r.tests,3)
    local version=dbquery("PRAGMA user_version")
    equals(version[1].user_version,3)
    local rows=dbquery("SELECT entered,correct,attempts,mistakes,corrections,accuracy,raw_accuracy,wpm,raw_wpm FROM sessions ORDER BY id")
    equals(#rows,3)
    local x=rows[3]
    equals(x.entered,#"little finches")
    equals(x.correct,x.entered)
    equals(x.attempts,x.entered+1)
    equals(x.mistakes,1)
    equals(x.corrections,1)
    equals(x.accuracy,100)
    assert(x.raw_accuracy>0 and x.raw_accuracy<100 and x.raw_wpm>x.wpm)
    print("Corrected-error QA PASS: raw mistakes survive corrections and SQLite v3")
elseif mode == "export" then
    local svg=read(assert(arg[2]))
    local md=read(assert(arg[3]))
    assert(svg:find("<svg",1,true) and svg:find("</svg>",1,true))
    assert(not svg:find("<script",1,true))
    assert(svg:find("Finch",1,true))
    equals((md:gsub("%s+$","")),"![Typefinch typing profile](./typefinch-profile.svg)")
    print("SVG/Markdown export PASS: static local card and relative link")
elseif mode == "stats" then
    local data=assert(json.decode(read(assert(arg[2]))))
    equals(data.version,1)
    equals(data.format,"typefinch-session-history")
    equals(data.order,"newest-first")
    equals(#data.sessions,3)
    for _, item in ipairs(data.sessions) do
        assert(item.profile_id==data.profile_id)
        assert(type(item.scoring_version)=="number")
        assert(item.typed==nil and item.raw_text==nil)
    end
    print("Full JSON history export PASS: three records, versioned aggregate-only payload")
elseif mode == "set-theme" then
    local path, theme=assert(arg[2]),assert(arg[3])
    assert(theme=="light" or theme=="dark")
    local config=assert(json.decode(read(path)))
    config.theme=theme
    save(path,json.encode(config))
elseif mode == "theme" then
    local light,dark=tonumber(arg[2]),tonumber(arg[3])
    assert(light and dark and light>dark+0.25)
    print(string.format("Theme render PASS: light luminance=%.3f, dark=%.3f",light,dark))
elseif mode == "prepare-lesson" then
    local path=assert(arg[2])
    local config=assert(json.decode(read(path)))
    config.test_mode="lesson"
    config.lesson_id="home"
    config.theme="dark"
    save(path,json.encode(config))
elseif mode == "prepare-spanish" then
    local path=assert(arg[2])
    local config=assert(json.decode(read(path)))
    config.test_mode="words"
    config.language="es-MX"
    config.word_difficulty="easy"
    config.theme="dark"
    save(path,json.encode(config))
elseif mode == "spanish" then
    local r=report(assert(arg[2]))
    assert(r.state=="finished")
    equals(r.tests,5)
    equals(r.entered,#"casa mesa gato perro")
    equals(r.correct,r.entered)
    local rows=dbquery("SELECT test_mode,duration_ms,entered,correct FROM sessions ORDER BY id")
    equals(#rows,5)
    equals(rows[1].test_mode,"words")
    local latest=rows[5]
    equals(latest.test_mode,"words/es-MX/easy")
    equals(latest.duration_ms,2000)
    equals(latest.entered,latest.correct)
    print("Multilingual native typing PASS: Spanish easy corpus and isolated SQLite history")
elseif mode == "lesson" then
    local r=report(assert(arg[2]))
    assert(r.state=="finished")
    equals(r.tests,4)
    local rows=dbquery("SELECT test_mode,duration_ms,elapsed_ms,entered,correct FROM sessions ORDER BY id DESC LIMIT 1")
    equals(#rows,1)
    local item=rows[1]
    equals(item.test_mode,"lesson/home")
    equals(item.duration_ms,0)
    assert(item.elapsed_ms>0)
    equals(item.entered,#"asdf jkl; asdf jkl; fj dk sl a; sad flask; jill asks; asdf jkl;")
    equals(item.entered,item.correct)
    print("Untimed practice PASS: last-character completion and distinct SQLite label")
else
    error("Unknown smoke assertion: " .. mode)
end
