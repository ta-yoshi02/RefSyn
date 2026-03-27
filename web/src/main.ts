import "./style.css";
import { runRefsynBrowser } from "./lib/refsyn-browser";
import { appendSample } from "./samples/append";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root not found");
}

app.innerHTML = `
  <main class="shell">
    <section class="hero">
      <p class="eyebrow">GitHub Pages Ready</p>
      <h1>RefSyn Browser Demo</h1>
      <p class="lede">
        Rust core is expected to run inside a worker-backed wasm module.
        This UI stays deliberately narrow: load a Kanon payload, run synthesis, inspect the output.
      </p>
    </section>
    <section class="workspace">
      <div class="panel controls">
        <label class="field">
          <span>Sample</span>
          <select id="sample-select">
            <option value="append">append-like trace pair</option>
          </select>
        </label>
        <label class="field">
          <span>Request JSON</span>
          <textarea id="request-input" spellcheck="false"></textarea>
        </label>
        <div class="options">
          <label class="field compact">
            <span>Max cost</span>
            <input id="max-cost" type="number" value="20" min="1" />
          </label>
          <label class="field compact">
            <span>Timeout ms</span>
            <input id="timeout-ms" type="number" value="2000" min="1" />
          </label>
          <label class="field compact">
            <span>Search size factor</span>
            <input id="search-size-factor" type="number" value="3" min="1" />
          </label>
        </div>
        <label class="toggle">
          <input id="trace" type="checkbox" />
          <span>Enable trace in Rust core</span>
        </label>
        <div class="actions">
          <button id="load-sample" class="secondary">Load sample</button>
          <button id="run-synthesis">Run browser synthesis</button>
        </div>
        <p id="status" class="status idle">Idle</p>
      </div>
      <div class="panel outputs">
        <article>
          <h2>Composed Method</h2>
          <pre id="composed-method">No result yet.</pre>
        </article>
        <article>
          <h2>Common Pattern</h2>
          <pre id="common-pattern">No result yet.</pre>
        </article>
        <article>
          <h2>Individual Codes</h2>
          <pre id="individual-codes">No result yet.</pre>
        </article>
        <article>
          <h2>Escher Results</h2>
          <pre id="escher-results">No result yet.</pre>
        </article>
        <article>
          <h2>Environment Summary</h2>
          <pre id="environment-summary">No result yet.</pre>
        </article>
      </div>
    </section>
  </main>
`;

const requestInput = document.querySelector<HTMLTextAreaElement>("#request-input");
const sampleSelect = document.querySelector<HTMLSelectElement>("#sample-select");
const loadSampleButton = document.querySelector<HTMLButtonElement>("#load-sample");
const runButton = document.querySelector<HTMLButtonElement>("#run-synthesis");
const statusEl = document.querySelector<HTMLParagraphElement>("#status");
const maxCostInput = document.querySelector<HTMLInputElement>("#max-cost");
const timeoutInput = document.querySelector<HTMLInputElement>("#timeout-ms");
const searchSizeFactorInput = document.querySelector<HTMLInputElement>("#search-size-factor");
const traceInput = document.querySelector<HTMLInputElement>("#trace");
const composedMethodEl = document.querySelector<HTMLPreElement>("#composed-method");
const commonPatternEl = document.querySelector<HTMLPreElement>("#common-pattern");
const individualCodesEl = document.querySelector<HTMLPreElement>("#individual-codes");
const escherResultsEl = document.querySelector<HTMLPreElement>("#escher-results");
const environmentSummaryEl = document.querySelector<HTMLPreElement>("#environment-summary");

if (
  !requestInput ||
  !sampleSelect ||
  !loadSampleButton ||
  !runButton ||
  !statusEl ||
  !maxCostInput ||
  !timeoutInput ||
  !searchSizeFactorInput ||
  !traceInput ||
  !composedMethodEl ||
  !commonPatternEl ||
  !individualCodesEl ||
  !escherResultsEl ||
  !environmentSummaryEl
) {
  throw new Error("UI wiring failed");
}

const samples = {
  append: appendSample,
};

const renderSample = (): void => {
  const sample = samples[sampleSelect.value as keyof typeof samples];
  requestInput.value = JSON.stringify(sample, null, 2);
};

const setStatus = (text: string, state: "idle" | "busy" | "error" | "done"): void => {
  statusEl.textContent = text;
  statusEl.className = `status ${state}`;
};

const renderResponse = (response: Awaited<ReturnType<typeof runRefsynBrowser>>): void => {
  composedMethodEl.textContent = response.composed_method_code ?? "No composed method.";
  commonPatternEl.textContent = response.common_pattern ?? "No common pattern.";
  individualCodesEl.textContent = response.individual_codes.join("\n") || "No individual code.";
  escherResultsEl.textContent = JSON.stringify(response.escher_results ?? [], null, 2);
  environmentSummaryEl.textContent = response.list_environment_info ?? "No environment summary.";
};

loadSampleButton.addEventListener("click", renderSample);

runButton.addEventListener("click", async () => {
  try {
    setStatus("Running synthesis in worker...", "busy");
    const request = JSON.parse(requestInput.value) as typeof appendSample;
    const response = await runRefsynBrowser(request, {
      trace: traceInput.checked,
      maxCost: Number(maxCostInput.value),
      timeoutMs: Number(timeoutInput.value),
      searchSizeFactor: Number(searchSizeFactorInput.value),
    });
    renderResponse(response);
    setStatus("Synthesis completed.", "done");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    composedMethodEl.textContent = "No composed method.";
    commonPatternEl.textContent = "No common pattern.";
    individualCodesEl.textContent = "No individual code.";
    escherResultsEl.textContent = `Worker error: ${message}`;
    environmentSummaryEl.textContent = "No environment summary.";
    setStatus(`Failed: ${message}`, "error");
  }
});

renderSample();
