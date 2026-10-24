// SPDX-License-Identifier: Apache-2.0
import { mount } from "svelte";
import Mobile from "./Mobile.svelte";
import { loadUiState } from "./lib/ui-state";
import "./app.css";

// The same reason as the desktop entry: reading the persisted theme before the
// first paint is what keeps somebody who chose light from seeing a dark flash.
await loadUiState().catch(() => {
  // An unreadable state is not a reason to refuse to start.
});

const app = mount(Mobile, {
  target: document.getElementById("app")!,
});

export default app;
