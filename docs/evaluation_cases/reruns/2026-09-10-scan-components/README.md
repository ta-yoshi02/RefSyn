# Scan-component diagnostic rerun (2026-09-10)

This diagnostic removes `last_ptr`, `penultimateRef`, `nthNextRef`, and
`findByValueRef`, then asks AscendRec to synthesize tail and indexed traversal
over lists of lengths 1 through 6. It is a backend/encoding diagnostic, not an
end-to-end MOLD evaluation.

The increasing encoding follows the RefSyn fixed-ID direction at the time of this diagnostic
(`0 -> 1 -> ... -> null`). The decreasing encoding reverses only those IDs
(`n-1 -> ... -> 0 -> null`). All other task structure is shared.

Options: `maxCost=20`, `timeoutMs=10000`, `searchSizeFactor=3`, reduction
rules and goal search enabled, decreasing-measure check enabled. Each task was
run once; the manuscript does not claim repeated trials for this diagnostic.

Exact task inputs are in `tasks.json`; complete outcomes and elapsed times are
in `results.json`.

This is a historical diagnostic. The RefSyn/escher-ts SHAs and dependency
versions were not recorded in this directory, so its results do not establish
current-HEAD behavior or a fully reproducible performance measurement.
