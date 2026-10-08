import { afterEach, expect, it, vi } from "vitest";
import { validateInWorker } from "../../../runtime/whole-method-validator-client.mjs";

const payload = { request: {}, response: { code: [] }, taskNames: [] };
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });
it("terminates a validator that does not finish and rejects adoption", async () => {
  vi.useFakeTimers();
  const terminate = vi.fn();
  vi.stubGlobal("Worker", class { terminate = terminate; postMessage() {} });
  const pending = validateInWorker(payload);
  await vi.advanceTimersByTimeAsync(1000);
  expect((await pending).status).toBe("failed");
  expect(terminate).toHaveBeenCalledOnce();
});
it("reports unavailable execution instead of passing when Worker cannot start", async () => {
  vi.stubGlobal("Worker", class { constructor() { throw new Error("unavailable"); } });
  expect((await validateInWorker(payload)).status).toBe("unavailable");
});
