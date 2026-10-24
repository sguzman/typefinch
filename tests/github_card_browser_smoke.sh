#!/usr/bin/env bash
# Verify the public, synthetic SVG is served unchanged from GitHub and
# decoded as an actual image by a headless browser. No user stats or tokens.
set -Eeuo pipefail
sha="${GITHUB_SHA:-}"
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || {
  echo "A concrete GitHub commit SHA is required" >&2; exit 1;
}
local_svg="assets/typefinch-card-demo.svg"
[[ -s "$local_svg" ]] || { echo "Missing public sample card" >&2; exit 1; }
grep -Fq '![Typefinch synthetic example profile card](assets/typefinch-card-demo.svg)' README.md
xmllint --nonet --noout "$local_svg"
url="https://raw.githubusercontent.com/sguzman/typefinch/$sha/$local_svg"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl --fail --location --silent --show-error --retry 2 --max-time 40 "$url" -o "$tmp/public.svg"
cmp "$local_svg" "$tmp/public.svg" || {
  echo "GitHub public SVG differs from checked-in source" >&2;exit 1;
}
browser=""
for name in google-chrome google-chrome-stable chromium chromium-browser;do
  if command -v "$name" >/dev/null 2>&1;then browser="$(command -v "$name")";break;fi
done
[[ -n "$browser" ]] || { echo "Native Chromium-family browser unavailable" >&2;exit 1; }
cat >"$tmp/fixture.html" <<HTML
<!doctype html><html><body data-card-loaded="pending">
<img id="card" src="$url" alt="Typefinch synthetic example profile card">
<script>
const image=document.getElementById("card");
function check(){
  document.body.dataset.cardLoaded=image.complete&&image.naturalWidth===550&&
    image.naturalHeight===145?"550x145":"invalid";
}
image.addEventListener("load",check);
image.addEventListener("error",()=>{document.body.dataset.cardLoaded="error";});
if(image.complete)check();
</script></body></html>
HTML
flags=(--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage
  --no-first-run --no-default-browser-check --virtual-time-budget=12000)
timeout 65s "$browser" "${flags[@]}" --user-data-dir="$tmp/browser-dom" \
  --dump-dom "file://$tmp/fixture.html" > "$tmp/dom.html" 2>"$tmp/browser.err" || {
  echo 'GitHub card browser DOM check failed' >&2
  tail -n 25 "$tmp/browser.err" >&2 || true
  exit 1
}
grep -Fq 'data-card-loaded="550x145"' "$tmp/dom.html" || {
  echo "GitHub-served sample SVG was not decoded as 550x145 in Chrome" >&2
  grep -o 'data-card-loaded="[^"]*"' "$tmp/dom.html" >&2 || true
  exit 1
}
# A screenshot of a real decoded browser image is archived separately from
# Typefinch's local resvg output. It can be visually reviewed on GitHub.
screenshot="$PWD/typefinch-github-card-browser.png"
timeout 65s "$browser" "${flags[@]}" --user-data-dir="$tmp/browser-screenshot" \
  --window-size=650,300 --screenshot="$screenshot"
  "file://$tmp/fixture.html" >"$tmp/screenshot.log" 2>>"$tmp/browser.err" || {
  echo 'GitHub card browser screenshot failed' >&2
  tail -n 25 "$tmp/browser.err" >&2 || true
  exit 1
}
[[ -s "$screenshot" ]] || { echo 'Missing browser render screenshot' >&2;exit 1; }
dimensions="$(identify -format '%wx%h' "$screenshot")"
[[ "$dimensions" == "650x300" ]] || { echo "Unexpected browser capture $dimensions" >&2;exit 1; }
echo 'GitHub-hosted SVG browser render PASS: decoded 550x145; browser screenshot archived'
