// Saves POST bodies from the Kanon page into the evidence directory, byte for byte.
import { createServer } from "node:http";
import { writeFileSync, mkdirSync } from "node:fs";
import { resolve, dirname, relative } from "node:path";

const root = resolve(process.argv[2]);
createServer((req, res) => {
  res.setHeader("Access-Control-Allow-Origin", "http://localhost:8001");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");
  if (req.method === "OPTIONS") return res.end();
  const name = new URL(req.url, "http://x").searchParams.get("path") ?? "";
  const target = resolve(root, name);
  if (req.method !== "POST" || !name || relative(root, target).startsWith("..")) {
    res.statusCode = 400;
    return res.end("bad request");
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, Buffer.concat(chunks));
    console.log(new Date().toISOString(), "saved", relative(root, target), Buffer.concat(chunks).length, "bytes");
    res.end("ok");
  });
}).listen(8765, "127.0.0.1", () => console.log("receiver on 127.0.0.1:8765, root", root));
