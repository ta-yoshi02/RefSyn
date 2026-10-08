const RUN_TIMEOUT_MS = 1000;
const STARTUP_TIMEOUT_MS = 10000;

export function validateInWorker(payload) {
  return new Promise((resolve) => {
    let worker;
    try {
      worker = new Worker(new URL("./whole-method-validator-worker.mjs", import.meta.url), { type: "module" });
    } catch (error) {
      resolve({ status: "unavailable", checked_demonstrations: 0, error: String(error) });
      return;
    }
    // Generated code runs inside the Worker and could post messages itself; only replies carrying this nonce count.
    const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
    let timer;
    const finish = (result) => {
      clearTimeout(timer);
      worker.terminate();
      resolve(result);
    };
    // Worker startup is not part of the run budget, so the 1s limit starts once the Worker reports it began.
    timer = setTimeout(() => finish({ status: "unavailable", checked_demonstrations: 0, error: "Whole-method validator did not start" }), STARTUP_TIMEOUT_MS);
    worker.onmessage = (event) => {
      if (event.data?.nonce !== nonce) return;
      if (event.data.started) {
        clearTimeout(timer);
        timer = setTimeout(() => finish({ status: "failed", checked_demonstrations: 0, error: "Whole-method validation timed out" }), RUN_TIMEOUT_MS);
        return;
      }
      finish(event.data.result);
    };
    worker.onerror = (event) => finish({ status: "failed", checked_demonstrations: 0, error: event.message });
    try {
      worker.postMessage({ nonce, payload });
    } catch (error) {
      finish({ status: "unavailable", checked_demonstrations: 0, error: String(error) });
    }
  });
}
