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
    assert(rows[1].entered>=8 and rows[1].scoring_version==2 and rows[1].test_mode=="words")
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
elseif mode == "orphan" then
    local r=report(assert(arg[2]))
    equals(r.state,"blocked")
    assert(r.synthetic)
    print("Orphaned-history GUI guard PASS: session rejected without replacement settings")
elseif mode == "backup" then
    local database=assert(arg[2])
    local settings=assert(json.decode(read(assert(arg[3]))))
    assert(settings.profiles and #settings.profiles>=1)
    assert(settings.language=="es-MX")
    local query="sqlite3 -json "..quoted(database).." "..quoted("SELECT COUNT(*) AS n FROM sessions")
    local pipe=assert(io.popen(query,"r"))
    local rows=assert(json.decode(assert(pipe:read("*a"))))
    local ok=pipe:close()
    assert(ok,"SQLite backup must open cleanly")
    equals(rows[1].n,5)
    local modequery="sqlite3 -json "..quoted(database).." "..quoted("PRAGMA user_version")
    local p=assert(io.popen(modequery,"r"))
    local versions=assert(json.decode(assert(p:read("*a"))))
    assert(p:close())
    equals(versions[1].user_version,3)
    print("Private backup PASS: paired JSON settings and complete consistent SQLite snapshot")
elseif mode == "prepare-personal-keymap" then
    local path=assert(arg[2])
    local config=assert(json.decode(read(path)))
    local id="custom/00000000-0000-4000-8000-000000000001"
    config.test_mode="words"
    config.language="en"
    config.word_difficulty="easy"
    config.layout=id
    config.custom_layouts={{
        version=1,id=id,name="X11 synthetic ANSI",
        base={"qwertyuiop","asdfghjkl;","zxcvbnm,./"},
        shifted={"QWERTYUIOP","ASDFGHJKL:","ZXCVBNM<>?"}
    }}
    save(path,json.encode(config))
elseif mode == "personal-keymap" then
    local r=report(assert(arg[2]))
    assert(r.state=="finished" and r.tests==6,r)
    assert(r.correct>0 and r.entered>=10,r)
    local rows=dbquery("SELECT layout,input_mode,test_mode,entered,correct FROM sessions ORDER BY id DESC LIMIT 1")
    equals(#rows,1)
    equals(rows[1].layout,"custom/00000000-0000-4000-8000-000000000001")
    equals(rows[1].input_mode,"Emulated")
    equals(rows[1].test_mode,"words/easy")
    assert(rows[1].correct>=10,rows[1].correct)
    print("Personal ANSI native QA PASS: physical events scored through a persisted custom keymap")
elseif mode == "prepare-card-theme" then
    local path=assert(arg[2])
    local config=assert(json.decode(read(path)))
    config.card_theme="daybreak"
    config.card_font="mono"
    config.card_template="compact"
    save(path,json.encode(config))
elseif mode == "png" then
    local data=read(assert(arg[2]))
    assert(data:sub(1,8)==string.char(137,80,78,71,13,10,26,10))
    local function big32(position)
        local a,b,c,d=data:byte(position,position+3)
        return a*16777216+b*65536+c*256+d
    end
    equals(big32(17),550)
    equals(big32(21),145)
    assert(#data>512)
    -- A valid PNG can still be missing all text when fontdb resolves no faces.
    -- Crop the plain-background heading area, excluding the decorative bird.
    local cmd="convert "..quoted(assert(arg[2]))..
        " -crop 120x28+20+10 +repage -colorspace Gray -format "..
        quoted("%[fx:standard_deviation]").." info:"
    local pipe=assert(io.popen(cmd,"r"))
    local deviation=tonumber(assert(pipe:read("*a")))
    assert(pipe:close())
    assert(deviation and deviation>0.01,
        "PNG text is missing or unreadable: heading pixels are uniform")
    print(string.format("Native PNG card export PASS: compact 550x145, palette and rendered glyphs (contrast spread %.3f)",deviation))
elseif mode == "card-theme" then
    local data=read(assert(arg[2]))
    assert(data:find('fill="#F5F6FF"',1,true),"Daybreak background missing")
    assert(data:find('font-family="monospace"',1,true),"Monospaced typography missing")
    assert(data:find('height="145"',1,true),"Compact layout size missing")
    assert(not data:find("<script",1,true),"Unsafe SVG script")
    print("Profile card studio PASS: Daybreak + mono + compact template reflected in native SVG")
elseif mode == "recovery" then
    local current=read(assert(arg[2]))
    local original=read(assert(arg[3]))
    equals(current,original,"Invalid settings preserved byte-for-byte")
    local state=report(assert(arg[5]))
    equals(state.state,"blocked","Actual session must hit the read-only persistence guard")
    assert(state.synthetic==true)
    assert(state.entered==nil and state.wpm==nil,"Read-only QA must never claim a score")
    local path=assert(arg[4])
    local f=io.open(path,"rb")
    if f then
        f:close()
        local query="sqlite3 -json "..quoted(path).." "..quoted("SELECT COUNT(*) AS n FROM sessions")
        local pipe=assert(io.popen(query,"r"))
        local records=assert(json.decode(assert(pipe:read("*a"))))
        assert(pipe:close())
        equals(records[1].n,0,"Guarded session must not create any saved scores")
    end
    print("Recovery safety PASS: invalid settings preserved, no scored session or synthetic success report")
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
