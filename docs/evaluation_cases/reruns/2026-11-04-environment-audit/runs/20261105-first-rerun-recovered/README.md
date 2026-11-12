# First rerun after aligning the field encoding (2026-11-05, about 02:37-03:34 UTC), recovered

This run's outputs were overwritten by the next run before runs/ existed. The two logs here are copied
from the terminal output shown in the Claude Code session that ran it (printed in full at the time); the
other outputs of this run (rerun_fields, offdemo_*, cegis_minimal.json) are lost.

The machine was asleep for most of the run (lid closed, on battery; ../../pmset_sleep_wake.txt): sleep
from 11:39:10 +0900 with short dark wakes until 12:51:08 +0900. Two timings match sleep intervals:
- insert-fixed-gHeap candidate, 87 742 ms against the 15 s limit: sleep of 86 s from 11:49:28.
- popBack-fixed-env-pair candidate, 934 208 ms: sleep of 933 s from 11:51:25.
The synthesizer checks its limit with Date.now() inside the search, and that clock advances during
sleep, so a search that is suspended past its limit reports a timeout when it resumes. Both cases are
solved (or time out at 15 s, as before) in the runs made while the machine was awake. This is the
sequence of events consistent with the logs, not a measured suspension of the process.
