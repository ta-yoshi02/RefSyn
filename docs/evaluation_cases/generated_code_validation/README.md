# Generated-code validation

This directory records a bounded validation of the reconstructed methods saved in
`docs/evaluation_cases/reruns/2026-09-09-null-guard-removal`.

## Scope

The validation loads the generated helper methods and `composed_method_code` from
the saved response for `setAt`, `append`, `prepend`, `insert`, and `popBack`.
For every case it compares:

- the value sequence reachable from the observed head through field `g`; and
- whether that reachable structure is acyclic.

For `prepend`, the generated method's return value is used as the observed head.
Object identity, external aliases, unreachable objects, invalid indices, empty
lists, and singleton `popBack` inputs are outside this check.

## Input generation

- Base values for list length `n` are `1000 + 20*n + 3*j`, for every index `j`.
- `setAt`: list lengths 1--5, every valid index, assigned value
  `2000 + 20*n + i`.
- `append`: list lengths 1--5, appended value `3000 + n`.
- `prepend`: list lengths 1--5, prepended value `4000 + n`.
- `insert`: list lengths 1--5, every valid predecessor index, inserted value
  `5000 + 20*n + i`.
- `popBack`: list lengths 2--6.
- `setAt` controls: `[7, 7, 7], i=1, value=0` and
  `[4, 9, 2], i=1, value=4`.
- `insert` controls: `[10, -5, 10], i=1, value=0` and
  `[10, -5, 10], i=0, value=10`.

The first 45 values differ from the values in the synthesis records. The four
controls vary the assigned value independently and include duplicate and negative
list values. The check contains 49 cases in total.

## Result

All 49 cases passed:

| Method | Passed |
| --- | ---: |
| `setAt` | 15/15 |
| `append` | 5/5 |
| `prepend` | 5/5 |
| `insert` | 15/15 |
| `popBack` | 5/5 |
| `setAt` controls | 2/2 |
| `insert` controls | 2/2 |

The source implementation commit is
`3b323c5ac64852b91c2f3baaaabdb4cd4733daaa`. The recorded run used Node.js
22.17.0 on arm64 macOS 26.6.2.

## Reproduction

From the RefSyn repository root:

```sh
node docs/evaluation_cases/generated_code_validation/validate.mjs \
  --out docs/evaluation_cases/generated_code_validation/results.json
```

The script also prints the complete JSON result to standard output.
