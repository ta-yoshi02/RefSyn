export interface RefsynTaskMetaArg {
  readonly name: string;
  readonly legacyType: string;
  readonly taskType: string;
}

export interface RefsynTaskMeta {
  readonly jsMethodName: string;
  readonly className: string;
  readonly thisRefName: string;
  readonly classHeapName: string;
  readonly valueFields: readonly string[];
  readonly pointerFields: readonly string[];
  readonly fieldHeapNames: Readonly<Record<string, string>>;
  readonly explicitArgs: readonly RefsynTaskMetaArg[];
}

export interface RefsynTaskSpec {
  readonly name?: string;
  readonly refsynMeta?: RefsynTaskMeta;
  readonly [key: string]: unknown;
}

export interface RefsynRunOptions {
  readonly quiet?: boolean;
  readonly maxCost?: number;
  readonly timeoutMs?: number | null;
  readonly searchSizeFactor?: number;
}

export interface RefsynRunOutcome {
  readonly name: string;
  readonly success: boolean;
  readonly rendered: string | null;
  readonly error: string | null;
  readonly compiled_js?: string | null;
}

export interface RefsynResultSummary {
  name: string;
  success: boolean;
  rendered?: string | null;
  error?: string | null;
}

export interface RefsynResponseLike {
  code: string[];
  individual_codes: string[];
  escher_results?: RefsynResultSummary[] | null;
}

export function runRefsynTasks(
  rawTasks: readonly RefsynTaskSpec[],
  options?: RefsynRunOptions,
): readonly RefsynRunOutcome[];

export function runRefsynTasksJson(jsonText: string, options?: RefsynRunOptions): string;

export function applyRefsynTaskOutcomes(
  response: RefsynResponseLike,
  outcomes: readonly RefsynRunOutcome[],
): void;
