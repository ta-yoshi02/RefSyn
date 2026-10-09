import { afterEach, expect, it, vi } from "vitest";
import { validateInWorker } from "../../../runtime/whole-method-validator-client.mjs";

const payload = { request: {}, response: { code: [] }, taskNames: [] };
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

class FakeWorker {
  static last: FakeWorker;
  onmessage?: (event: { data: unknown }) => void;
  terminate = vi.fn();
  nonce?: string;
  constructor() { FakeWorker.last = this; }
  postMessage(message: { nonce: string }) { this.nonce = message.nonce; }
  reply(data: object) { this.onmessage?.({ data: { nonce: this.nonce, ...data } }); }
}

it("terminates a validator that does not finish and rejects adoption", async () => {
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  const pending = validateInWorker(payload);
  FakeWorker.last.reply({ started: true });
  await vi.advanceTimersByTimeAsync(1000);
  expect((await pending).status).toBe("failed");
  expect(FakeWorker.last.terminate).toHaveBeenCalledOnce();
});
it("does not charge Worker startup to the run budget", async () => {
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  const pending = validateInWorker(payload);
  await vi.advanceTimersByTimeAsync(5000);
  FakeWorker.last.reply({ started: true });
  FakeWorker.last.reply({ result: { status: "passed", checked_demonstrations: 1, error: null } });
  expect((await pending).status).toBe("passed");
});
it("reports a Worker that never starts as unavailable", async () => {
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  const pending = validateInWorker(payload);
  await vi.advanceTimersByTimeAsync(10000);
  expect((await pending).status).toBe("unavailable");
});
it("ignores messages that do not carry the request nonce", async () => {
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  const pending = validateInWorker(payload);
  FakeWorker.last.reply({ started: true });
  FakeWorker.last.onmessage?.({ data: { result: { status: "passed", checked_demonstrations: 1, error: null } } });
  await vi.advanceTimersByTimeAsync(1000);
  expect((await pending).status).toBe("failed");
});
it("reports unavailable execution instead of passing when Worker cannot start", async () => {
  vi.stubGlobal("Worker", class { constructor() { throw new Error("unavailable"); } });
  expect((await validateInWorker(payload)).status).toBe("unavailable");
});
