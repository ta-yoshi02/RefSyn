# 2026-10-30: doubly linked list, unlinking a middle node

One Kanon recording of `unlink()` on a middle node of a doubly linked list, to see how the
current implementation handles a class with a forward and a backward pointer field. The
demonstrations and the inputs for the check on other inputs were fixed in `plan.md` before
recording. Nothing in `src`, `runtime`, Kanon or escher-ts was changed, and no component was
added (`unlink/provenance.txt`: RefSyn `a21e42c`, no tracked change; Kanon `93e6ba7`;
escher-ts `4e7c4bc`; adapter sha256 `9540366b…`, the same as in the 2026-10-15 runs; native
server `target/debug/refsyn` with the adapter defaults `maxCost=20`, `timeoutMs=2000`,
`searchSizeFactor=3`).

**Kanon is at `93e6ba7` here, not `60fa5c8` as in the 2026-10-15 recordings.**

## Program and demonstrations

`unlink/source_before.js`: `class Node { constructor() { this.prev = null; this.next = null; } unlink() {} }`,
list 10–60, three calls:

| Call | Receiver | this.prev | this.next | Recorded operations (`unlink/operations.json`) |
| --- | --- | --- | --- | --- |
| `head.next.next.unlink()` | 30 | 20 | 40 | `editEdgeReference 20.next 30→40`, `editEdgeReference 40.prev 30→20` |
| `head.next.unlink()` | 20 | 10 (head) | 40 | `editEdgeReference 10.next 20→40`, `editEdgeReference 40.prev 20→10` |
| `head.next.next.unlink()` | 50 | 40 | 60 (tail) | `editEdgeReference 40.next 50→60`, `editEdgeReference 60.prev 50→40` |

## Result

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis / translation | 2/2, 2/2 |
| Whole-method validation | `passed`, 3/3 demonstrations |
| Applied to the editor | yes (`unlink/source_after.js`) |
| Kanon re-run | 3/3 `testPassed` (`unlink/kanon_rerun_markers.json`) |
| Other inputs within the demonstrated condition | **15/15 agree** with a reference unlink (`unlink/off_demo_result.json`) |

Helpers: `unlink_h0` = `prevOf(@thisRef, @nodeHeap, @nodeHeap)` (`this.prev`), `unlink_h1` =
`nextOf(@thisRef, @nodeHeap, @nodeHeap)` (`this.next`). Composed method:

```js
unlink() {
    const hole0 = this.unlink_h0();
    const hole1 = this.unlink_h1(hole0);
    hole0.next = hole1;
    hole1.prev = hole0;
}
```

Other inputs (`check_unlink.mjs`): every list of length 3–7 and every position, the generated
method and a reference on separate copies, comparing `val`, `prev` and `next` of every node.
At the 15 middle positions all agree. At the 10 end positions (`this.prev` or `this.next`
null, which no demonstration had) the method throws a `TypeError` on the null write; the
demonstrations do not determine the behaviour there, so this is reported, not counted.

## Which demonstrations the result depends on

`subset_search.mjs` re-runs each helper task with the adapter settings on every subset of the
three demonstrations (`unlink/subset_search_result.txt`):

| Helper | Subsets giving the field read | Other results |
| --- | --- | --- |
| `h0` (`this.prev`) | {1,2}, {2,3}, {1,2,3} | {1}, {3}, {1,3}: `penultimateRef(@thisRef, @prevHeap)`; {2}: `last_ptr(@thisRef, @prevHeap)` |
| `h1` (`this.next`) | every subset except {3} | {3}: `last_ptr(@thisRef, @nextHeap)` |

`h0` is the field read only when call 2 (the receiver right after the head) is present. In calls
1 and 3 the node two steps back along `prev` is the head, so `penultimateRef` along `prev`
equals `this.prev`; `plan.md` predicted that call 1 would exclude the list components and
missed this. The list components (`last_ptr`, `penultimateRef`) cost 3 and the field read
costs 4, so whenever a demonstration leaves one of them equal to the field, the search returns
it. What this recording supports: with these three demonstrations the current implementation
returns the field reads and the method agrees on all middle positions up to length 7. What it
does not: that a user's natural choice of demonstrations avoids the list components; the
subset results show it depends on including a receiver adjacent to an end.

## How it was recorded, and deviations

- Servers started by this session after checking that ports 3030 and 8001 were free
  (`target/debug/refsyn` on 3030, `python3 -m http.server 8001` on `Kanon/`), stopped after
  the recording. A one-shot receiver on 127.0.0.1:8765 wrote the page's evidence files and exited.
- The program was put into the editor with `__$__.editor.setValue`; each test window was
  opened through Kanon's "set test" tooltip on the call's parentheses.
- In each test window the node positions were set with `nodes.update({id, x, y, fixed: true})`,
  the `prev`/`next` edges were drawn curved, and physics was stopped; this changes drawing only.
  All edits were made with the mouse through vis' Edit Edge tool, and the recorded operation list
  was read from `__$__.Testize.focusedTestOperations` after every edit.
- In call 3 the first drag of the `prev` edge's end moved the canvas instead of the edge; no
  operation was recorded by it (read back) and the edge was then dragged again. No demonstration
  was discarded.
- A wrapper around `window.fetch`, installed before Synthesize, kept a copy of the request body
  and response text (`unlink/raw_request.json`, `unlink/response.json`); Kanon received the
  original response. `unlink/mold_escher_tasks.json` is the task file the server saved
  (`target/escher/ts/unlink_1793325744_144375000_p52265_0.json`).
- `unlink/operations.json` is taken from `method_calls` in the request; Kanon's own stored
  operation list was empty after the re-run.
