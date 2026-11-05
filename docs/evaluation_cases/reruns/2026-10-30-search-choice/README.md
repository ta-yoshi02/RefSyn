# 2026-10-30: which term the search returns, under candidate search settings

Every saved helper task of nine cases was searched again under candidate settings, to see whether
changing how the search chooses among terms fixes the cases where it returned a term that fits only
the demonstrations, and what it does to the cases that already worked. Nothing was recorded anew and
nothing in `src`, `runtime` or `external/escher-ts` was changed; one setting needed a patched copy of
escher-ts' `dist` outside the repository (below).

## Cases and the intended term of each non-trivial helper

| Case | Saved task | Intended term |
| --- | --- | --- |
| append, insert, popBack, prepend, setAt | `../2026-10-14-fresh-recording/<case>/` | the terms found there (`last_ptr`, `nthNextRef`, `penultimateRef`, arguments) |
| rotateRight, 3 demonstrations | `../2026-10-15-rotate-right-494d6d4/` | `h1` = `rightOf(leftOf(this))` (this.left.right), `h2` = `leftOf(this)` |
| rotateRight, 4 demonstrations | `../2026-10-15-rotate-right-4demos-494d6d4/` | same |
| unlink | `../2026-10-30-dll-unlink/` | `prevOf(this)`, `nextOf(this)` |
| union | `../2026-10-30-uf-union/` | `h0` = `if leInt(rank(this), rank(other)) then this else other` |

## Settings

- `searchSizeFactor` 3 (adapter default) or 1. With 3, AscendRec at size level L accepts a conditional
  of total cost up to 3L built from terms of size ≤ L, and returns the first hit in level order, so a
  conditional of cost 17 or 21 can be returned before a straight-line term of cost 7 or 16 is enumerated.
  With 1, a term is accepted only at a level ≥ its cost, so the result has minimum cost (ties broken by
  enumeration order).
- List components (`last_ptr`, `penultimateRef`, `nthNextRef`, `findByValueRef`) kept at cost 1, removed,
  or kept at cost 3. Cost 3 is the smallest cost at which an application of a list component on the
  receiver (e.g. `penultimateRef(this, @prevHeap)`, 3+1+1) costs more than one field read
  (`prevOf(this, @nodeHeap, @nodeHeap)`, 4); with cost 2 they tie. **The value was chosen after seeing
  the failures in rotateRight, unlink and union**, by this rule, not tuned over several values.
- Per-component cost is not an option of escher-ts: AscendRec gives every component cost 1
  (`synthesizer.ts`, `const compCost = 1`). For the cost-3 runs a copy of `external/escher-ts/dist` (at
  `4e7c4bc`) was patched in that one line to read `globalThis.__refsynCompCost?.[impl.name] ?? 1`.
- Limits: adapter (maxCost 20, 2 s, what Kanon's server uses) and the paper's measurement (32, 15 s).

`sweep.mjs` (cost 1: `sweep_result.json`, `sweep_log.txt`; cost 3: `LIST_COST=3 ESCHER_DIST=<patched dist>`,
`sweep_listcost3_result.json`, `sweep_listcost3_log.txt`).

## Result: does each case get the intended terms? (adapter limits)

| Case | f3, list cost 1 (current) | f1, list cost 1 | f3, list removed | f1, list removed | f3, list cost 3 | **f1, list cost 3** |
| --- | --- | --- | --- | --- | --- | --- |
| append | yes | yes | no (fixed-depth conditional) | timeout | yes | **yes** |
| insert | yes | yes | no | timeout | no (`h2` conditional) | **yes** |
| popBack | yes | yes | no | exhausted | yes | **yes** |
| prepend | yes | yes | yes | yes | yes | **yes** |
| setAt | yes | yes | no | timeout (recursive term found at 15 s) | yes | **yes** |
| rotateRight (3) | no (`last_ptr`) | no | yes | yes | yes | **yes** |
| rotateRight (4) | no (cost-17 conditional) | yes | yes | yes | yes | **yes** |
| unlink | yes | yes | yes | yes | yes | **yes** |
| union | no (cost-21 conditional) | timeout (findByValueRef term at 15 s) | no | yes (`leInt`) | no | **yes (`leInt`, 1.4 s)** |

Only "factor 1 and list components at cost 3" gives the intended term for every helper of the nine
cases, and it does so within the adapter limits. The terms are the ones listed above, so the
off-demonstration checks already made for them apply (lists `../2026-10-15-off-demo-five-cases/`,
unlink 15/15; for rotateRight and union the intended term is the reference behaviour).

Notes on the other columns:

- Without the list components, traversals of unbounded length are not found: AscendRec can build a
  recursive helper (setAt, at 15 s: `if isZero(a) then this else nextOf(setAt_h0(this, …, dec(a), a))`),
  but for append, insert and popBack it times out or returns a fixed-depth conditional. The list
  components are what makes the list cases work; they have to stay, but must not be cheaper than a
  field read.
- Factor 1 alone fixes the 4-demonstration rotateRight but not the 3-demonstration one, where
  `last_ptr(leftOf(this), @rightHeap)` (6) is simply cheaper than `rightOf(leftOf(this))` (7).
- Cost 3 alone (factor 3) still returns level-first conditionals for insert `h2` and union.

## Dependence on the choice of demonstrations (`subsets_weighted.mjs`, `subsets_weighted_result.txt`)

Factor 1, list cost 3, adapter limits, every non-empty subset of the demonstrations:

- unlink: `prevOf(this)` and `nextOf(this)` from **every** subset, including a single demonstration.
  With the current settings `h0` needed the demonstration next to the head (`../2026-10-30-dll-unlink/`).
- union: `leInt` only from all four demonstrations; smaller subsets give the cheapest condition that
  separates them (`isZero(rank(this))`, `isZero(dec(rank(this)))`, …). That is what a minimum-cost
  search should do with fewer examples; it is not the shortcut through list components.

## What this supports and does not

Supports: on these nine saved tasks, the failures seen in rotateRight, unlink and union come from the
two causes (level-first acceptance, and list components cheaper than field reads), and removing both gives the intended terms without breaking the list cases.

Does not: that the setting generalizes. Nine cases were used both to find the causes and to judge the
fix, and the list cost was chosen after seeing them. It needs new cases recorded after the setting is
fixed. Factor 1 makes union `h0` take 1.4 s of the 2 s server limit, so larger conditions may time out.
Implementing it means changing the adapter default (RefSyn) and adding per-component costs to
escher-ts (a submodule), and re-running the measurements reported in the paper.
