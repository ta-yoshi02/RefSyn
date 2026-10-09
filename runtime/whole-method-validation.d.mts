export interface ValidationResult {
  status: "passed" | "failed" | "unavailable";
  checked_demonstrations: number;
  error: string | null;
}
export interface ValidationPayload {
  request: unknown;
  response: {
    code: string[];
    composed_method_code?: string | null;
    escher_results?: { name: string; success: boolean }[] | null;
    validation?: ValidationResult;
  };
  taskNames: string[];
}
export function validateWholeMethod(payload: ValidationPayload): ValidationResult;
export function applyValidationResult<T extends ValidationPayload["response"]>(response: T, result: ValidationResult): T;
