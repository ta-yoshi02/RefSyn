# Requests not sent by this recording session

- Shortly after ./target/debug/refsyn started on 3030 (see refsyn_server_start.txt), one
  POST /synthesize arrived before this session had opened any browser tab.
  - Content: methodName insert, calls call8/call9/call10, arguments [0,89] [2,46] [1,71],
    class source "class Node { constructor(v) {...} insert(i, arg) {} }".
  - Task file: target/escher/ts/insert_1791955295_786937000_p4348_0.json
  - Likely source: the Kanon served on :8000 from the main checkout (not this worktree),
    which had been using the refsyn process (PID 90770) stopped just before.
  - It is NOT part of this session's evidence and is excluded from every case directory.
