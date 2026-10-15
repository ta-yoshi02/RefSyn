# Requests not sent by this recording session

- Shortly after ./target/debug/refsyn started on 3030 (see refsyn_server_start.txt), one
  POST /synthesize arrived before this session had opened any browser tab.
  - Content: methodName insert, calls call8/call9/call10, arguments [0,89] [2,46] [1,71],
    class source "class Node { constructor(v) {...} insert(i, arg) {} }".
  - Task file: target/escher/ts/insert_1791955295_786937000_p4348_0.json
  - Source (corrected 2026-10-15): the request body is byte-identical (sha256
    `9872b622b2a6515e…`) to `../../2026-10-14-recording/case1-capture/request.json`, which
    another session captured at 2026-10-14T04:44:03Z from Kanon on :8100 and sent to its
    server PID 90770. So this was a resend from that session after PID 90770 stopped. The
    first version of this note guessed the Kanon on :8000 of the main checkout; that guess
    was wrong.
  - PID 90770: this session stopped it with `kill 90770` (SIGTERM), with the user's
    approval, just before starting its own server at 2026-10-14T05:21:24Z, because Kanon
    and RefSyn both hard-code port 3030. `../../2026-10-14-recording/setup/server-start.txt`
    records that PID 90770 exited with code 144 and that the restart at 14:21:30 JST failed
    because this session's PID 4348 held the port. A SIGTERM exit is usually reported as 143;
    why 144 was recorded is not known from either record.
  - It is NOT part of this session's evidence and is excluded from every case directory.
