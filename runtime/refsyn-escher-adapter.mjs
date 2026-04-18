import { anyArgSmaller } from "../external/escher-ts/dist/components/component.js";
import {
  parseJsonSynthesisSpec,
  prepareJsonSynthesisJob,
} from "../external/escher-ts/dist/components/user-friendly-json.js";
import { AscendRecSynthesizer } from "../external/escher-ts/dist/synthesis/ascendrec/synthesizer.js";
import { showTerm } from "../external/escher-ts/dist/types/term.js";
import { showType } from "../external/escher-ts/dist/types/type.js";

const defaultMaxCost = 20;
const defaultTimeoutMs = 2000;
const defaultSearchSizeFactor = 3;

const getProcessEnv = () => {
  if (typeof process === "undefined" || process === null) {
    return undefined;
  }
  return process.env;
};

const parseEnvInt = (name, fallback) => {
  const raw = getProcessEnv()?.[name];
  if (raw === undefined || raw.trim() === "") {
    return fallback;
  }
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const parseEnvNullableInt = (name, fallback) => {
  const raw = getProcessEnv()?.[name];
  if (raw === undefined || raw.trim() === "") {
    return fallback;
  }
  if (raw.trim().toLowerCase() === "null") {
    return null;
  }
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
};

const isRefType = (type) =>
  type.kind === "apply" && type.constructor.name === "Ref" && type.params.length === 1;

const isIntType = (type) =>
  type.kind === "apply" && type.constructor.name === "Int" && type.params.length === 0;

const isBoolType = (type) =>
  type.kind === "apply" && type.constructor.name === "Bool" && type.params.length === 0;

const sanitizeJsIdentifier = (raw) => {
  let out = "";
  for (const ch of raw) {
    out += /[A-Za-z0-9_$]/.test(ch) ? ch : "_";
  }
  if (out.length === 0) {
    return "_";
  }
  return /^[0-9]/.test(out) ? `_${out}` : out;
};

const resolveParamName = (raw, index) => {
  const sanitized = sanitizeJsIdentifier(raw);
  if (sanitized === "_" || /^_+$/.test(sanitized)) {
    return index === 0 ? "arg" : `arg${index}`;
  }
  if (sanitized === "this") {
    return index === 0 ? "arg_this" : `arg${index}_this`;
  }
  return sanitized;
};

const isValidJsField = (raw) => /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(raw);

const fieldAccess = (baseExpr, field) =>
  isValidJsField(field) ? `(${baseExpr}).${field}` : `(${baseExpr})[${JSON.stringify(field)}]`;

const memberCall = (baseExpr, methodName, args) => {
  const receiver = baseExpr === "this" ? "this" : `(${baseExpr})`;
  return `${receiver}.${methodName}(${args.join(", ")})`;
};

const capitalize = (text) => (text.length === 0 ? text : text[0].toUpperCase() + text.slice(1));

const inferReturnKind = (returnType) => {
  if (isRefType(returnType)) {
    return "object";
  }
  if (isIntType(returnType)) {
    return "int";
  }
  if (isBoolType(returnType)) {
    return "bool";
  }
  return "unknown";
};

const createCompileContext = (rawTask, job) => {
  const meta = rawTask.refsynMeta;
  if (meta === undefined) {
    throw new Error(`refsynMeta is required for ${rawTask.name ?? "unnamed task"}`);
  }
  const receiverIndex = job.inputNames.indexOf(meta.thisRefName);
  if (receiverIndex < 0) {
    throw new Error(`receiver input '${meta.thisRefName}' is missing from synthesized signature`);
  }

  const explicitArgs = meta.explicitArgs.map((arg, index) => {
    const inputIndex = job.inputNames.indexOf(arg.name);
    if (inputIndex < 0) {
      throw new Error(`explicit arg '${arg.name}' is missing from synthesized signature`);
    }
    return {
      rawName: arg.name,
      paramName: resolveParamName(arg.name, index),
      taskType: arg.taskType,
      inputIndex,
    };
  });

  const heapNames = new Set([meta.classHeapName, ...Object.values(meta.fieldHeapNames ?? {})]);

  return {
    taskName: rawTask.name ?? "synthesized",
    jsMethodName: meta.jsMethodName,
    thisRefName: meta.thisRefName,
    receiverIndex,
    explicitArgs,
    heapNames,
    valueFields: new Set(meta.valueFields ?? []),
    pointerFields: new Set(meta.pointerFields ?? []),
    primaryValueField: meta.valueFields?.[0] ?? null,
    primaryPointerField: meta.pointerFields?.[0] ?? null,
    returnKind: inferReturnKind(job.returnType),
  };
};

const compileVar = (name, ctx) => {
  if (name === ctx.thisRefName) {
    return { code: "this", kind: "object" };
  }

  const explicit = ctx.explicitArgs.find((arg) => arg.rawName === name);
  if (explicit !== undefined) {
    if (explicit.taskType.startsWith("Ref[")) {
      return { code: explicit.paramName, kind: "object" };
    }
    if (explicit.taskType === "Int") {
      return { code: explicit.paramName, kind: "int" };
    }
    if (explicit.taskType === "Bool") {
      return { code: explicit.paramName, kind: "bool" };
    }
    return { code: explicit.paramName, kind: "unknown" };
  }

  if (ctx.heapNames.has(name)) {
    throw new Error(`heap variable '${name}' cannot appear directly in generated JS`);
  }

  throw new Error(`unknown variable '${name}' in generated term`);
};

const compileAutoFieldComponent = (name, args, ctx) => {
  for (const field of [...ctx.valueFields, ...ctx.pointerFields]) {
    if (name === `${field}Of`) {
      if (args.length < 2) {
        throw new Error(`${name} expects receiver + heap args`);
      }
      const base = compileExpr(args[0], ctx);
      const access = fieldAccess(base.code, field);
      if (ctx.pointerFields.has(field)) {
        return { code: access, kind: "object" };
      }
      return { code: access, kind: "valueFieldAccess" };
    }
    if (name === `has${capitalize(field)}`) {
      if (args.length < 2) {
        throw new Error(`${name} expects receiver + heap args`);
      }
      const base = compileExpr(args[0], ctx);
      const access = fieldAccess(base.code, field);
      if (ctx.pointerFields.has(field)) {
        return { code: `${access} !== null`, kind: "bool" };
      }
      return { code: `${access} !== undefined`, kind: "bool" };
    }
  }
  return null;
};

const compileRecursiveCall = (args, ctx) => {
  const receiverTerm = args[ctx.receiverIndex];
  if (receiverTerm === undefined) {
    throw new Error(`recursive call is missing receiver arg at index ${ctx.receiverIndex}`);
  }
  const receiver = compileExpr(receiverTerm, ctx);
  if (receiver.kind !== "object") {
    throw new Error("recursive receiver must compile to an object expression");
  }
  const callArgs = ctx.explicitArgs.map((arg) => {
    const term = args[arg.inputIndex];
    if (term === undefined) {
      throw new Error(`recursive call is missing explicit arg '${arg.rawName}'`);
    }
    return compileExpr(term, ctx).code;
  });
  return {
    code: memberCall(receiver.code, ctx.jsMethodName, callArgs),
    kind: ctx.returnKind,
  };
};

const compileComponent = (name, args, ctx) => {
  if (name === ctx.taskName) {
    return compileRecursiveCall(args, ctx);
  }

  const autoField = compileAutoFieldComponent(name, args, ctx);
  if (autoField !== null) {
    return autoField;
  }

  switch (name) {
    // Generated traversal helpers currently assume acyclic pointer structures.
    // Cycle-safe execution is not implemented in the emitted JS yet.
    case "nthNextRef": {
      if (args.length !== 4) {
        throw new Error("nthNextRef expects 4 args");
      }
      const fieldName = ctx.primaryPointerField;
      if (fieldName === null) {
        throw new Error("nthNextRef requires at least one pointer field");
      }
      const base = compileExpr(args[0], ctx);
      const steps = compileExpr(args[3], ctx);
      return {
        code:
          `(() => { let __refsynCur = ${base.code}; let __refsynSteps = ${steps.code}; ` +
          `while (__refsynSteps > 0 && __refsynCur !== null) { __refsynCur = ${fieldAccess("__refsynCur", fieldName)}; __refsynSteps -= 1; } ` +
          "return __refsynCur; })()",
        kind: "object",
      };
    }
    case "findByValueRef": {
      if (args.length !== 5) {
        throw new Error("findByValueRef expects 5 args");
      }
      const pointerField = ctx.primaryPointerField;
      const valueField = ctx.primaryValueField;
      if (pointerField === null || valueField === null) {
        throw new Error("findByValueRef requires one pointer field and one value field");
      }
      const base = compileExpr(args[0], ctx);
      const target = compileExpr(args[4], ctx);
      return {
        code:
          `(() => { let __refsynCur = ${base.code}; const __refsynTarget = ${target.code}; ` +
          `while (__refsynCur !== null) { if (${fieldAccess("__refsynCur", valueField)} === __refsynTarget) { return __refsynCur; } ` +
          `__refsynCur = ${fieldAccess("__refsynCur", pointerField)}; } return null; })()`,
        kind: "object",
      };
    }
    case "last_ptr": {
      if (args.length !== 2) {
        throw new Error("last_ptr expects 2 args");
      }
      const fieldName = ctx.primaryPointerField;
      if (fieldName === null) {
        throw new Error("last_ptr requires at least one pointer field");
      }
      const base = compileExpr(args[0], ctx);
      return {
        code:
          `(() => { let __refsynCur = ${base.code}; ` +
          `while (__refsynCur !== null && ${fieldAccess("__refsynCur", fieldName)} !== null) { __refsynCur = ${fieldAccess("__refsynCur", fieldName)}; } ` +
          "return __refsynCur; })()",
        kind: "object",
      };
    }
    case "loadInt": {
      if (args.length !== 2) {
        throw new Error("loadInt expects 2 args");
      }
      const refExpr = compileExpr(args[1], ctx);
      if (refExpr.kind !== "valueFieldAccess") {
        throw new Error("loadInt is only supported on valueOf(...) expressions in refsyn codegen");
      }
      return { code: refExpr.code, kind: "int" };
    }
    case "isNull": {
      if (args.length !== 1) {
        throw new Error("isNull expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `${value.code} === null`, kind: "bool" };
    }
    case "equal": {
      if (args.length !== 2) {
        throw new Error("equal expects 2 args");
      }
      const left = compileExpr(args[0], ctx);
      const right = compileExpr(args[1], ctx);
      return { code: `${left.code} === ${right.code}`, kind: "bool" };
    }
    case "and": {
      if (args.length !== 2) {
        throw new Error("and expects 2 args");
      }
      const left = compileExpr(args[0], ctx);
      const right = compileExpr(args[1], ctx);
      return { code: `${left.code} && ${right.code}`, kind: "bool" };
    }
    case "or": {
      if (args.length !== 2) {
        throw new Error("or expects 2 args");
      }
      const left = compileExpr(args[0], ctx);
      const right = compileExpr(args[1], ctx);
      return { code: `${left.code} || ${right.code}`, kind: "bool" };
    }
    case "not": {
      if (args.length !== 1) {
        throw new Error("not expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `!(${value.code})`, kind: "bool" };
    }
    case "isZero": {
      if (args.length !== 1) {
        throw new Error("isZero expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `${value.code} === 0`, kind: "bool" };
    }
    case "isNonNeg": {
      if (args.length !== 1) {
        throw new Error("isNonNeg expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `${value.code} >= 0`, kind: "bool" };
    }
    case "inc": {
      if (args.length !== 1) {
        throw new Error("inc expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `${value.code} + 1`, kind: "int" };
    }
    case "dec": {
      if (args.length !== 1) {
        throw new Error("dec expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `${value.code} - 1`, kind: "int" };
    }
    case "neg": {
      if (args.length !== 1) {
        throw new Error("neg expects 1 arg");
      }
      const value = compileExpr(args[0], ctx);
      return { code: `-(${value.code})`, kind: "int" };
    }
    case "plus": {
      if (args.length !== 2) {
        throw new Error("plus expects 2 args");
      }
      const left = compileExpr(args[0], ctx);
      const right = compileExpr(args[1], ctx);
      return { code: `${left.code} + ${right.code}`, kind: "int" };
    }
    case "zero":
      return { code: "0", kind: "int" };
    case "trueConst":
      return { code: "true", kind: "bool" };
    case "falseConst":
      return { code: "false", kind: "bool" };
    case "leInt": {
      if (args.length !== 2) {
        throw new Error("leInt expects 2 args");
      }
      const left = compileExpr(args[0], ctx);
      const right = compileExpr(args[1], ctx);
      return { code: `${left.code} <= ${right.code}`, kind: "bool" };
    }
    default:
      throw new Error(`unsupported component '${name}' in refsyn adapter codegen`);
  }
};

const compileExpr = (term, ctx) => {
  switch (term.kind) {
    case "var":
      return compileVar(term.name, ctx);
    case "component":
      return compileComponent(term.name, term.args, ctx);
    case "if": {
      const cond = compileExpr(term.condition, ctx);
      const thenExpr = compileExpr(term.thenBranch, ctx);
      const elseExpr = compileExpr(term.elseBranch, ctx);
      return {
        code: `(${cond.code}) ? (${thenExpr.code}) : (${elseExpr.code})`,
        kind: thenExpr.kind === elseExpr.kind ? thenExpr.kind : "unknown",
      };
    }
    default:
      throw new Error(`unsupported term kind '${term.kind}' in refsyn adapter codegen`);
  }
};

const compileRefsynMethod = (rawTask, job, term) => {
  const ctx = createCompileContext(rawTask, job);
  const expr = compileExpr(term, ctx);
  const params = ctx.explicitArgs.map((arg) => arg.paramName).join(", ");
  return `${ctx.jsMethodName}(${params}) { return ${expr.code}; }`;
};

const renderOutcome = (taskName, inputNames, inputTypes, returnType, body) =>
  `${taskName}(${inputNames
    .map((name, index) => `@${name}: ${showType(inputTypes[index])}`)
    .join(", ")}): ${showType(returnType)} =\n  ${showTerm(body)}`;

export const runRefsynTasks = (rawTasks, options = {}) => {
  const maxCost = options.maxCost ?? parseEnvInt("ESCHER_TS_MAX_COST", defaultMaxCost);
  const timeoutMs =
    options.timeoutMs ?? parseEnvNullableInt("ESCHER_TS_TIMEOUT_MS", defaultTimeoutMs);
  const searchSizeFactor =
    options.searchSizeFactor ??
    parseEnvInt("ESCHER_TS_SEARCH_SIZE_FACTOR", defaultSearchSizeFactor);

  return rawTasks.map((rawTask) => {
    const taskName = rawTask.name ?? "synthesized";
    try {
      const spec = parseJsonSynthesisSpec(JSON.stringify(rawTask));
      const job = prepareJsonSynthesisJob(spec);
      const synth = new AscendRecSynthesizer({
        maxCost,
        timeoutMs,
        searchSizeFactor,
        useReductionRules: true,
        onlyForwardSearch: false,
        argListCompare: anyArgSmaller,
      });
      const result = synth.synthesize(
        job.functionName,
        job.inputTypes,
        job.inputNames,
        job.returnType,
        job.env,
        job.examples,
      );

      if (result === null) {
        return {
          name: taskName,
          success: false,
          rendered: null,
          error: `escher-ts could not synthesize '${taskName}' within the configured budget`,
          compiled_js: null,
        };
      }

      const rendered = renderOutcome(
        taskName,
        job.inputNames,
        job.inputTypes,
        job.returnType,
        result.program.body,
      );

      let compiledJs = null;
      try {
        compiledJs = compileRefsynMethod(rawTask, job, result.program.body);
      } catch (compileError) {
        if (!options.quiet) {
          const message =
            compileError instanceof Error ? compileError.message : String(compileError);
          console.error(`refsyn adapter codegen skipped for '${taskName}': ${message}`);
        }
      }

      return {
        name: taskName,
        success: true,
        rendered,
        error: null,
        compiled_js: compiledJs,
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        name: taskName,
        success: false,
        rendered: null,
        error: message,
        compiled_js: null,
      };
    }
  });
};

export const runRefsynTasksJson = (jsonText, options = {}) => {
  const parsed = JSON.parse(jsonText);
  if (!Array.isArray(parsed)) {
    throw new Error("refsyn runner expects a JSON array of task specs");
  }
  return JSON.stringify(runRefsynTasks(parsed, options));
};

export const applyRefsynTaskOutcomes = (response, outcomes) => {
  const existing = Array.isArray(response.escher_results) ? response.escher_results : [];
  const merged = existing.slice();

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
