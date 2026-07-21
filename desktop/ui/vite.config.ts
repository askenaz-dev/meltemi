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
  },
});
