import type { BrowserSynthesisOptions, SynthesisRequest, SynthesisResponse } from "./browser-runner";

interface WorkerSuccess {
  id: number;
  ok: true;
  response: SynthesisResponse;
}

interface WorkerFailure {
  id: number;
  ok: false;
  error: string;
}

type WorkerMessage = WorkerSuccess | WorkerFailure;

let nextRequestId = 1;
let sharedWorker: Worker | null = null;
const pending = new Map<
  number,
  {
    resolve: (response: SynthesisResponse) => void;
    reject: (error: Error) => void;
  }
>();

const getWorker = (): Worker => {
  if (sharedWorker) {
    return sharedWorker;
  }

  sharedWorker = new Worker(new URL("../worker.ts", import.meta.url), {
    type: "module",
  });
  sharedWorker.addEventListener("message", (event: MessageEvent<WorkerMessage>) => {
    const callback = pending.get(event.data.id);
    if (!callback) {
      return;
    }
    pending.delete(event.data.id);
    if (event.data.ok) {
      callback.resolve(event.data.response);
      return;
    }
    callback.reject(new Error(event.data.error));
  });
  return sharedWorker;
};

export const runRefsynBrowser = (
  request: SynthesisRequest,
  options: BrowserSynthesisOptions = {},
): Promise<SynthesisResponse> => {
  const worker = getWorker();
  const requestId = nextRequestId++;

  return new Promise((resolve, reject) => {
    pending.set(requestId, { resolve, reject });
    worker.postMessage({
      id: requestId,
      request,
      options,
    });
  });
};
