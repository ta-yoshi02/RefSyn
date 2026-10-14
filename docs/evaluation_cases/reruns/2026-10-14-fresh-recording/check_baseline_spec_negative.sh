#!/bin/sh
# Shows that check_baseline_spec.mjs rejects altered recordings.
# Usage (from the repository root): sh <run>/check_baseline_spec_negative.sh <run-dir>
# Expected: append call3 pre=false, popBack call1 post=false, prepend call2 ret=false,
# insert and setAt still equal (setAt only has its object order reversed), exit status 1.
set -u
run="$1"
tmp=$(mktemp -d)
for c in insert_general append popBack prepend setAt; do
  mkdir -p "$tmp/$c" && cp "$run/$c/raw_request.json" "$tmp/$c/"
done
node -e '
const fs = require("fs"); const t = process.argv[1];
const edit = (c, f) => { const p = t + "/" + c + "/raw_request.json"; const j = JSON.parse(fs.readFileSync(p)); f(j); fs.writeFileSync(p, JSON.stringify(j)); };
edit("append", (j) => { j.validation.cases[2].objects[0].fields.next = { ref: "main-new1" }; });      // self-reference before the call
edit("popBack", (j) => { j.method_calls[0].operations[0].from = "main-new2"; });                      // deletes the wrong edge
edit("prepend", (j) => { j.method_calls[1].operations.find((o) => o.editType === "addVariable").to = "__temp1"; }); // wrong return
edit("setAt", (j) => { j.validation.cases[0].objects.reverse(); });                                   // order only
' "$tmp"
node "$run/check_baseline_spec.mjs" "$tmp"
status=$?
rm -rf "$tmp"
echo "exit=$status"
