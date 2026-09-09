# `swapAdjacent` fixed-version diagnostic

The payload contains three executions at indices 1, 3, and 2. Each execution
swaps the values of the node at `i` and its successor using two same-kind field
updates.

The payload was synthesized three times at implementation commit
`3b323c5ac64852b91c2f3baaaabdb4cd4733daaa`. Every run solved all four helper
tasks, but the selected expressions and composed assignment order varied. When
the three composed methods were executed on the three recorded preconditions,
all nine observations differed from the expected value sequence.

This evidence supports the following diagnosis: the current correspondence
criterion does not uniquely associate two same-kind updates with their intended
roles, and the selected correspondence is not validated by executing the whole
recomposed method. It does not support treating the older output sequence
`2,3,2` as the unique failure path.

Run `validate.mjs --out validation.json` to reproduce the execution check.
