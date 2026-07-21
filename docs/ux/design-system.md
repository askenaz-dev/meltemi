<!-- SPDX-License-Identifier: Apache-2.0 -->
# Design system — desktop surface

Tokens and rules for the Meltemi desktop client (`desktop/`, Tauri webview),
derived from brand V2 (`brand/README.md`). The same tokens are designed to be
reused by the phase-3 mobile companion without a redesign. The transversal rule
inherited from the terminal surface applies everywhere: **every state is
encoded as symbol + word; color is never the only carrier of meaning.**

## Brand palette

| Token | Value | Source |
|---|---|---|
| `--mel-aegean` | `#2563EB` | brand V2 gradient start (Aegean blue) |
| `--mel-wind` | `#22D3EE` | brand V2 gradient end (wind cyan) |

The gradient (aegean → wind, lower-left → upper-right) is reserved for brand
marks: the app icon and the onboarding lockup. UI chrome never paints the
gradient; interactive accents use the solid tokens below. Per brand rules: no
glow, no shadows-as-decoration, no circular containers around the mark.

## Color tokens

Semantic tokens as CSS custom properties; both themes ship from day one and
follow the OS, with a config override. All text/background pairs meet WCAG AA
(≥ 4.5:1 normal text, ≥ 3:1 large text and UI glyphs).

| Token | Light | Dark | Role |
|---|---|---|---|
| `--bg` | `#F8FAFC` | `#0B1220` | window background |
| `--surface` | `#FFFFFF` | `#111A2E` | cards, panels, trays |
| `--surface-2` | `#EEF2F7` | `#1A2540` | nested panels, table stripes |
| `--text` | `#0F172A` | `#E2E8F0` | primary text |
| `--text-muted` | `#475569` | `#94A3B8` | secondary text, hints |
| `--border` | `#CBD5E1` | `#2C3A57` | hairlines, dividers |
| `--accent` | `#2563EB` | `#60A5FA` | interactive, links, selection |
| `--focus` | `#1D4ED8` | `#22D3EE` | focus ring (see Focus) |
| `--ok` | `#15803D` | `#4ADE80` | success, active session |
| `--warn` | `#B45309` | `#FBBF24` | pending permission, degraded |
| `--danger` | `#B91C1C` | `#F87171` | daemon down, errors, destructive |
| `--info` | `#0E7490` | `#67E8F9` | notices, streaming activity |

## Status vocabulary

Shared with the TUI so both surfaces speak one language. Each status renders
glyph + label (localized); the glyph has an ASCII twin (TUI rule) and the
desktop uses the same shapes in its iconography.

| Status | Glyph | ASCII twin | Color token |
|---|---|---|---|
| starting / connecting | `◌` | `~` | `--info` |
| active / streaming | `▸` | `>` | `--ok` |
| waiting_permission | `●` + count + word | `!` | `--warn` |
| ended / done | `■` | `x` | `--text-muted` |
| error / unreachable | `▲` | `!` | `--danger` |

## Signal priority

Visual weight follows the daemon signal order and never inverts it:

1. **Daemon unreachable** — full-width persistent banner (`--danger`), above all content.
2. **Permission pending** — tray indicator: symbol + counter + word, always visible in the chrome.
3. **Session error / unexpected end** — inline persistent notice on the affected session.
4. **Streaming** — passive activity glyph; never a popup.

A lower-priority signal never occludes a higher one.

## Typography

- UI: `Inter, "Segoe UI", "SF Pro Text", Roboto, system-ui, sans-serif`.
  Inter is preferred, never required (the brand wordmark is outlined; no font
  dependency ships with the app).
- Code, transcripts, diffs: `"Cascadia Mono", "SF Mono", "JetBrains Mono",
  Consolas, monospace`.
- Scale (rem): 0.75 (caption) / 0.8125 (dense table) / 0.875 (body, default) /
  1.0 (section) / 1.25 (view title). Line height 1.45 body, 1.3 dense.
- Body text is never lighter than 400; dense tables use tabular numerals
  (`font-variant-numeric: tabular-nums`).

## Spacing, density, shape

- 4 px base unit; scale: 4 / 8 / 12 / 16 / 24 / 32.
- Density target is a control plane, not a marketing page: tables at 32 px
  rows, 8 px cell padding; panels at 16 px padding.
- Radii: 4 px controls, 8 px panels/cards. No pill buttons.
- Elevation: 1 px `--border` hairlines; a single soft shadow level for
  overlays only (`0 4px 16px rgb(0 0 0 / 0.18)`); nothing else floats.

## Focus and keyboard

- Focus ring: 2 px solid `--focus` with 2 px offset, on **every** focusable
  element, always visible when navigating by keyboard (`:focus-visible`).
- Focus is additionally marked by shape where rows/panels are selected
  (leading `▸` marker), so selection survives monochrome and high contrast.
- Full keyboard operation is a requirement (`gui-shell` spec); pointer-only
  affordances (hover menus without keyboard path) are forbidden.
- High contrast: honor `prefers-contrast` / forced-colors mode — in
  forced-colors, rely on system colors and the symbol+word rule.

## Motion

- Durations 120–160 ms, ease-out, opacity/transform only.
- `prefers-reduced-motion: reduce` suppresses every non-essential animation
  (spinners become static glyphs with a textual "working…" label).
- Never animate layout of the permission tray or signal banners.

## Cross-webview CSS budget

Target engines: WebView2 (Chromium), WKWebView (Safari), WebKitGTK. Keep to
the conservative intersection: flexbox, grid, custom properties, `:focus-visible`,
`prefers-*` media queries. Avoid: `backdrop-filter` as a load-bearing effect,
subgrid, scroll-driven animations, exotic selectors. Fonts, images and styles
are bundled — the webview never loads remote content (CSP).

## Iconography and app icon

- Line icons, 1.5 px stroke, rounded terminals matching the mark's geometry;
  every icon that conveys state is paired with a visible or accessible label.
- App/launcher icon: `brand/meltemi-app-icon.svg` (512 px PNG export already
  in `brand/`); tray/taskbar monochrome variants use the mono marks.

## Internationalization

- ES and EN from day one; every visible string goes through the message
  catalog (lint-enforced). Layouts tolerate +30 % text expansion; no text
  baked into images; dates/numbers via `Intl` with the app locale.
