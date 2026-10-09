import { validateWholeMethod } from "./whole-method-validation.mjs";
// Bound before any generated code runs, so later changes to self.postMessage cannot rewrite the reply.
const post = self.postMessage.bind(self);
self.onmessage = ({ data: { nonce, payload } }) => {
  post({ nonce, started: true });
  post({ nonce, result: validateWholeMethod(payload) });
};
