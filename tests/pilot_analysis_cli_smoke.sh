#!/usr/bin/env bash
# Strict real-binary proof for explicit local, aggregate-only public study CLI.
# Every JSON datum is an authored synthetic fixture, NOT a human study result.
set -Eeuo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT
reference="steady fingers follow quiet letters across a bright keyboard while practice builds a patient rhythm"
grep -Fq "$reference" src/training.rs || {
  echo "Public QA study passage does not match the production source" >&2;exit 1;
}
n="$(printf %s "$reference" | wc -c | tr -d ' ')"
version="$(./target/debug/typefinch --version | awk '{print $2}')"
[[ -n "$version" ]] || exit 1
make_record(){
  local arm="$1" immediate="$2" delayed="$3" destination="$4"
  jq -n --arg version "$version" --arg arm "$arm" \
    --argjson n "$n" --argjson im "$immediate" --argjson de "$delayed" '
    def score($missed): {
      opportunities:$n,ever_missed:$missed,corrected_wpm:46.5,
      attempts:($n+1),elapsed_ms:22000
    };
    {
      schema:4,type:"typefinch-local-held-out-pilot",
      app_version:$version,randomized_arm:$arm,input_mode:"os",
      configured_duration_seconds:0,reference_opportunities:$n,
      coverage_policy:"all_reference_positions_reached",
      public_reference_cohort:"typefinch.en.standard.unlimited.v1",
      immediate_deadline_seconds:600,
      retention_window_seconds:[3600,86400],
      actual_retention_gap_ms:3600000,
      control_adjusted:false,causal_claim:false,
      consent:"explicit local clipboard action; no network publication",
      baseline:score(5),practice:{
        opportunities:15,ever_missed:0,corrected_wpm:42.0,
        attempts:15,elapsed_ms:20000
      },
      immediate:score($im),delayed:score($de)
    }' >"$destination"
}
make_record adaptive 1 2 "$tmp/a.json"
make_record non_adaptive 4 5 "$tmp/b.json"
./target/debug/typefinch --compare-public-pilots "$tmp/a.json" "$tmp/b.json" \
  >"$tmp/result.json"
jq -e --arg cohort 'typefinch.en.standard.unlimited.v1' '
  .schema==1 and .causal_claim==false and .descriptive_only==true and
  .public_reference_cohort==$cohort and
  .adaptive.n==1 and .non_adaptive.n==1 and
  .difference_of_mean_immediate_missed_change_pp<0 and
  .difference_of_mean_delayed_missed_change_pp<0 and
  (.warnings|length)>=4 and
  (.raw_passage==null) and (.profile_id==null)
' "$tmp/result.json" >/dev/null
if grep -Fq "$tmp/" "$tmp/result.json" ||
   grep -Fq "$reference" "$tmp/result.json";then
  echo "Local group comparison disclosed a file path or source text" >&2;exit 1
fi
require_refused(){
  local one="$1" two="$2"
  if ./target/debug/typefinch --compare-public-pilots "$one" "$two" \
    >"$tmp/refused-out" 2>"$tmp/refused-err";then
    echo "Unsafe or incompatible study exports were accepted" >&2;exit 1
  fi
  if grep -Fq "$tmp/" "$tmp/refused-err" ||
      grep -Fq "$reference" "$tmp/refused-err";then
    echo "Failed comparison echoed private file path or study source" >&2;exit 1
  fi
}
jq '.schema=3' "$tmp/b.json" >"$tmp/old.json"
require_refused "$tmp/a.json" "$tmp/old.json"
jq '.public_reference_cohort=null' "$tmp/b.json" >"$tmp/private.json"
require_refused "$tmp/a.json" "$tmp/private.json"
jq '.raw_passage="NOT-A-REAL-PASSAGE"' "$tmp/b.json" >"$tmp/leak.json"
require_refused "$tmp/a.json" "$tmp/leak.json"
jq '.immediate.ever_missed=(.immediate.opportunities+1)' "$tmp/b.json" \
  >"$tmp/impossible.json"
require_refused "$tmp/a.json" "$tmp/impossible.json"
jq '.app_version="999.0.0"' "$tmp/b.json" >"$tmp/different.json"
require_refused "$tmp/a.json" "$tmp/different.json"
ln -s "$tmp/b.json" "$tmp/alias.json"
require_refused "$tmp/a.json" "$tmp/alias.json"
require_refused "$tmp/a.json" "$tmp/a.json"
echo "Offline public pilot analyzer PASS: descriptive two-arm output, strict malformed/private/symlink rejection; synthetic records only"
