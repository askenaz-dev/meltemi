// SPDX-License-Identifier: Apache-2.0
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The dev server is only ever reached by the Tauri webview on localhost; the
// production bundle is embedded and served by Tauri itself (no network).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
    // Two entries, one bundle: the desktop shell and the mobile companion. They
    // share every module below the shell — stores, catalogue, the decision
    // modules — and differ only in what sits on top, which is the whole point
    // of a companion rather than a shrunken desktop (companero-movil design D1).
    rollupOptions: {
      input: {
        index: "index.html",
        mobile: "mobile.html",
      },
    },
  },
});
