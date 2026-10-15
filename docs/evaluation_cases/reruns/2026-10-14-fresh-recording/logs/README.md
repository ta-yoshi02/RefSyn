# Logs

| File | Content |
| --- | --- |
| `refsyn_server.log` | stdout/stderr of `./target/debug/refsyn` for the whole session (raw requests, escher-ts terms, composed code). Each case's `server_log_lines_before.txt` gives the line count before its Synthesize click; `finalize_case.mjs` takes the first raw payload after it. Requests after each case's own one are the transcription-check replays. |
| `refsyn_server_start.txt` | start time, command, `ESCHER_TS_*`/`REFSYN_*` variables (none) |
| `kanon_server.log` | `npx grunt connect:server:keepalive --port=8001` output |
| `receiver.mjs`, `receiver.log` | local receiver on 127.0.0.1:8765 that wrote page data (editor text, markers, captured responses) byte for byte into the case directories |
| `FOREIGN_REQUESTS.md`, `foreign/` | one request not sent by this session (excluded from every case) |
| `failed_first_start_attempt_at.txt` | time of the first start attempt, which failed because port 3030 was held by another session's server (stopped with the user's approval); the server used here is the one in `refsyn_server_start.txt` |
