#!/usr/bin/env bash
# Synthetic local exclusions only; no human data, telemetry or publication.
set -Eeuo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
version="$(./target/debug/typefinch --version | awk '{print $2}')"
[[ -n "$version" ]] || exit 1
create(){
  local arm="$1" stage="$2" reason="$3" dest="$4"
  jq -n --arg arm "$arm" --arg stage "$stage" --arg reason "$reason" \
    --arg version "$version" '{
      schema:1,type:"typefinch-local-pilot-exclusion",
      public_reference_cohort:"typefinch.en.standard.unlimited.v1",
      app_version:$version,randomized_arm:$arm,excluded_stage:$stage,
      reason:$reason,causal_claim:false,
      consent:"explicit local clipboard action; no network publication"
    }' >"$dest"
}
create adaptive practice participant_withdrew "$tmp/a.json"
create non_adaptive immediate focus_lost "$tmp/b.json"
create adaptive delayed protocol_invalid "$tmp/c.json"
./target/debug/typefinch --summarize-pilot-exclusions \
  "$tmp/a.json" "$tmp/b.json" "$tmp/c.json" >"$tmp/result.json"
jq -e '
  .schema==1 and .total_submitted_exclusions==3 and
  .adaptive.submitted==2 and .non_adaptive.submitted==1 and
  .adaptive.by_stage.practice==1 and .adaptive.by_stage.delayed==1 and
  .non_adaptive.by_reason.focus_lost==1 and
  .causal_claim==false and .descriptive_only==true and
  (.warnings|length)>2 and (.profile_id==null) and (.raw_passage==null)
' "$tmp/result.json" >/dev/null
if grep -Fq "$tmp/" "$tmp/result.json";then
  echo "Excluded-study report leaked an input pathname" >&2;exit 1
fi
refuse(){
  if ./target/debug/typefinch --summarize-pilot-exclusions "$@" \
    >"$tmp/bad-out" 2>"$tmp/bad-err";then
    echo "Unsafe exclusion input accepted" >&2;exit 1
  fi
  ! grep -Fq "$tmp/" "$tmp/bad-err" || {
    echo "Rejected exclusion report leaked an input pathname" >&2;exit 1
  }
}
jq '.public_reference_cohort="secret"' "$tmp/a.json" >"$tmp/private.json"
refuse "$tmp/private.json"
jq '.raw_passage="PRIVATE-TEXT"' "$tmp/a.json" >"$tmp/leak.json"
refuse "$tmp/leak.json"
jq '.excluded_stage="complete"' "$tmp/a.json" >"$tmp/complete.json"
refuse "$tmp/complete.json"
jq '.app_version="2.0.0"' "$tmp/a.json" >"$tmp/different.json"
refuse "$tmp/different.json" "$tmp/b.json"
jq '.causal_claim=true' "$tmp/a.json" >"$tmp/claim.json"
refuse "$tmp/claim.json"
ln -s "$tmp/a.json" "$tmp/link.json"
refuse "$tmp/link.json"
refuse "$tmp/a.json" "$tmp/a.json"
ln "$tmp/a.json" "$tmp/hardlink.json"
refuse "$tmp/a.json" "$tmp/hardlink.json"
echo "Local public exclusion tally PASS: real binary, explicit files, no private text, strict refusals"
