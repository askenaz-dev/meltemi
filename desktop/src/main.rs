// SPDX-License-Identifier: Apache-2.0

//! Meltemi desktop client binary: a thin wrapper over [`meltemi_desktop::run`].

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    meltemi_desktop::run();
}
