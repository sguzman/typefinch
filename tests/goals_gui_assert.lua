#!/usr/bin/env lua5.4
-- Synthetic-only goal JSON QA; never logs or copies a user's personal data.
local json=require("dkjson")
local function read(path)
  local f=assert(io.open(path,"rb"))
  local bytes=assert(f:read("*a")); f:close(); return bytes
end
local mode,path,fixture=assert(arg[1]),assert(arg[2]),assert(arg[3])
local data=assert(json.decode(read(path)))
local expected=assert(json.decode(read(fixture)))
local goals
if mode=="imported" then
  assert(type(data.profiles)=="table" and #data.profiles==1)
  local profile=data.profiles[1]
  assert(type(profile.id)=="string" and #profile.id>0)
  assert(profile.tests==0 and profile.best_wpm==0 and profile.best_accuracy==0)
  goals=profile.goals
  local raw=read(path)
  assert(not raw:find(fixture,1,true),"import source path must not persist")
  assert(not raw:find("typefinch-profile-goals",1,true),"transfer wrapper must not persist")
elseif mode=="exported" then
  assert(data.format=="typefinch-profile-goals" and data.version==1)
  goals=data.goals
else error("unknown goals QA mode") end
assert(goals.target_wpm==expected.goals.target_wpm)
assert(goals.target_accuracy==expected.goals.target_accuracy)
assert(goals.daily_sessions==expected.goals.daily_sessions)
assert(goals.show_streak==expected.goals.show_streak)
print("Native goal "..mode.." PASS: targets accurate and profile identity untouched")
