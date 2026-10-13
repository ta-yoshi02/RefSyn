export interface ValidationResult {
  status: "passed" | "failed" | "unavailable";
  checked_demonstrations: number;
  error: string | null;
}
/** One helper's output example per demonstration: a number, a boolean, or a pre-call object ID. */
export interface HoleCheck {
  helper: string;
  expected: Array<number | boolean | { ref: string }>;
}
export interface ValidationPayload {
  request: unknown;
  response: {
    code: string[];
    composed_method_code?: string | null;
    hole_checks?: HoleCheck[] | null;
    escher_results?: { name: string; success: boolean }[] | null;
    validation?: ValidationResult;
  };
  taskNames: string[];
}
export function validateWholeMethod(payload: ValidationPayload): ValidationResult;
export function applyValidationResult<T extends ValidationPayload["response"]>(response: T, result: ValidationResult): T;
