# 2026-10-15: the five main cases on inputs that were not demonstrated

The generated methods of append, insert, popBack, prepend and setAt were run on inputs
that were not demonstrated and compared with reference implementations. Nothing was
synthesized again; implementation and search were not touched.

## What was run

- **Methods:** the applied sources `../2026-10-14-fresh-recording/<case>/source_after.js`.
  Every helper and the composed method in them equal what `494d6d4` generates from the same
  requests (`../2026-10-15-fixed-494d6d4/remeasure/<case>/run_payload_output.txt`; checked by
  comparing the `code` entries and `composed_method_code` with the applied source).
- **Inputs:** lists of length 1–7 with values 100, 107, 114, … (distinct, and different from
  every argument); for setAt every index 0…length with value `index − 5`; for insert every
  index 0…length−1 with value `1000 + index`; append and prepend with 0, −3 and 41; popBack
  once per length. 112 runs. The values and lengths differ from the demonstrations; some list
  shapes coincide with demonstrated ones.
- **Compared per run** (`check_off_demo.mjs`), against the reference for the method: the list
  from the receiver (from the returned node for prepend) as object identities and values;
  `val` and `next` of every object that existed before the call; `val`, `next` and the own
  fields (`next`, `val` only) of a created object; and the return value (prepend: the created
  node; others: `undefined`).

| Method | Reference | Within the demonstrated condition | Outside it |
| --- | --- | --- | --- |
| setAt(a, b) | the node `a` steps from the receiver gets `val = b` | `0 ≤ a < length` | `a = length` |
| append(x) | a new node with `val = x` after the last node | length ≥ 1 | — |
| prepend(x) | returns a new node with `val = x`, `next` = receiver | length ≥ 1 | — |
| insert(i, arg) | a new node with `val = arg` between node `i` and node `i+1` | node `i+1` exists (`i ≤ length − 2`), as in every demonstration | `i = length − 1` (successor `null`) |
| popBack() | the next-to-last node's `next` becomes `null` | length ≥ 2 | length 1 |

## Result (`result.json`)

**Within the demonstrated condition: 97 of 97 runs agree with the reference.**

| Method | Within | Outside |
| --- | --- | --- |
| setAt | 28/28 | 0/7: `TypeError: Cannot set properties of null (setting 'val')` |
| append | 21/21 | — |
| prepend | 21/21 | — |
| insert | 21/21 | 7/7 agree (the successor hole gives `null`, so the new node becomes the last) |
| popBack | 6/6 | 0/1: `TypeError: Cannot set properties of null (setting 'next')` |

Outside the condition the reference has no defined result for setAt with `a = length` and
popBack on one node; the generated methods throw there. insert at the end was never
demonstrated, yet agrees.

## Negative control (`negative_control.sh`, `negative_control_result.json`)

Copies of the five sources with one change each — insert's successor at `i` instead of
`i+1`, append writing `this.next`, popBack cutting `this.next`, prepend returning `this`,
setAt storing the index — run through the same check. Within the condition: setAt 0/28,
prepend 0/21, insert 0/21, append 3/21 (length 1, where `this` is the tail), popBack 1/6
(length 2, where `this` is the next-to-last node). The check detects the changes except
where the changed method happens to be right.

## What this supports and does not

It supports that the five generated methods behave as the intended methods on lists of
length up to 7 within the demonstrated condition, and that the terms (`@x`, `@arg`, `@b`,
`@thisRef`, `nthNextRef(…, i)`, `nthNextRef(…, i+1)`, `last_ptr`, `penultimateRef`) are not
specific to the demonstrations. It does not cover longer lists, lists with cycles or shared
nodes, other value kinds, or a proof for all inputs. Compare the right rotation
(`../2026-10-15-rotate-right-4demos-494d6d4/`), where the methods reproduced every
demonstration and still differed on other inputs.

## Reproduce

```
node docs/evaluation_cases/reruns/2026-10-15-off-demo-five-cases/check_off_demo.mjs /tmp/result.json
cd docs/evaluation_cases/reruns/2026-10-15-off-demo-five-cases && sh negative_control.sh /tmp/negative.json
```
