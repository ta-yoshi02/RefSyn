# 2026-10-15: right rotation with a fourth demonstration

Follow-up to `../2026-10-15-rotate-right-494d6d4/` (kept unchanged). There, with three
demonstrations, the helper for `this.left.right` was synthesized as the end of the `right`
chain from `this.left`, which the demonstrations did not tell apart. Here the same three
calls are demonstrated again and a fourth is added in which `this.left.right` has a right
child, so that the two differ. Implementation, component generation and search are
unchanged (`src`, `runtime`, `Cargo.lock`, `scripts` equal to `494d6d4`; HEAD `b293584`
adds only the evidence of the previous run; the server used the adapter defaults
`maxCost=20`, `timeoutMs=2000`).

## Program and demonstrations

`rotateRight/source_before.js` is the previous program followed by

```js
root.left.right = new TreeNode();      // 25 under 20
root.left.right.val = 25;
root.left.right.right = new TreeNode(); // 27 under 25
root.left.right.right.val = 27;
root = root.rotateRight();
```

so the pre-states of calls 1–3 are those of the previous recording. Call 4 rotates at 30:
`this.left` = 20, `this.left.right` = 25, and 25 has the right child 27 (the end of the
`right` chain from 20 is 27, not 25). Every call was demonstrated with the same three
operations: `editEdgeReference this.left l→lr`, `editEdgeReference l.right lr→this`,
`addVariable return → l` (`rotateRight/operations.json`).

## Result: all stages pass, the helper is again not `this.left.right`

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis / translation | 3/3, 3/3 |
| Whole-method validation | `passed`, 4/4 demonstrations |
| Applied / Kanon re-run | yes / 4/4 `testPassed` |
| Node re-run of the four calls | every check true; final tree as expected (`rotateRight/node_rerun_result.json`, part A) |
| Transcription check | response equal except task path and key order; task JSON identical |

Per call (part A): the returned object is the original left child, the receiver's `left`
is the original `left.right`, the returned object's `right` is the receiver, only those two
references changed (compared on the objects that existed before the call, before the
caller's assignment), and the same objects stay reachable.

Helpers (template unchanged from the previous run):

| Helper | Term |
| --- | --- |
| `rotateRight_h0` | `@thisRef` |
| `rotateRight_h1` | `if equal(@thisRef, penultimateRef(@thisRef, @leftHeap)) then rightOf(last_ptr(@thisRef, @leftHeap), @nodeHeap, @nodeHeap) else last_ptr(penultimateRef(@thisRef, @leftHeap), @rightHeap)` |
| `rotateRight_h2` | `leftOf(@thisRef, @nodeHeap, @nodeHeap)` |

`rotateRight_h1` distinguishes the demonstrations by the length of the `left` chain from
the receiver: if `this.left.left` is null it takes `this.left.right`, otherwise the end of
the `right` chain from the next-to-last node of the `left` chain. That equals
`this.left.right` on the four demonstrations only.

Part B of `node_rerun_result.json` (inputs that were not demonstrated, compared with a
reference right rotation):

| Input | Generated method | Right rotation |
| --- | --- | --- |
| `50(30(20, 40(-, 45)), 70)` | **differs**: `30(20, 50(45, 70))`, node 40 lost | `30(20, 50(40(-, 45), 70))` |
| `50(30(-, 40(-, 45)), 70)` | agrees | |
| `50(30(20(10, -), 40), 70)` | **differs**: `50.left` becomes 20, which is also `30.left` | `30(20(10, -), 50(40, 70))` |
| `50(30(20, 40(35, 45(-, 47))), -)` | **differs**: `30(20, 50(47, -))`, 40, 35, 45 lost | `30(20, 50(40(35, 45(-, 47)), -))` |
| `50(30(20, -), 70)` (`left.right` null; outside the demonstrated condition) | **differs**: `50.left` becomes 30, a cycle | `30(20, 50(-, 70))` |

## Diagnosis (no search run, no change)

`diagnose_terms.mjs` evaluates candidate terms with escher-ts' own semantics on the saved
task's four examples (`diagnose_terms.json`):

| Term | Satisfies all four examples |
| --- | --- |
| the term found here | yes |
| the term found with three demonstrations, `last_ptr(leftOf(this), @rightHeap)` | no (fails example 4, as intended) |
| `rightOf(leftOf(@thisRef, @nodeHeap, @nodeHeap), @nodeHeap, @nodeHeap)` | **yes** |
| `rightOf(leftOf(@thisRef, @nodeHeap, @leftHeap), @nodeHeap, @rightHeap)` | **yes** |

`rightOf` and `leftOf` are in the component set (the task's components are listed in
`diagnose_terms.json`). So the intended term was expressible and consistent with the
examples, and the search returned a conditional instead. By the paper's cost (each input,
constant and application 1) the intended term costs 7; the condition and the two branches of
the conditional cost 5, 6 and 5 (16, before counting the conditional itself), so each part
costs at most 6. A plausible reason is that AscendRec assembles a
conditional from parts found at a lower cost level before it reaches level 7; the search was
not traced, so this is not established.

What this shows: adding a demonstration that separates the earlier term removed that term,
and the result passes every check on the demonstrations; but the synthesized helper is
still specific to the demonstrations and is wrong on four of five other inputs. The
whole-method validation and Kanon's re-run check only the demonstrations and did not detect
this.

## How it was recorded, and deviations

As in `../2026-10-15-rotate-right-494d6d4/README.md` (vis toolbar; layout set with
`nodes.update` and physics stopped with `stopSimulation`, which changes drawing only;
operations read back after every edit). No demonstration was discarded. The work was
interrupted once (usage limit) after demonstration 4's operations were recorded and before
it was accepted; the open test window and its recorded operations were unchanged when work
resumed (read back before accepting). `check_rotate.mjs` is the previous one extended to four
calls and five probes; it marks a node met again on the same path as `cycle at <val>`.

## Layout

| Path | Content |
| --- | --- |
| `rotateRight/` | the recording |
| `check_rotate.mjs` | Node re-run (part A: four demonstrated calls; part B: five other inputs) |
| `diagnose_terms.mjs`, `diagnose_terms.json` | evaluation of candidate terms on the task's examples |
| `finalize_case.mjs`, `collect_provenance.sh` | evidence extraction and provenance |
| `logs/` | server, Kanon and receiver logs (`.log` files match `.gitignore`) |
| `SHA256SUMS` | sha256 of every other file here |
