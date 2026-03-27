import { runRefsynTasks, type RefsynRunOutcome, type RefsynTaskSpec } from "../../../external/escher-ts/src/refsyn.ts";

export interface BrowserSynthesisOptions {
  trace?: boolean;
  maxCost?: number;
  timeoutMs?: number | null;
  searchSizeFactor?: number;
}

export interface EscherResult {
  name: string;
  success: boolean;
  rendered?: string | null;
  error?: string | null;
}

export interface SynthesisResponse {
  common_pattern?: string | null;
  hole_information?: Record<string, string[]> | null;
  code: string[];
  composed_method_code?: string | null;
  individual_codes: string[];
  list_environment_info?: string | null;
  operation_analysis?: {
    common_operations_count: number;
    total_operations_counts: number[];
    difference_summary: string;
    differences_found: number;
    synthesis_matches?: number | null;
  } | null;
  escher_results?: EscherResult[] | null;
}

export interface BrowserSynthesisArtifacts {
  response: SynthesisResponse;
  task_json?: string | null;
  spec_json?: string | null;
  warnings?: string[];
}

export interface SynthesisRequest {
  method_calls: unknown[];
  vis_graph: {
    nodes: unknown[];
    edges: unknown[];
  };
}

const defaultOptions = {
  maxCost: 20,
  timeoutMs: 2000,
  searchSizeFactor: 3,
} satisfies Required<Pick<BrowserSynthesisOptions, "maxCost" | "timeoutMs" | "searchSizeFactor">>;

const cloneResponse = (response: SynthesisResponse): SynthesisResponse =>
  JSON.parse(JSON.stringify(response)) as SynthesisResponse;

const appendWarnings = (response: SynthesisResponse, warnings: string[] = []): void => {
  if (warnings.length === 0) {
    return;
  }
  const text = warnings.join("\n");
  response.list_environment_info = response.list_environment_info
    ? `${response.list_environment_info}\n${text}`
    : text;
};

const applyTaskOutcomes = (
  response: SynthesisResponse,
  outcomes: readonly RefsynRunOutcome[],
): void => {
  const existing = response.escher_results ?? [];
  const merged: EscherResult[] = [...existing];

  for (const outcome of outcomes) {
    merged.push({
      name: outcome.name,
      success: outcome.success,
      rendered: outcome.rendered,
      error: outcome.error,
    });

    if (typeof outcome.compiled_js === "string" && outcome.compiled_js.length > 0) {
      response.code.push(outcome.compiled_js);
      response.individual_codes.push(`${outcome.name}: ${outcome.compiled_js}`);
      continue;
    }

    if (typeof outcome.error === "string" && outcome.error.length > 0) {
      response.individual_codes.push(`${outcome.name}: ERROR ${outcome.error}`);
      continue;
    }

    if (typeof outcome.rendered === "string" && outcome.rendered.length > 0) {
      response.individual_codes.push(
        `${outcome.name}: ERROR compiled_js missing for rendered term ${outcome.rendered}`,
      );
      continue;
    }

    response.individual_codes.push(`${outcome.name}: no output`);
  }

  response.escher_results = merged;
};

export const completeBrowserSynthesis = async (
  artifacts: BrowserSynthesisArtifacts,
  options: BrowserSynthesisOptions = {},
): Promise<SynthesisResponse> => {
  const response = cloneResponse(artifacts.response);
  appendWarnings(response, artifacts.warnings);

  if (!artifacts.task_json) {
    return response;
  }

  const tasks = JSON.parse(artifacts.task_json) as RefsynTaskSpec[];
  const outcomes = runRefsynTasks(tasks, {
    quiet: true,
    maxCost: options.maxCost ?? defaultOptions.maxCost,
    timeoutMs: options.timeoutMs ?? defaultOptions.timeoutMs,
    searchSizeFactor: options.searchSizeFactor ?? defaultOptions.searchSizeFactor,
  });
  applyTaskOutcomes(response, outcomes);
  return response;
};

export const handleSynthesisMessage = async (
  runCore: (
    request: SynthesisRequest,
    options: BrowserSynthesisOptions,
  ) => Promise<BrowserSynthesisArtifacts> | BrowserSynthesisArtifacts,
  request: SynthesisRequest,
  options: BrowserSynthesisOptions = {},
): Promise<SynthesisResponse> => {
  const artifacts = await runCore(request, options);
  return completeBrowserSynthesis(artifacts, options);
};
