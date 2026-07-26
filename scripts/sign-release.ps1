# SPDX-License-Identifier: Apache-2.0
#
# Signs a Meltemi release's SHA256SUMS with minisign and uploads the signature
# (docs/release.md, "Signing a release"). Meant to run on the maintainer's own
# machine, never in CI: the private key is read from a local file and minisign
# always prompts for its passphrase interactively — this script never sees, asks
# for, or stores that passphrase itself.
#
# Manual equivalent (if you prefer not to run this):
#   gh release download vX.Y.Z --pattern SHA256SUMS --dir .
#   minisign -Sm SHA256SUMS -s meltemi.key
#   minisign -Vm SHA256SUMS -p meltemi.pub
#   gh release upload vX.Y.Z SHA256SUMS.minisig
#   gh release edit vX.Y.Z --draft=false
#
# Usage:
#   ./sign-release.ps1 -Tag v0.1.0
#   ./sign-release.ps1                                    # signs the latest release
#   ./sign-release.ps1 -Tag v0.1.0 -SecretKey D:\keys\meltemi.key -PublicKey D:\keys\meltemi.pub

param(
    [string]$Tag,
    [string]$Repo = "askenaz-dev/meltemi",
    [string]$SecretKey = "$HOME\keys\meltemi\meltemi.key",
    [string]$PublicKey = "$HOME\keys\meltemi\meltemi.pub"
)

$ErrorActionPreference = "Stop"

foreach ($tool in "gh", "minisign") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool is not on PATH. Install it first (winget install jedisct1.minisign)."
    }
}
if (-not (Test-Path $SecretKey)) {
    throw "Secret key not found at $SecretKey. Pass -SecretKey <path>."
}
if (-not (Test-Path $PublicKey)) {
    throw "Public key not found at $PublicKey. Pass -PublicKey <path>."
}

if (-not $Tag) {
    $Tag = gh release list --repo $Repo --limit 1 --json tagName --jq ".[0].tagName"
    if (-not $Tag) { throw "No releases found in $Repo; pass -Tag explicitly." }
    Write-Host "No -Tag given; using the latest release: $Tag"
}

# A throwaway directory, never the repo: SHA256SUMS and its signature are
# release assets, not files this project tracks.
$work = Join-Path $env:TEMP "meltemi-sign-$Tag"
if (Test-Path $work) { Remove-Item -Recurse -Force $work }
New-Item -ItemType Directory -Path $work | Out-Null

Push-Location $work
try {
    Write-Host "Downloading SHA256SUMS for ${Tag}..."
    gh release download $Tag --repo $Repo --pattern SHA256SUMS --clobber
    if ($LASTEXITCODE -ne 0) { throw "gh release download failed" }

    # The only step that touches the private key. minisign reads it from disk
    # and asks for its passphrase on the terminal — nothing here supplies or
    # captures that passphrase.
    Write-Host "Signing with $SecretKey — enter its passphrase when asked:"
    minisign -Sm SHA256SUMS -s $SecretKey -t "Meltemi $Tag release checksums"
    if ($LASTEXITCODE -ne 0) { throw "minisign signing failed" }

    # Verify before anything gets uploaded: catches a wrong key or a corrupted
    # download before either becomes a public, hard-to-take-back mistake.
    Write-Host "Verifying the signature just produced..."
    minisign -Vm SHA256SUMS -p $PublicKey
    if ($LASTEXITCODE -ne 0) { throw "signature verification failed — nothing was uploaded" }

    Write-Host "Uploading SHA256SUMS.minisig..."
    gh release upload $Tag SHA256SUMS.minisig --repo $Repo --clobber
    if ($LASTEXITCODE -ne 0) { throw "gh release upload failed" }

    Write-Host ""
    Write-Host "Signature published for $Tag."

    # Publishing is the one step this script does not do on its own: it is the
    # most visible, least reversible action in the whole procedure, so it stays
    # a decision you make each time, not a default a script reaches for you.
    $isDraft = gh release view $Tag --repo $Repo --json isDraft --jq ".isDraft"
    if ($isDraft -eq "true") {
        $answer = Read-Host "Release $Tag is still a draft. Publish it now? [y/N]"
        if ($answer -match "^[Yy]") {
            gh release edit $Tag --repo $Repo --draft=false
            Write-Host "Published."
        }
        else {
            Write-Host "Left as a draft. Publish later with:"
            Write-Host "  gh release edit $Tag --repo $Repo --draft=false"
        }
    }
}
finally {
    Pop-Location
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
