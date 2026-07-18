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
#   4. Extract meltemi.exe and meltemid.exe into a directory on your PATH.
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
$baseUrl = if ($env:MELTEMI_BASE_URL) { $env:MELTEMI_BASE_URL } else { "https://github.com/meltemi/meltemi/releases/download" }
$asset = "meltemi-Windows.zip"

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("meltemi-" + [System.Guid]::NewGuid()))
try {
    Write-Host "Downloading $asset ($version)..."
    Invoke-WebRequest -Uri "$baseUrl/$version/$asset" -OutFile (Join-Path $tmp $asset)
    Invoke-WebRequest -Uri "$baseUrl/$version/SHA256SUMS" -OutFile (Join-Path $tmp "SHA256SUMS")

    Write-Host "Verifying checksum..."
    $expected = (Select-String -Path (Join-Path $tmp "SHA256SUMS") -Pattern ([regex]::Escape($asset))).Line.Split(" ")[0].Trim()
    $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected.ToLower()) {
        throw "checksum mismatch: refusing to install"
    }

    Write-Host "Installing to $InstallDir..."
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path (Join-Path $tmp $asset) -DestinationPath $InstallDir -Force

    # The short alias `mel` -> meltemi (a .cmd shim on the PATH).
    $shim = "@echo off`r`n`"%~dp0meltemi.exe`" %*`r`n"
    Set-Content -Path (Join-Path $InstallDir "mel.cmd") -Value $shim -NoNewline

    Write-Host "Installed: meltemi.exe, meltemid.exe, and the alias 'mel' in $InstallDir"
    Write-Host "Ensure $InstallDir is on your PATH."
}
finally {
    Remove-Item -Recurse -Force $tmp
}
