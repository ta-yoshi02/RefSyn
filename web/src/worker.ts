import init, { synthesize_browser } from "/pkg/refsyn.js";
import {
  handleSynthesisMessage,
  type BrowserSynthesisArtifacts,
  type BrowserSynthesisOptions,
  type SynthesisRequest,
} from "./lib/browser-runner";

interface WorkerRequest {
  id: number;
  request: SynthesisRequest;
  options?: BrowserSynthesisOptions;
}

let wasmInit: Promise<void> | null = null;

const ensureWasm = async (): Promise<void> => {
  if (!wasmInit) {
    wasmInit = init().then(() => undefined);
  }
  await wasmInit;
};

const runCore = async (
  request: SynthesisRequest,
  options: BrowserSynthesisOptions,
): Promise<BrowserSynthesisArtifacts> => {
  await ensureWasm();
  const raw = synthesize_browser(
    JSON.stringify(request),
    JSON.stringify({
      trace: options.trace ?? false,
    }),
  );
  return JSON.parse(raw) as BrowserSynthesisArtifacts;
};

export const handleWorkerSynthesis = (
  request: SynthesisRequest,
  options: BrowserSynthesisOptions = {},
) => handleSynthesisMessage(runCore, request, options);

self.addEventListener("message", async (event: MessageEvent<WorkerRequest>) => {
  const { id, request, options = {} } = event.data;
  try {
    const response = await handleWorkerSynthesis(request, options);
    self.postMessage({ id, ok: true, response });
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
});

export {};
