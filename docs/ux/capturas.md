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

## Desktop surface — manual, on a release machine

The desktop capture needs a running window, so it is a maintainer action:

1. Create a temporary fixture repository outside this one, and point it at the
   simulated agent:

   ```bash
   mkdir -p /tmp/meltemi-fixture/.meltemi
   cd /tmp/meltemi-fixture && git init -q
   printf '[agent]\ncommand = ["%s"]\n' "$(pwd)/../mock-agent" > .meltemi/config.toml
   printf '[[rule]]\neffect = "allow"\n' > .meltemi/permissions.toml
   ```

   Build `mock-agent` from this repository (`cargo build --release -p mock-agent`)
   and use that path. No real agent, no network.

2. Launch the desktop client with the fixture as its working directory, run one
   session so the views have content, and set the theme you want to publish.

3. Capture the window at 2× scale, PNG, no window shadow, and crop to the
   application frame. Nothing else may be on screen.

4. Check the image before committing it: no user name in a path, no real project
   name, no third-party product name, no e-mail, no token.

5. Save it as `site/media/desktop-sessions.png` and add the figure to both
   language trees with alt text and a caption declaring surface, fixture origin,
   product version and platform. The lint requires both.

## Refreshing

Captures age against the interface. Refresh them as part of a release when the
surface they show has changed; a caption that names a version the product left
behind is worse than no caption.
