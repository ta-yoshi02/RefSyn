import fs from 'node:fs';

const [payloadPath, responsePath, resultPath] = process.argv.slice(2);
const payload = JSON.parse(fs.readFileSync(payloadPath, 'utf8'));
const response = JSON.parse(fs.readFileSync(responsePath, 'utf8'));
const results = [];
for (const call of payload.method_calls) {
  const graph = call.precondGraph ?? payload.vis_graph;
  const created = [];
  class Node {
    constructor() { this.val = undefined; this.next = null; created.push(this); }
  }
  const methods = [response.composed_method_code, ...response.code].filter(Boolean).join(',\n');
  Object.assign(Node.prototype, eval(`({${methods}})`));
  const objects = new Map();
  const literals = new Map([['null', null]]);
  const literal = value => typeof value === 'string' && /^-?\d+$/.test(value) ? Number(value) : value;
  for (const node of graph.nodes) {
    if (node.isLiteral) literals.set(node.id, literal(node.label));
    else if (!node.id.startsWith('__Variable-') && node.id !== '__RectForVariable__') {
      objects.set(node.id, Object.assign(Object.create(Node.prototype), {val:undefined,next:null}));
    }
  }
  const resolve = id => objects.has(id) ? objects.get(id) : literals.get(id);
  for (const edge of graph.edges) {
    if (objects.has(edge.from)) objects.get(edge.from)[edge.label] = resolve(edge.to);
  }
  const expected = new Map();
  for (const [id, object] of objects) expected.set(id, {...object});
  const expectedIds = new Map([...objects].map(([id, object]) => [object, id]));
  for (const object of expected.values()) {
    for (const field of Object.keys(object)) {
      if (expectedIds.has(object[field])) object[field] = {ref: expectedIds.get(object[field])};
    }
  }
  const newIds = [];
  let expectedReturn;
  for (const op of call.operations) {
    if (op.editType === 'addNode') {
      if (op.isLiteral) literals.set(op.id, literal(op.label));
      else { expected.set(op.id, {val:undefined,next:null}); newIds.push(op.id); }
    } else if (['addEdge','editEdgeReference','deleteEdge'].includes(op.editType)) {
      const target = op.newTo ?? op.to;
      const value = op.editType === 'deleteEdge' ? null : expected.has(target) ? {ref:target} : literals.get(target);
      expected.get(op.from)[op.label] = value;
    } else if (['addVariable','editVariableReference'].includes(op.editType) && op.label === 'return') {
      const target = op.newTo ?? op.to;
      expectedReturn = expected.has(target) ? {ref:target} : literals.get(target);
    } else throw new Error(`Unsupported replay operation ${op.editType}`);
  }
  let actualReturn, error;
  try { actualReturn = objects.get(call.receiverObject)[call.methodName](...call.arguments); }
  catch (e) { error = String(e); }
  const actualIds = new Map([...objects].map(([id, object]) => [object,id]));
  created.forEach((object,i) => actualIds.set(object,newIds[i] ?? `unexpected${i}`));
  const encode = value => actualIds.has(value) ? {ref:actualIds.get(value)} : value;
  const mismatches = [];
  if (created.length !== newIds.length) mismatches.push({kind:'allocationCount', expected:newIds.length, actual:created.length});
  for (const [id, fields] of expected) {
    const object = objects.get(id) ?? created[newIds.indexOf(id)];
    if (!object) { mismatches.push({id,kind:'missingObject'}); continue; }
    for (const field of new Set([...Object.keys(fields), ...Object.keys(object)])) {
      const actual=encode(object[field]);
      if (JSON.stringify(actual) !== JSON.stringify(fields[field])) mismatches.push({id,field,expected:fields[field],actual});
    }
  }
  if (JSON.stringify(encode(actualReturn)) !== JSON.stringify(expectedReturn)) mismatches.push({kind:'return', expected:expectedReturn,actual:encode(actualReturn)});
  results.push({call:call.callLabel,arguments:call.arguments,error,mismatches,pass:!error && mismatches.length===0});
}
fs.writeFileSync(resultPath,JSON.stringify(results,null,2));
console.log(JSON.stringify(results,null,2));
