export function validateInWorker(payload) {
  return new Promise((resolve) => {
    let worker;
    try {
      worker = new Worker(new URL("./whole-method-validator-worker.mjs", import.meta.url), { type: "module" });
    } catch (error) {
      resolve({ status: "unavailable", checked_demonstrations: 0, error: String(error) });
      return;
    }
    const finish = (result) => {
      clearTimeout(timer);
      worker.terminate();
      resolve(result);
    };
    const timer = setTimeout(() => finish({ status: "failed", checked_demonstrations: 0, error: "Whole-method validation timed out" }), 1000);
    worker.onmessage = (event) => finish(event.data);
    worker.onerror = (event) => finish({ status: "failed", checked_demonstrations: 0, error: event.message });
    try {
      worker.postMessage(payload);
    } catch (error) {
      finish({ status: "unavailable", checked_demonstrations: 0, error: String(error) });
    }
  });
}
