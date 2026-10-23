#!/usr/bin/env lua5.4
-- Reproducible brand-color WCAG contrast audit. No browser or external data.
-- Scope: Typefinch project swatch and explicitly defined UI accent inks.
local function read(path)
  local f=assert(io.open(path,"rb"))
  local bytes=assert(f:read("*a"))
  assert(f:close())
  return bytes
end
local function linear(byte)
  local c=byte/255
  if c<=0.04045 then return c/12.92 end
  return ((c+0.055)/1.055)^2.4
end
local function luminance(hex)
  assert(hex:match("^#[0-9a-fA-F]+$") and #hex==7,"invalid sRGB token: "..hex)
  local r=linear(tonumber(hex:sub(2,3),16))
  local g=linear(tonumber(hex:sub(4,5),16))
  local b=linear(tonumber(hex:sub(6,7),16))
  return 0.2126*r+0.7152*g+0.0722*b
end
local function contrast(fg,bg)
  local a,b=luminance(fg),luminance(bg)
  return (math.max(a,b)+0.05)/(math.min(a,b)+0.05)
end
local app=read("src/app.rs")
local svg=read("assets/typefinch-color.svg")
local docs=read("docs/COLOR.md")
assert(app:find("Color32::from_rgb(134, 150, 245)",1,true),"brand app RGB drift")
assert(app:find("Color32::from_rgb(69,89,181)",1,true),"light UI ink drift")
assert(app:find("Color32::from_rgb(170,181,255)",1,true),"dark UI ink drift")
assert(docs:find("#8696F5",1,true) and docs:find("oklch(70.123%",1,true),
  "perceptual token documentation missing")
assert(svg:find('aria-label="Typefinch soft periwinkle project swatch #8696F5"',1,true),
  "swatch must include an accessible image label")
local swatch=svg:match('<rect x="1" y="1"[^>]*fill="(#[%x]+)"')
local title_ink=svg:match('<text x="112"[^>]*fill="(#[%x]+)"')
local caption_ink=svg:match('<text x="114"[^>]*fill="(#[%x]+)"')
assert(swatch=="#8696F5" and title_ink=="#202645" and caption_ink=="#202645",
  "swatch text/background colors changed without audit")
local pairs={
  {"swatch label", "#202645","#8696F5"},
  {"UI ink light", "#4559B5","#FFFFFF"},
  {"UI ink dark", "#AAB5FF","#202645"},
  {"selection light", "#FFFFFF","#4559B5"},
  {"selection dark", "#202645","#8696F5"},
}
for _,p in ipairs(pairs) do
  local ratio=contrast(p[2],p[3])
  assert(ratio>=4.5,p[1].." must meet WCAG AA normal-text contrast")
  print(string.format("%s: %.2f:1 (AA text)",p[1],ratio))
end
assert(contrast("#FFFFFF","#8696F5")<4.5,
  "the contrast warning for white text on the brand swatch needs reassessment")
print("Brand contrast audit PASS; covers project swatch and semantic accent pairs, not a full keyboard-only or Unicode-font audit")
