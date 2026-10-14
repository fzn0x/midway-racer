import { defineConfig } from "vitest/config";

export default defineConfig({
  // Relative URLs let the build be served from any path, such as a
  // GitHub Pages project site.
  base: "./",
  build: {
    target: "es2022",
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
