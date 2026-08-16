# SPDX-License-Identifier: Apache-2.0
#
# Measures what N parked sessions actually cost: resident memory of the daemon
# and of the agent subprocesses it is holding open.
#
# The risk `sesion-que-espera` declared is that an idle session is a live agent
# process, and a number nobody measured is a number nobody can defend. This is
# the measurement, run by hand against release binaries.
#
#   pwsh scripts/measure-idle-sessions.ps1 -Sessions 5
#
param(
    [int]$Sessions = 5,
    [string]$Endpoint = "\.\pipe\meltemid-idle-probe"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$daemon = Join-Path $root "target/release/meltemid.exe"
$mock = Join-Path $root "target/release/mock-agent.exe"
foreach ($bin in @($daemon, $mock)) {
    if (-not (Test-Path $bin)) {
        throw "missing $bin — build with: cargo build --release --workspace"
    }
}

# A throwaway project pointed at the mock, with waiting switched on long enough
# to observe and permissions allowed so no turn stalls on a human.
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) "meltemi-idle-probe"
Remove-Item -Recurse -Force $fixture -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $fixture ".meltemi") | Out-Null
$mockPath = $mock -replace "\\", "/"
@"
[agent]
command = ['$mockPath']

[sessions]
idle-timeout = 600
max-idle = $Sessions
"@ | Set-Content (Join-Path $fixture ".meltemi/config.toml")
"[[rule]]`neffect = `"allow`"" | Set-Content (Join-Path $fixture ".meltemi/permissions.toml")

Write-Host "Start $Sessions detached sessions against $fixture, then press Enter."
Write-Host "  (from another shell: meltemi session ... — or drive session/start with detach:true)"
Read-Host | Out-Null

$procs = Get-Process -Name "meltemid", "mock-agent" -ErrorAction SilentlyContinue
$procs | Select-Object Name, Id, @{ n = "WorkingSetMB"; e = { [math]::Round($_.WorkingSet64 / 1MB, 1) } } |
    Format-Table -AutoSize
$total = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
Write-Host ("total resident: {0:N1} MB across {1} processes" -f ($total / 1MB), $procs.Count)
Write-Host ("per waiting session: {0:N1} MB" -f (($total / 1MB) / [math]::Max($Sessions, 1)))
