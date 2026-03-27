import { fileURLToPath } from "node:url";
import { defineConfig } from "../external/escher-ts/node_modules/vitest/dist/config.js";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const webRoot = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root: webRoot,
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
