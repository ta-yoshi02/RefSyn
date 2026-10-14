// Injected into the Kanon page (in-app browser) after load. It only observes: each wrapped
// function is called with the same arguments and its result returned unchanged.
(() => {
  const T = __$__.Testize;
  if (T.__refsynRecordInstalled) return "already installed";
  const clone = (x) => { try { return JSON.parse(JSON.stringify(x)); } catch (e) { return `unserializable: ${e}`; } };
  const rec = (window.__refsynRecord = { installedAt: new Date().toISOString(), href: location.href, events: [] });
  // ID normalization: inputs before the call (merge* mutate their first argument) and results.
  for (const name of ["buildTempToActualIdMapping", "buildRuntimeAliasMapping", "sanitizeIdMapping",
    "mergeRuntimeIdMapping", "mergeRuntimeAliasMapping", "normalizeOperationsWithRuntimeMap", "normalizeIdWithRuntimeMap"]) {
    const original = T[name];
    T[name] = function (...args) {
      const before = clone(args);
      const out = original.apply(this, args);
      rec.events.push({ fn: name, at: new Date().toISOString(), argsBefore: before, argsAfter: clone(args), out: clone(out) });
      return out;
    };
  }
  // The request body exactly as Kanon serializes it, and the parsed response.
  const dispatch = T.dispatchSynthesisRequest;
  T.dispatchSynthesisRequest = function (payload) {
    const entry = { fn: "dispatchSynthesisRequest", at: new Date().toISOString(), body: JSON.stringify(payload) };
    rec.events.push(entry);
    return dispatch.call(this, payload).then(
      (response) => { entry.respondedAt = new Date().toISOString(); entry.response = clone(response); return response; },
      (error) => { entry.error = String(error && error.stack || error); throw error; });
  };
  const warn = console.warn;
  console.warn = function (...args) {
    rec.events.push({ fn: "console.warn", at: new Date().toISOString(), args: args.map(String) });
    return warn.apply(this, args);
  };
  T.__refsynRecordInstalled = true;
  return "installed";
})();
