<!-- SPDX-License-Identifier: Apache-2.0 -->
# `mobile/` — the Meltemi companion

The Agent Boss's remote post: monitor, approve, review, direct. Governed by the
`mobile-companion` spec, which limits it to those four verbs and forbids
authoring; widening that needs a foundational amendment.

The frontend is **not here**: it is the second entry of `desktop/ui`
(`mobile.html` → `Mobile.svelte`), sharing every module below the shell. What
lives in this crate is what a phone needs and a desktop does not — an SSH
client (`ssh.rs`), a key of its own (`keys.rs`) sealed by Android's keystore
through the plugin in `plugins/vault/`, the dialer the shared bridge actor
calls (`dial.rs`), the files it keeps between runs (`store.rs`) — plus the
Tauri host (`host.rs`): the desktop's `daemon_request`/`daemon_notify` door
and the pairing commands that are the companion's own.

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

The Android-only half of the Rust (`cfg(target_os = "android")`: the keystore
calls) is type-checked without a device by pointing clippy at the target, with
the NDK's clang for `ring`'s C:

```
export CC_x86_64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/x86_64-linux-android34-clang"
export AR_x86_64_linux_android="$NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/llvm-ar"
cargo clippy -p meltemi-mobile -p tauri-plugin-meltemi-vault --target x86_64-linux-android -- -D warnings
```

## Trying it against a daemon, without touching your machine's `sshd`

`core/mock-sshd` is a test SSH server that accepts exactly one public key and
runs exactly one command, `meltemi bridge`, with the binary and environment it
is handed. It is what the channel e2e uses (`tests/channel.rs`, which needs
`cargo build --workspace` first, as CI does), and it is how the app on an
emulator reaches a real daemon on this machine:

1. Start a daemon on an endpoint of its own (never your own one):
   `MELTEMI_ENDPOINT=… MELTEMI_CONFIG_DIR=… MELTEMI_DATA_DIR=… MELTEMI_START_HERE=1 meltemi status`.
2. Install and launch the app on the emulator. Its first screen shows the
   app's key line; copy it into a file.
3. `mock-sshd --listen 127.0.0.1:2222 --authorized <that file> --bridge <path to meltemi> --env MELTEMI_ENDPOINT=…`
4. In the app, the destination is `anyone@10.0.2.2:2222` — `10.0.2.2` is the
   emulator's name for the host — and Connect.

Starting `mock-sshd` again makes a new host key, which the app must refuse as a
reinstalled machine until you forget the host on purpose; that is a feature of
the fixture, not an accident.

## Signing

Release signing is the maintainer's, with their own keystore, like the rest of
the distribution. No key, keystore or password belongs in this tree — the
`.gitignore` says so and means it.
