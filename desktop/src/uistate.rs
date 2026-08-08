// SPDX-License-Identifier: Apache-2.0

//! Persistent surface state (gui-clase-mundial design D3): theme, window
//! geometry, last view, editor recents, palette frecency and the "open with"
//! template live in `<data_dir>/desktop-ui.json` — the user's data directory,
//! like the onboarding flag, because webview storage is wiped by runtime
//! reinstalls. Nothing here leaves the machine and nothing is a credential.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The window's remembered geometry, in physical pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub maximized: bool,
}

/// One palette entry's usage record (recency + count = frecency).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub count: u32,
    /// Unix seconds of the last invocation.
    pub last_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    /// `system` (default), `light` or `dark`.
    pub theme: Option<String>,
    /// `es`, `en`, or absent to follow the system.
    pub locale: Option<String>,
    pub last_view: Option<String>,
    pub window: Option<WindowState>,
    /// `{file}` / `{line}` template handed to the user's editor.
    pub open_with_template: Option<String>,
    /// Recently opened files per project root.
    pub editor_recents: HashMap<String, Vec<String>>,
    /// Palette frecency per contract method.
    pub palette_usage: HashMap<String, Usage>,
    /// The project the surface was last scoped to.
    pub active_project: Option<String>,
    /// BYO language servers the user configured, by LSP language id: the argv to
    /// run, the first element being the binary. Overrides the well-known default
    /// for that language and adds languages Meltemi knows nothing about. Nothing
    /// is bundled either way — the binary is always the user's.
    #[serde(default)]
    pub lsp_servers: HashMap<String, Vec<String>>,
    /// Whether the navigation sidebar is folded to its rail. Absent — a fresh
    /// profile — means unfolded: the first run shows the whole navigation,
    /// which is what the onboarding promises (panel-opaco-y-nav-plegable D3).
    #[serde(default)]
    pub nav_collapsed: bool,
}

/// The command the user configured for a language, if any.
pub fn configured_lsp(_app: &tauri::AppHandle, language: &str) -> Option<Vec<String>> {
    UiState::load().lsp_servers.get(language).cloned()
}

fn state_path() -> PathBuf {
    meltemi_client::paths::data_dir().join("desktop-ui.json")
}

impl UiState {
    /// Reads the persisted state; a missing or corrupt file yields defaults, so
    /// a first run behaves exactly like the pre-persistence surface.
    pub fn load() -> Self {
        std::fs::read_to_string(state_path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Writes the state; best-effort, a failure never breaks the surface.
    pub fn save(&self) {
        let path = state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(raw) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, raw);
        }
    }
}

#[tauri::command]
pub fn ui_state_load() -> UiState {
    UiState::load()
}

#[tauri::command]
pub fn ui_state_save(state: UiState) {
    state.save();
}

/// Whether a remembered geometry is still usable: its top-left must fall
/// inside a currently attached monitor, or a monitor change would strand the
/// window off-screen (design D3 / gui-clase-mundial spec).
pub fn geometry_is_visible(monitors: &[(i32, i32, u32, u32)], state: &WindowState) -> bool {
    // A margin so a window remembered flush against an edge still counts.
    const MARGIN: i32 = 8;
    monitors.iter().any(|&(mx, my, mw, mh)| {
        state.x + MARGIN >= mx
            && state.y + MARGIN >= my
            && state.x + MARGIN < mx + mw as i32
            && state.y + MARGIN < my + mh as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: (i32, i32, u32, u32) = (0, 0, 1920, 1080);

    // Scenario: Ventana restaurada con regla de visibilidad
    #[test]
    fn a_geometry_outside_every_monitor_is_refused() {
        let inside = WindowState {
            x: 100,
            y: 80,
            width: 1200,
            height: 800,
            maximized: false,
        };
        assert!(geometry_is_visible(&[MONITOR], &inside));

        // The second monitor is gone: the remembered position is stranded.
        let stranded = WindowState { x: 2600, ..inside };
        assert!(!geometry_is_visible(&[MONITOR], &stranded));
        // With that monitor attached again it is usable once more.
        assert!(geometry_is_visible(
            &[MONITOR, (1920, 0, 1920, 1080)],
            &stranded
        ));

        // Negative coordinates (a monitor left of the primary) are honored.
        let left = WindowState { x: -1800, ..inside };
        assert!(!geometry_is_visible(&[MONITOR], &left));
        assert!(geometry_is_visible(&[(-1920, 0, 1920, 1080)], &left));
    }

    #[test]
    fn defaults_survive_a_missing_or_corrupt_file() {
        let state: UiState = serde_json::from_str("{}").expect("empty object");
        assert!(state.theme.is_none());
        assert!(state.window.is_none());
        assert!(state.editor_recents.is_empty());
        // A partial file keeps the fields it does carry.
        let partial: UiState =
            serde_json::from_str(r#"{"theme":"dark","lastView":"fleet"}"#).expect("partial");
        assert_eq!(partial.theme.as_deref(), Some("dark"));
        assert_eq!(partial.last_view.as_deref(), Some("fleet"));
    }
}
