import fs from "node:fs";
import vm from "node:vm";
import { validateWholeMethod } from "../runtime/whole-method-validation.mjs";
const payload = JSON.parse(fs.readFileSync(0, "utf8"));
let result;
try {
  result = vm.runInNewContext(`(${validateWholeMethod.toString()})(payload)`, { payload }, { timeout: 1000 });
} catch (error) {
  result = { status: "failed", checked_demonstrations: 0, error: String(error.message ?? error) };
}
process.stdout.write(JSON.stringify(result));
