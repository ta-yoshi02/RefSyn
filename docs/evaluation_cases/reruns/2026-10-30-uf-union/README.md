# 2026-10-30: Union-Find, union by rank of two roots

Two Kanon recordings of `union(other)` on a `UF` class (`parent`, `rank`), to see a case where the
hole has to choose an object by a condition. The demonstrations, predictions and the inputs for the
check on other inputs were fixed in `plan.md` before recording. Nothing in `src`, `runtime`, Kanon or
escher-ts was changed (`provenance.txt`: RefSyn `a21e42c`, no tracked change; Kanon `93e6ba7`;
escher-ts `4e7c4bc`; adapter sha256 `9540366b…`; native server with the adapter defaults
`maxCost=20`, `timeoutMs=2000`, `searchSizeFactor=3`). The run with the added `lt` component was made
offline on the saved task; it was not taken through Kanon.

## Recording 1: four calls with different ranks (`union/`)

| Call | this (rank) | other (rank) | Recorded operation |
| --- | --- | --- | --- |
| `a.union(b)` | a (1) | b (0) | `addEdge b.parent → a` |
| `d.union(e)` | d (0) | e (1) | `addEdge d.parent → e` |
| `c.union(a)` | c (2) | a (1) | `addEdge a.parent → c` |
| `e.union(c)` | e (1) | c (2) | `addEdge e.parent → c` |

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis / translation | 2/2, 2/2 |
| Whole-method validation | `passed`, 4/4 demonstrations |
| Applied to the editor / Kanon re-run | yes / 4/4 `testPassed` (`union/kanon_rerun_markers.json`) |
| Other inputs (two childless roots, ranks 0–3, r1 ≠ r2, both orders) | **9/12 agree** (`union/off_demo_result.json`) |

Composed method: `hole0 = union_h0(other); hole1 = union_h1(other, hole0); hole0.parent = hole1;`

| Helper | Term |
| --- | --- |
| `union_h0` (the root that gets a parent) | `if isZero(rank(this)) then this else if isNull(findByValueRef(other, …, parentHeap, rankHeap, 2)) then other else this` (cost 21) |
| `union_h1` (the new parent) | `if equal(this, hole0) then other else this` |

`rank(x)` stands for `loadInt(@rankHeap, rankOf(x, @nodeHeap, @rankHeap))`. `union_h0` says: if this
has rank 0 it becomes the child; otherwise other becomes the child unless a node of rank 2 is reached
from other along `parent`. That separates the four demonstrations and is not a rank comparison. It
differs from union by rank on (this, other) = (1, 3), (2, 3) and (3, 2).

## Search conditions compared on the saved task (`union_variants.mjs`, `union/variants_result.txt`)

(A) the task as saved; (B) the task with one added component `lt: (Int, Int) → Bool`. Each with the
adapter limits (20 / 2 s) and the paper's measurement limits (32 / 15 s), and with `searchSizeFactor`
3 (the adapter default) and 1.

| | factor 3 | factor 1 |
| --- | --- | --- |
| A, adapter limits | the term above, level 9, cost 21 | timeout |
| A, paper limits | same | `if isNull(findByValueRef(this, …, inc(rank(other)))) then this else other`, level 16, cost 16 (4.4 s) |
| B, adapter limits | same as A | timeout |
| B, paper limits | same as A | same as A (4.7 s) |

`union_h1` is the same term in every run.

Diagnosis without search (`diagnose_union_terms.mjs`, `union/diagnose_terms.json`): with `lt` added,
`if lt(rank(this), rank(other)) then this else other` satisfies all four examples and costs 16. The
term found with factor 1 also costs 16 and satisfies them; it says "this becomes the child unless this
has rank `rank(other)+1`", which holds in the demonstrations because every pair of ranks differed by
exactly 1. So:

- With factor 3 the search returns a cost-21 conditional at level 9, before the cost-16 terms can be
  formed (their condition alone costs 13). Adding `lt` changes nothing there.
- With factor 1 the search reaches level 16 and returns a term of the same cost as the intended one;
  which of the two equal-cost terms comes first is an ordering detail of the search, not a preference
  for comparisons. The term is still specific to the demonstrations.
- So `lt` makes the intended term expressible but, with these demonstrations, it is not selected under
  either factor. A demonstration with a rank difference other than 1 would exclude the factor-1 term;
  that was not recorded.

**Correction (found afterwards in `../2026-10-30-search-choice/`):** `plan.md` and the run (B) assumed
that the task had no comparison of two integers. It has one: escher-ts adds `leInt` (≤) to every task
implicitly (`external/escher-ts/src/components/user-friendly-json.ts`, `implicitJsonComponentDefs`), and
the adapter translates it. `if leInt(rank(this), rank(other)) then this else other`, which is union by
rank for different ranks, was expressible without `lt`; with factor 1 and the list components removed
the search returns it. Adding `lt` therefore tested nothing that `leInt` did not already allow.

## Recording 2: equal ranks are rejected (`union_tie/`)

`a.union(b)` (1 vs 0: `addEdge b.parent → a`) and `f.union(g)` (0 vs 0: `addEdge g.parent → f`,
`addNode 1` (literal), `editEdgeReference f.rank 0 → 1`). The server answered HTTP 400 before synthesis:
`demonstration 2: the operation graph is not isomorphic to the first one (3 vertices and 2 writes vs 2
and 1)`, as predicted. Union by rank, whose rank write happens only on equal ranks, cannot be
demonstrated as one method in the current template; demonstrating only one of the two cases is the
only way to obtain a method, and that method is wrong on the other case.

## How it was recorded, and deviations

- Same set-up as `../2026-10-30-dll-unlink/` (servers started after checking the ports and stopped
  afterwards; program put in with `__$__.editor.setValue`; test windows opened through "set test";
  node positions fixed and edges drawn straight with JS, drawing only; every edit made with the mouse
  through vis' toolbar and the operation list read back after it). The page was reloaded between the
  two recordings.
- New edges were added with Add Edge and the label typed into Kanon's label field. The literal `1` was
  added with Add Node, label and Literal checkbox read back before saving.
- In recording 2 the first drag of `f.rank`'s end moved the canvas instead of the edge; nothing was
  recorded by it (read back), the view was fitted again and the edge dragged again. No demonstration
  was discarded.
- `operations.json` is taken from `method_calls` in the request. `union/mold_escher_tasks.json` is the
  task file the server saved (`target/escher/ts/union_1793326449_802265000_p60071_0.json`).
