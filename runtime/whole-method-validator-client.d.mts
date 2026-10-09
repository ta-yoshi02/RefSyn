import type { ValidationPayload, ValidationResult } from "./whole-method-validation.mjs";
export function validateInWorker(payload: ValidationPayload): Promise<ValidationResult>;
