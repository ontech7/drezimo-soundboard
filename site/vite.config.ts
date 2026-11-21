import { defineConfig } from "vite";

export default defineConfig({
  build: {
    // three.js on its own is about 700 kB; it loads after the page has painted.
    chunkSizeWarningLimit: 900,
  },
});
