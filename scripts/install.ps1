# SPDX-License-Identifier: Apache-2.0
#
# Meltemi installer (Windows, PowerShell). Short and auditable on purpose: read
# it before running it, and verify its published hash. It verifies the release
# checksum and refuses on a mismatch — no blind execution.
#
# Manual equivalent (if you prefer not to run this):
#   1. Download meltemi-Windows.zip and SHA256SUMS from the release page.
#   2. Verify: Get-FileHash meltemi-Windows.zip -Algorithm SHA256 against SHA256SUMS.
#   3. Verify the signature of SHA256SUMS with the published signing key.
#   4. Extract meltemi.exe, meltemid.exe and the two meltemi-*-acp.exe adapters
#      into a directory on your PATH. Keep the adapters BESIDE the daemon: that
#      is where it looks for them.
#   5. Create the alias: a `mel.cmd` shim that calls meltemi.exe.
#
# Usage:
#   $env:MELTEMI_VERSION = "v0.1.0"; ./install.ps1 [-InstallDir <dir>]
# Default install dir: $env:LOCALAPPDATA\Programs\meltemi

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\meltemi"
)
$ErrorActionPreference = "Stop"

$version = if ($env:MELTEMI_VERSION) { $env:MELTEMI_VERSION } else { "latest" }
# Canonical download base — declared once in docs/release.md and verified by
# the site lint; override only for a local mirror while testing.
$baseUrl = if ($env:MELTEMI_BASE_URL) { $env:MELTEMI_BASE_URL } else { "https://github.com/askenaz-dev/meltemi/releases" }
# The two shapes the host serves: the version-free redirector for the latest
# release, and the tagged path for a pinned one. `latest` is NOT a tag, so
# asking for `download/latest/<asset>` is a 404 — the mistake this guards.
$assetBase = if ($version -eq "latest") { "$baseUrl/latest/download" } else { "$baseUrl/download/$version" }
$asset = "meltemi-Windows.zip"

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("meltemi-" + [System.Guid]::NewGuid()))
try {
    Write-Host "Downloading $asset ($version)..."
    Invoke-WebRequest -Uri "$assetBase/$asset" -OutFile (Join-Path $tmp $asset)
    Invoke-WebRequest -Uri "$assetBase/SHA256SUMS" -OutFile (Join-Path $tmp "SHA256SUMS")

    Write-Host "Verifying checksum..."
    $expected = (Select-String -Path (Join-Path $tmp "SHA256SUMS") -Pattern ([regex]::Escape($asset))).Line.Split(" ")[0].Trim()
    $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected.ToLower()) {
        throw "checksum mismatch: refusing to install"
    }

    Write-Host "Installing to $InstallDir..."
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    # Everything in the archive lands in one directory, which is what the daemon
    # expects: it probes its own directory for the ACP adapters that ship with it.
    Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $InstallDir -Force
    foreach ($required in @("meltemi.exe", "meltemid.exe", "meltemi-claude-acp.exe", "meltemi-codex-acp.exe")) {
        if (-not (Test-Path (Join-Path $InstallDir $required))) {
            throw "the archive did not contain $required : refusing a partial installation"
        }
    }

    # The short alias `mel` -> meltemi (a .cmd shim on the PATH).
    $shim = "@echo off`r`n`"%~dp0meltemi.exe`" %*`r`n"
    Set-Content -Path (Join-Path $InstallDir "mel.cmd") -Value $shim -NoNewline

    Write-Host "Installed in ${InstallDir}: meltemi.exe, meltemid.exe, the alias 'mel',"
    Write-Host "and the ACP adapters meltemi-claude-acp.exe and meltemi-codex-acp.exe."
    Write-Host "Ensure $InstallDir is on your PATH."
}
finally {
    Remove-Item -Recurse -Force $tmp
}
