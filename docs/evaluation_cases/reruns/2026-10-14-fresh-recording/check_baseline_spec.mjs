// Checks that the old fixed-ID baseline tasks specify the same problem as this session's
// recordings, call by call:
//   1. arguments are equal;
//   2. every object of the recorded typed pre-call state (reachable or not) maps one-to-one
//      onto the baseline's heap indices, with the receiver on `thisRef`, equal values and
//      equal references (so sharing and null are compared, not just the reachable values);
//   3. replaying the recorded edits on that state (created objects appended in creation
//      order, as the baseline encodes them) gives exactly the baseline's output heaps;
//   4. for prepend, the recorded return value maps to the baseline's `returnRef` output.
// Field mapping: new `val`/`next` correspond to the old `f`/`g`.
// Usage (from the repository root): node <run>/check_baseline_spec.mjs <run-dir>
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const run = process.argv[2];
const baselineTasks = JSON.parse(readFileSync("docs/evaluation_cases/nonseparated_general_components/tasks.json", "utf8"));
const cases = [
  ["insert_general", "insert", false],
  ["append", "append", false],
  ["popBack", "popBack", false],
  ["prepend", "prepend", true],
  ["setAt", "setAt", false],
];
const VAL = "val", NEXT = "next";

const refIndex = (r) => (r && typeof r === "object" && "ref" in r ? r.ref : null);

// Pre-state from the typed capture: id -> {val, next(id|null)}.
const preState = (capture) =>
  Object.fromEntries(capture.objects.map((o) => [o.id, { val: o.fields[VAL], next: o.fields[NEXT] === null ? null : o.fields[NEXT].ref }]));

// Finds a bijection from recorded object ids to baseline indices consistent with the input.
const findMapping = (state, receiver, input) => {
  const [thisRef, nodeHeap, fHeap, gHeap] = input;
  const ids = Object.keys(state);
  if (ids.length !== nodeHeap.length) return null;
  const toIdx = {};
  const used = new Set();
  const ok = (id, k) => {
    if (state[id].val !== fHeap[k]) return false;
    const g = refIndex(gHeap[k]);
    const n = state[id].next;
    if ((n === null) !== (g === -1)) return false;
    if (n === id && g !== k) return false;
    if (n !== null && toIdx[n] !== undefined && toIdx[n] !== g) return false;
    for (const [other, j] of Object.entries(toIdx)) if (state[other].next === id && refIndex(gHeap[j]) !== k) return false;
    return true;
  };
  const order = [receiver, ...ids.filter((id) => id !== receiver)];
  const search = (i) => {
    if (i === order.length) return true;
    const id = order[i];
    const candidates = i === 0 ? [refIndex(thisRef)] : nodeHeap.map((_, k) => k);
    for (const k of candidates) {
      if (used.has(k) || !ok(id, k)) continue;
      toIdx[id] = k;
      used.add(k);
      if (search(i + 1)) return true;
      delete toIdx[id];
      used.delete(k);
    }
    return false;
  };
  if (!search(0)) return null;
  // Independent final check of the whole mapping.
  for (const id of ids) {
    const k = toIdx[id];
    const n = state[id].next;
    if (state[id].val !== fHeap[k] || refIndex(gHeap[k]) !== (n === null ? -1 : toIdx[n])) return null;
  }
  return { ...toIdx };
};

// Replays Kanon edits; returns {state, created(order), ret}.
const replay = (pre, capture, ops) => {
  const state = structuredClone(pre);
  const literals = { ...capture.values };
  const created = [];
  let ret;
  const valueOf = (to) => (to in state ? { ref: to } : { value: literals[to] });
  const write = (from, label, to) => {
    const v = valueOf(to);
    if (label === NEXT) state[from].next = v.ref ?? null;
    else if (label === VAL) state[from].val = v.value;
    else throw new Error("unexpected field " + label);
  };
  for (const op of ops) {
    if (op.editType === "addNode" && op.isLiteral) literals[op.id] = op.type === "number" ? Number(op.label) : op.label;
    else if (op.editType === "addNode") {
      state[op.id] = { val: undefined, next: null }; // `new Node()` state: next = null, val absent
      created.push(op.id);
    } else if (op.editType === "addEdge") write(op.from, op.label, op.to);
    else if (op.editType === "editEdgeReference") write(op.from, op.label, op.newTo);
    else if (op.editType === "deleteEdge") state[op.from].next = null;
    else if (op.editType === "addVariable" && op.label === "return") ret = op.to;
    else throw new Error("unexpected edit " + op.editType);
  }
  return { state, created, ret };
};

const report = [];
for (const [dir, method, hasReturn] of cases) {
  const request = JSON.parse(readFileSync(join(run, dir, "raw_request.json"), "utf8"));
  const pair = baselineTasks.find((t) => t.name === `${method}-fixed-env-pair`);
  const retTask = hasReturn ? baselineTasks.find((t) => t.name === `${method}-fixed-returnRef`) : null;
  const calls = request.method_calls.map((call, i) => {
    const capture = request.validation.cases[i];
    const [input, output] = pair.examples[i];
    const argsBaseline = input.slice(4);
    const pre = preState(capture);
    const mapping = findMapping(pre, call.receiverObject, input);
    const result = { call: call.callLabel, args_equal: JSON.stringify(call.arguments) === JSON.stringify(argsBaseline), objects: Object.keys(pre).length, pre_state_mapping: mapping };
    if (!mapping) return { ...result, pre_equal: false, post_equal: false };
    const { state, created, ret } = replay(pre, capture, call.operations);
    const full = { ...mapping };
    created.forEach((id, j) => (full[id] = input[1].length + j));
    const size = Object.keys(full).length;
    const fHeap = Array(size), gHeap = Array(size);
    for (const [id, k] of Object.entries(full)) {
      fHeap[k] = state[id].val;
      gHeap[k] = { ref: state[id].next === null ? -1 : full[state[id].next] };
    }
    const expected = output.pair;
    result.pre_equal = true;
    result.post = { replayed: [fHeap, gHeap], baseline: expected };
    result.post_equal = JSON.stringify([fHeap, gHeap]) === JSON.stringify(expected);
    if (hasReturn) {
      const expectedRet = retTask.examples[i][1];
      result.return = { replayed: { ref: full[ret] }, baseline: expectedRet };
      result.return_equal = full[ret] === expectedRet.ref;
    }
    return result;
  });
  const ok = calls.length === pair.examples.length && calls.every((c) => c.args_equal && c.pre_equal && c.post_equal && (c.return_equal ?? true));
  report.push({ case: dir, baseline_tasks: [pair.name, ...(retTask ? [retTask.name] : [])], calls_recorded: calls.length, calls_baseline: pair.examples.length, all_equal: ok, calls });
}
writeFileSync(join(run, "baseline_spec_check.json"), JSON.stringify({ field_mapping: { val: "f", next: "g" }, cases: report }, null, 2) + "\n");
for (const r of report) console.log(r.case, "all_equal=" + r.all_equal, r.calls.map((c) => `${c.call}:args=${c.args_equal},pre=${c.pre_equal},post=${c.post_equal}${"return_equal" in c ? ",ret=" + c.return_equal : ""}`).join(" "));
process.exitCode = report.every((r) => r.all_equal) ? 0 : 1;
