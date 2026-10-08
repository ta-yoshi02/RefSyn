import { validateWholeMethod } from "./whole-method-validation.mjs";
self.onmessage = (event) => self.postMessage(validateWholeMethod(event.data));
