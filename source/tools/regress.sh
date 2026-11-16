#!/bin/bash
# Full regression: every tab, every topic, interactions — in all three language modes.
cd "$(dirname "$0")"
TABS='[{"snap":"home"},{"click":"#tabs [data-tab=learn]","snap":"learn"},{"click":"#tabs [data-tab=cards]","snap":"cards"},{"click":"#tabs [data-tab=quiz]","snap":"quiz"},{"click":"#tabs [data-tab=calc]","snap":"calc"},{"click":"#tabs [data-tab=lab]","snap":"lab"},{"click":"#tabs [data-tab=formulas]","snap":"formulas"},{"click":"#tabs [data-tab=extra]","snap":"extra"}]'
for L in both zh en; do
 for S in "$TABS" "$(cat /tmp/learn_steps.json)" "$(cat /tmp/inter.json)" "$(cat /tmp/inter2.json)"; do
  out=$(LANG2=$L node diff.js "$S" 2>&1)
  echo "$L: $(echo "$out" | grep -c '^SAME') same, $(echo "$out" | grep -c '^DIFF') different"
  echo "$out" | grep -A2 '^DIFF' | cut -c1-400 | head -9
 done
done
