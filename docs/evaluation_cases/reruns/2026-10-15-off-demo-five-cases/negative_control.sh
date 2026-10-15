#!/bin/sh
# Checks that check_off_demo.mjs detects wrong methods: copies the five applied sources,
# changes one helper or statement in each, and runs the check on the copies.
# Usage: sh negative_control.sh <out.json>   (from this directory)
set -eu
src=../2026-10-14-fresh-recording
tmp=$(mktemp -d)
for c in append insert_general popBack prepend setAt; do mkdir -p "$tmp/$c"; cp "$src/$c/source_after.js" "$tmp/$c/"; done
# insert: successor at i instead of i+1
sed -i '' 's/let __refsynSteps = (i + 1);/let __refsynSteps = i;/' "$tmp/insert_general/source_after.js"
# append: write to the receiver instead of the tail
sed -i '' 's/        hole1.next = created0;/        this.next = created0;/' "$tmp/append/source_after.js"
# popBack: cut after the receiver instead of the penultimate node
sed -i '' 's/        hole0.next = null;/        this.next = null;/' "$tmp/popBack/source_after.js"
# prepend: return the receiver
sed -i '' 's/        return created0;/        return this;/' "$tmp/prepend/source_after.js"
# setAt: store the index instead of the value
sed -i '' 's/    setAt_h1(a, b, hole0) { return b; }/    setAt_h1(a, b, hole0) { return a; }/' "$tmp/setAt/source_after.js"
for c in append insert_general popBack prepend setAt; do
  if cmp -s "$src/$c/source_after.js" "$tmp/$c/source_after.js"; then echo "mutation not applied: $c"; exit 1; fi
done
node check_off_demo.mjs "$1" "$tmp"
rm -r "$tmp"
