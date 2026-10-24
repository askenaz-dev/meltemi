<!-- SPDX-License-Identifier: Apache-2.0 -->
# `mobile/` — the Meltemi companion

The Agent Boss's remote post: monitor, approve, review, direct. Governed by the
`mobile-companion` spec, which limits it to those four verbs and forbids
authoring; widening that needs a foundational amendment.

The frontend is **not here**: it is the second entry of `desktop/ui`
(`mobile.html` → `Mobile.svelte`), sharing every module below the shell. What
lives in this crate is the one thing a phone needs and a desktop does not — an
SSH client — plus the Tauri entry point.

## Building it

**Use the native Tauri CLI, not the npm one.** Install it once:

```
cargo install tauri-cli --version "^2.11" --locked
```

This is not a preference. The generated Gradle project records how the CLI was
invoked so it can call back into it, and from an npm bin shim it records
`node` with the script name as an argument — which Gradle cannot start, failing
with `A problem occurred starting process 'command '…\node.bat''` after trying
every extension. With the native CLI it records `cargo`, which works. If you ever
see that error, this is why: regenerate `gen/` with the native CLI.

Then, with the Android SDK and NDK reachable:

```
export ANDROID_HOME=…/Android/Sdk
export NDK_HOME="$ANDROID_HOME/ndk/<version>"
cargo tauri android init                        # regenerates gen/
cargo tauri android build --debug --target x86_64   # for the emulator
cargo tauri android build --target aarch64          # the release ABI
```

`gen/` is **generated and gitignored**. It is a build output that drifts from its
source, and the next `init` would fight the diff.

## Signing

Release signing is the maintainer's, with their own keystore, like the rest of
the distribution. No key, keystore or password belongs in this tree — the
`.gitignore` says so and means it.
