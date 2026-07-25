// SPDX-License-Identifier: Apache-2.0
import { mount } from "svelte";
import App from "./App.svelte";
import { loadUiState } from "./lib/ui-state";
import "./app.css";

// The persisted theme is applied BEFORE the first paint: mounting after the
// state is read is what keeps a user who chose light from seeing a dark flash
// (or the reverse) on every start. The read is a local IPC call, not a network
// round trip, and the shell renders immediately after it.
await loadUiState().catch(() => {
  // A missing or unreadable state is not a reason to refuse to start: the OS
  // preference then decides, which is the documented default.
});

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
