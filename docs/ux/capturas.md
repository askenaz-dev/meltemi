<!-- SPDX-License-Identifier: Apache-2.0 -->
# Capture procedure for the product site

The site's figures must be **real captures of the real surfaces**, taken over a
temporary fixture repository driven by the simulated agent — never over a real
project and never under a real agent account (`sitio-web-producto` design D5).
No image may leak a personal path, an identity, a third-party product name or
any credential material.

Every figure carries two things the lint enforces: descriptive alternative text
and its provenance (surface, capture source and product state). What is inside
the pixels is the human's judgement; the lint cannot inspect a raster.

## Terminal surface — automated, reproducible

The terminal capture is generated from the renderer itself, so it can never
drift into a mockup:

```bash
cargo run -p meltemi --example capture_svg > site/media/tui-sessions.svg
```

The example builds a fixture session set (two generic project roots, the
simulated agent, two launch profiles) and renders it through the same code path
the shell uses. Regenerate it whenever the sessions view changes; the diff is
reviewable because the output is text.

## Desktop surface — scripted, run by a maintainer

The desktop capture needs a running window, so it is a maintainer action rather
than a CI step. It is still scripted, so the provenance in the caption is a fact
anybody can re-derive:

```powershell
cargo build --release -p mock-agent -p meltemi -p meltemid
cd desktop && ui/node_modules/.bin/tauri build --no-bundle && cd ..
pwsh -NoProfile -File scripts/capture-desktop.ps1
```

The script builds a throwaway fixture repository with two launch profiles over
the simulated agent, dispatches four tasks and one proposal so the views have
content, seeds the window geometry for the machine's display scale, captures the
window with `PrintWindow`/`PW_RENDERFULLCONTENT` (a WebView2 surface comes out
blank without it), downscales to 1600 px wide and writes
`site/media/desktop-sessions.png`. Its endpoint, data directory and config
directory are its own: it never touches the maintainer's daemon, projects or
agents. No real agent, no network.

Only the platform differs — it is PowerShell because the surface it captures is
a Windows window, and Windows is a first-class platform here. On macOS and Linux
the equivalent is a manual capture of the same fixture; the caption must then
name that platform.

Before committing the image, check it: no user name in a path, no real project
name, no third-party product name, no e-mail, no token. Then add the figure to
both language trees with alt text and a caption declaring surface, fixture
origin, product version and platform. The lint requires both.

## Refreshing

Captures age against the interface. Refresh them as part of a release when the
surface they show has changed; a caption that names a version the product left
behind is worse than no caption.
