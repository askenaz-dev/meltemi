// SPDX-License-Identifier: Apache-2.0
import { invoke } from "@tauri-apps/api/core";
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

// The window was created hidden: it is shown now, with the interface mounted
// and its theme applied, instead of as an empty frame. Not after an animation
// frame — a hidden webview may not paint, and waiting for a frame the hidden
// window never produces would always end at the host's cap, which shows the
// window anyway if this never arrives (senales-de-vida D6).
void invoke("first_frame").catch(() => {
  // Outside the desktop host (a plain browser) there is no window to show.
});

export default app;
