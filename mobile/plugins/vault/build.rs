// SPDX-License-Identifier: Apache-2.0

// No webview-facing commands: the host calls the Kotlin side from Rust, so
// there is nothing for a capability to grant. What this build hands the app's
// build is the path of the Android library under `android/`.
fn main() {
    tauri_plugin::Builder::new(&[])
        .android_path("android")
        .build();
}
