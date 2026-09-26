# SPDX-License-Identifier: Apache-2.0
<#
.SYNOPSIS
  The integrated run of Meltemi against the agent CLIs installed on this
  machine (corrida-integrada).

.DESCRIPTION
  Manual and opt-in: every real agent it drives spends turns on the user's own
  subscription, so it never runs in CI and never runs by itself.

  Isolated: the daemon it talks to is its own — its endpoint, data and
  configuration live in a temporary directory (MELTEMI_ENDPOINT,
  MELTEMI_DATA_DIR, MELTEMI_CONFIG_DIR, MELTEMI_DAEMON_BIN, and
  MELTEMI_START_HERE to ask for a start at that endpoint). The user's daemon,
  data and configuration are never read or touched.

  Over a temporary fixture repository, every ready agent goes through the same
  seven verbs: a reading session, a writing session with no rule (it must be
  denied), a writing session with the fixture's rule, propose, dispatch in a
  managed worktree, direct, and cancel. What it reports is what the product
  recorded — exit codes, statuses, permissions and who decided them, empty
  turns, the usage the agent reported — and the wall-clock time. Nothing is
  estimated. Agents of the catalog that are not installed are listed as not
  measured.

.PARAMETER Agents
  Catalog ids to run. By default, every agent the fleet reports ready.

.PARAMETER Mock
  Rehearse against the scripted mock agent: no real turn is spent.

.PARAMETER Report
  Where the report is written. By default docs/qa/<date>-corrida-integrada.md
  (or a temporary file when rehearsing with -Mock).

.PARAMETER BinDir
  Where meltemi, meltemid and the adapters are. By default target\debug of
  this checkout, built with `cargo build --workspace`.

.PARAMETER Timeout
  Seconds any one command may take before it is stopped.
#>
[CmdletBinding()]
param(
  [string[]]$Agents = @(),
  [switch]$Mock,
  [string]$Report = '',
  [string]$BinDir = '',
  [int]$Timeout = 300
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $BinDir) { $BinDir = Join-Path $repo 'target\debug' }
$meltemi = Join-Path $BinDir 'meltemi.exe'
$meltemid = Join-Path $BinDir 'meltemid.exe'
foreach ($needed in @($meltemi, $meltemid)) {
  if (-not (Test-Path $needed)) { throw "not found: $needed — run ``cargo build --workspace`` first" }
}

# ---- the isolated environment ------------------------------------------------
# Five variables, all pointing at this run's own directory: nothing here can
# reach the user's daemon, data or configuration.

$stamp = (Get-Date).ToUniversalTime()
$base = Join-Path ([System.IO.Path]::GetTempPath()) "meltemi-smoke-agents-$PID"
$fixture = Join-Path $base 'fixture'
$runDir = Join-Path $base 'out'
New-Item -ItemType Directory -Force -Path $fixture, $runDir, (Join-Path $base 'data'), (Join-Path $base 'config') | Out-Null
$env:MELTEMI_ENDPOINT = "\\.\pipe\meltemi-smoke-agents-$PID"
$env:MELTEMI_DATA_DIR = Join-Path $base 'data'
$env:MELTEMI_CONFIG_DIR = Join-Path $base 'config'
$env:MELTEMI_DAEMON_BIN = $meltemid
$env:MELTEMI_START_HERE = '1'
Remove-Item Env:\MELTEMI_AGENT_COMMAND -ErrorAction SilentlyContinue

# ---- running one command -------------------------------------------------------
# Output goes to files, never to a pipe: a `meltemi` in a pipeline hands the
# daemon it starts the pipe's handle, and the pipeline never ends.

$script:step = 0
function Invoke-Meltemi([string[]]$Argv, [int]$Seconds = $Timeout, [switch]$NoWait) {
  $script:step += 1
  $out = Join-Path $runDir ("{0:d3}.out" -f $script:step)
  $err = Join-Path $runDir ("{0:d3}.err" -f $script:step)
  # Start-Process joins its arguments with spaces and quotes none of them.
  $quoted = foreach ($arg in $Argv) {
    if ($arg -match '[\s"]') { '"' + ($arg -replace '"', '\"') + '"' } else { $arg }
  }
  $clock = [System.Diagnostics.Stopwatch]::StartNew()
  $process = Start-Process -FilePath $meltemi -ArgumentList $quoted -WorkingDirectory $fixture `
    -NoNewWindow -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
  if ($NoWait) { return [pscustomobject]@{ Process = $process; Clock = $clock; OutFile = $out; ErrFile = $err } }
  $finished = $process.WaitForExit($Seconds * 1000)
  if (-not $finished) { try { $process.Kill() } catch {} }
  $clock.Stop()
  [pscustomobject]@{
    Exit     = if ($finished) { $process.ExitCode } else { $null }
    Ms       = $clock.ElapsedMilliseconds
    Out      = (Get-Content $out -Raw -ErrorAction SilentlyContinue)
    Err      = (Get-Content $err -Raw -ErrorAction SilentlyContinue)
    TimedOut = -not $finished
  }
}

function Complete-Meltemi($Started, [int]$Seconds = $Timeout) {
  $finished = $Started.Process.WaitForExit($Seconds * 1000)
  if (-not $finished) { try { $Started.Process.Kill() } catch {} }
  $Started.Clock.Stop()
  [pscustomobject]@{
    Exit     = if ($finished) { $Started.Process.ExitCode } else { $null }
    Ms       = $Started.Clock.ElapsedMilliseconds
    Out      = (Get-Content $Started.OutFile -Raw -ErrorAction SilentlyContinue)
    Err      = (Get-Content $Started.ErrFile -Raw -ErrorAction SilentlyContinue)
    TimedOut = -not $finished
  }
}

function Read-Json($Result) {
  if (-not $Result.Out) { return $null }
  try { return ($Result.Out | ConvertFrom-Json) } catch { return $null }
}

# What the session's own record says: permissions asked and who decided them,
# empty turns, and the usage the agent reported over the protocol.
function Get-SessionFacts([string]$SessionId) {
  $facts = [ordered]@{ Asked = 0; ByRule = 0; ByClient = 0; Denied = 0; Empty = 0; Cost = ''; Context = '' }
  if (-not $SessionId) { return [pscustomobject]$facts }
  $log = Invoke-Meltemi @('--json', 'log', $SessionId) 60
  foreach ($line in ($log.Out -split "`n")) {
    if (-not $line.Trim()) { continue }
    try { $record = $line | ConvertFrom-Json } catch { continue }
    switch ($record.type) {
      'permission_requested' { $facts.Asked += 1 }
      'permission_decided' {
        if ($record.payload.decidedBy -eq 'rule') { $facts.ByRule += 1 }
        if ($record.payload.decidedBy -eq 'client') { $facts.ByClient += 1 }
        if ($record.payload.denied -eq $true) { $facts.Denied += 1 }
      }
      'turn_empty' { $facts.Empty += 1 }
      'agent_update' {
        $update = $record.payload.update
        if ($update.sessionUpdate -eq 'usage_update') {
          if ($update.cost) { $facts.Cost = "$($update.cost.amount) $($update.cost.currency)" }
          if ($update.size) { $facts.Context = "$($update.used)/$($update.size)" }
        }
      }
    }
  }
  [pscustomobject]$facts
}

# ---- the fixture ---------------------------------------------------------------

Set-Content -Path (Join-Path $fixture 'README.md') -Value "# harbour`nA tiny fixture for the integrated run.`nIt greets people.`n"
New-Item -ItemType Directory -Force -Path (Join-Path $fixture 'src'), (Join-Path $fixture '.meltemi\changes\greeting') | Out-Null
Set-Content -Path (Join-Path $fixture 'src\greet.py') -Value "def greet(name):`n    return f`"hello {name}`"`n"
Set-Content -Path (Join-Path $fixture '.meltemi\changes\greeting\proposal.md') -Value "# greeting`n`n## Why`n`nPeople leave as well as arrive.`n`n## What Changes`n`n- A farewell next to the greeting.`n"
Set-Content -Path (Join-Path $fixture '.meltemi\changes\greeting\tasks.md') -Value "## 1. Farewell`n`n- [ ] 1.1 Add a function ``farewell(name)`` to src/greet.py that returns ``goodbye {name}```n"
if ($Mock) {
  $mockBin = (Join-Path $BinDir 'mock-agent.exe') -replace '\\', '/'
  Set-Content -Path (Join-Path $fixture '.meltemi\config.toml') -Value "[agent]`ncommand = ['$mockBin', '--turn-delay-ms', '1500', '--honor-cancel', '--titled-permission']`n"
}
git -C $fixture init -q
git -C $fixture config core.autocrlf false
git -C $fixture config user.email 'smoke@meltemi.test'
git -C $fixture config user.name 'Meltemi Smoke'
git -C $fixture config commit.gpgsign false
git -C $fixture add -A
git -C $fixture commit -q -m 'fixture'

# The rule the second phase runs under: reading and editing inside the fixture,
# in both spellings of its path, and nothing else. Running commands stays
# without a rule, and without a human it is denied.
$ruled = @"
[[rule]]
effect = "allow"
tool = "read"
path-prefix = '$fixture'

[[rule]]
effect = "allow"
tool = "read"
path-prefix = '$($fixture -replace '\\', '/')'

[[rule]]
effect = "allow"
tool = "edit"
path-prefix = '$fixture'

[[rule]]
effect = "allow"
tool = "edit"
path-prefix = '$($fixture -replace '\\', '/')'
"@

# ---- who runs ------------------------------------------------------------------

$version = (Invoke-Meltemi @('--version') 30).Out.Trim()
$fleet = Read-Json (Invoke-Meltemi @('--json', 'fleet') 60)
$catalog = @($fleet.agents)
$versions = @{}
if ($Mock) {
  $ready = @('mock')
  $versions['mock'] = 'mock-agent (scripted)'
} else {
  $ready = @($catalog | Where-Object { $_.installState -eq 'ready' } | ForEach-Object { $_.id })
  if ($Agents.Count -gt 0) { $ready = @($ready | Where-Object { $Agents -contains $_ }) }
  foreach ($entry in $catalog | Where-Object { $ready -contains $_.id }) {
    $cli = @($entry.layers | Where-Object { $_.kind -eq 'cli' })[0]
    $versions[$entry.id] = try { (& $cli.binaryPath --version 2>&1 | Select-Object -First 1).ToString().Trim() } catch { 'unknown' }
  }
}
$notInstalled = @($catalog | Where-Object { $_.installState -ne 'ready' })
Write-Host "$version · agents: $($ready -join ', ') · fixture: $fixture"

# ---- the matrix ----------------------------------------------------------------

$rows = @{}
foreach ($id in $ready) { $rows[$id] = [System.Collections.Generic.List[object]]::new() }
function Add-Row([string]$Id, [string]$Verb, $Result, [string]$Verdict, $Facts, [string]$Note = '') {
  $rows[$Id].Add([pscustomobject]@{
    Verb = $Verb; Verdict = $Verdict; Exit = $Result.Exit; Seconds = [math]::Round($Result.Ms / 1000, 1)
    Facts = $Facts; Note = $Note
  })
  Write-Host ("  {0,-12} {1,-22} {2} ({3} s, exit {4})" -f $Id, $Verb, $Verdict, [math]::Round($Result.Ms / 1000, 1), $Result.Exit)
}
function Agent-Args([string]$Id) { if ($Mock) { @() } else { @('--agent', $Id) } }

$readSessions = @{}
# Phase 1, with no rule: a reading session, and a write that must be denied.
foreach ($id in $ready) {
  $r = Invoke-Meltemi (@('--json', 'session', 'Read README.md and reply with its first line only. Change nothing.', $fixture) + (Agent-Args $id))
  $j = Read-Json $r
  $readSessions[$id] = $j.sessionId
  $verdict = if ($r.Exit -eq 0 -and $j.status -eq 'completed') { 'completed' } else { "exit $($r.Exit)" }
  Add-Row $id 'session (read)' $r $verdict (Get-SessionFacts $j.sessionId)

  $file = "NOTES-$id-unruled.md"
  $r = Invoke-Meltemi (@('--json', 'session', "Create a file named $file containing the single line: hello. Do nothing else.", $fixture) + (Agent-Args $id))
  $j = Read-Json $r
  $written = Test-Path (Join-Path $fixture $file)
  $verdict = if (-not $written) { 'not written (denied)' } else { 'WRITTEN without a rule' }
  Add-Row $id 'session (write, no rule)' $r $verdict (Get-SessionFacts $j.sessionId)
}

Set-Content -Path (Join-Path $fixture '.meltemi\permissions.toml') -Value $ruled

# Phase 2, under the fixture's rule.
foreach ($id in $ready) {
  $file = "NOTES-$id-ruled.md"
  $r = Invoke-Meltemi (@('--json', 'session', "Create a file named $file containing the single line: hello. Do nothing else.", $fixture) + (Agent-Args $id))
  $j = Read-Json $r
  $written = Test-Path (Join-Path $fixture $file)
  $verdict = if ($written) { 'written by rule' } else { 'not written' }
  Add-Row $id 'session (write, rule)' $r $verdict (Get-SessionFacts $j.sessionId)

  # `propose` answers with the change, not the session: the session is the one
  # the fixture did not have before.
  $before = @((Read-Json (Invoke-Meltemi @('--json', 'sessions', $fixture) 60)).sessions | ForEach-Object { $_.sessionId })
  $r = Invoke-Meltemi (@('--json', 'propose', "add a farewell function next to greet, drafted by $id", $fixture) + (Agent-Args $id))
  $j = Read-Json $r
  $proposed = @((Read-Json (Invoke-Meltemi @('--json', 'sessions', $fixture) 60)).sessions |
      Where-Object { $before -notcontains $_.sessionId } | ForEach-Object { $_.sessionId })[0]
  $verdict = if ($r.Exit -eq 0) { "$($j.status)" } else { "exit $($r.Exit)" }
  Add-Row $id 'propose' $r $verdict (Get-SessionFacts $proposed) "$($j.changeName)"

  $assigned = Invoke-Meltemi @('--json', 'assign', 'greeting', '1.1', $id, $fixture) 120
  $r = Invoke-Meltemi @('--json', 'dispatch', 'greeting', '1.1', $id, $fixture)
  $j = Read-Json $r
  $verdict = if ($r.Exit -eq 0) { "$($j.status)" } else { "exit $($r.Exit)" }
  $note = if ($assigned.Exit -ne 0) { "assign exit $($assigned.Exit)" } elseif ($j.committed) { "commit $($j.sha)" } else { 'no commit' }
  Add-Row $id 'dispatch' $r $verdict (Get-SessionFacts $j.sessionId) $note

  $r = Invoke-Meltemi @('--json', 'direct', "$($readSessions[$id])", 'Reply with the second line of README.md only.', $fixture)
  $j = Read-Json $r
  $verdict = if ($r.Exit -eq 0) { "$($j.disposition)" } else { "exit $($r.Exit)" }
  $note = if ($r.Exit -ne 0) { "$((Read-Json $r).error.message)" } else { '' }
  if ($r.Exit -ne 0 -and -not $note) { $note = ($r.Out + $r.Err) -replace '\s+', ' ' }
  if ($note.Length -gt 160) { $note = $note.Substring(0, 160) + '…' }
  Add-Row $id 'direct' $r $verdict (Get-SessionFacts $j.sessionId) $note

  # A long turn, cancelled from another command as soon as it is live.
  $known = @((Read-Json (Invoke-Meltemi @('--json', 'sessions', $fixture) 60)).sessions | ForEach-Object { $_.sessionId })
  $started = Invoke-Meltemi (@('--json', 'session', 'Write a long essay about lighthouses, at least twenty paragraphs.', $fixture) + (Agent-Args $id)) -NoWait
  $live = $null
  $deadline = (Get-Date).AddSeconds(60)
  while (-not $live -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 300
    $listed = Read-Json (Invoke-Meltemi @('--json', 'status') 30)
    $live = @($listed.sessions | Where-Object { $known -notcontains $_.sessionId })[0].sessionId
  }
  $cancel = if ($live) { Invoke-Meltemi @('--json', 'cancel', $live) 60 } else { $null }
  $r = Complete-Meltemi $started
  $verdict = if (-not $live) { 'never live' } elseif ($cancel.Exit -eq 0) { "cancelled ($((Read-Json $r).status))" } else { "cancel exit $($cancel.Exit)" }
  Add-Row $id 'cancel' $r $verdict (Get-SessionFacts $live)
}

# ---- the report ----------------------------------------------------------------

$commit = (git -C $repo rev-parse --short HEAD).Trim()
$os = (Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue).Caption
if (-not $Report) {
  $Report = if ($Mock) { Join-Path $base 'report.md' } else { Join-Path $repo ("docs\qa\{0:yyyy-MM-dd}-corrida-integrada.md" -f $stamp) }
}
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add('<!-- SPDX-License-Identifier: Apache-2.0 -->')
$lines.Add("# Corrida integrada — $('{0:yyyy-MM-dd}' -f $stamp)")
$lines.Add('')
$lines.Add("Escrita por ``scripts/smoke-agents.ps1`` el $('{0:yyyy-MM-dd HH:mm}' -f $stamp) UTC, sobre ``$version`` (commit ``$commit``), en $os. Daemon, datos y configuración aislados; un repositorio fixture temporal.")
$lines.Add('')
$lines.Add('## Agentes')
$lines.Add('')
$lines.Add('| Agente | Versión |')
$lines.Add('| --- | --- |')
foreach ($id in $ready) { $lines.Add("| ``$id`` | $($versions[$id]) |") }
foreach ($entry in $notInstalled) { $lines.Add("| ``$($entry.id)`` | **no medido**: no instalado |") }
$lines.Add('')
$lines.Add('## La matriz')
$lines.Add('')
$lines.Add('Permisos: pedidos · decididos por regla · por el cliente · denegados. Uso: el que el agente reportó por el protocolo (costo, contexto usado/tamaño), vacío si no reportó.')
foreach ($id in $ready) {
  $lines.Add('')
  $lines.Add("### ``$id``")
  $lines.Add('')
  $lines.Add('| Verbo | Resultado | Segundos | Permisos | Turnos vacíos | Uso | Nota |')
  $lines.Add('| --- | --- | --- | --- | --- | --- | --- |')
  foreach ($row in $rows[$id]) {
    $f = $row.Facts
    $usage = (@($f.Cost, $f.Context) | Where-Object { $_ }) -join ' · '
    $lines.Add("| $($row.Verb) | $($row.Verdict) | $($row.Seconds) | $($f.Asked) · $($f.ByRule) · $($f.ByClient) · $($f.Denied) | $($f.Empty) | $usage | $($row.Note -replace '\|', '/') |")
  }
}
$lines.Add('')
$lines.Add('## Hallazgos')
$lines.Add('')
$lines.Add('_Los escribe quien corre: qué no se esperaba, y la change que lo aborda._')
Set-Content -Path $Report -Value ($lines -join "`n") -Encoding utf8
Write-Host "report: $Report"

# ---- teardown: only what this run started ---------------------------------------

Invoke-Meltemi @('stop') 30 | Out-Null
Start-Sleep -Milliseconds 800
Get-Process -Name meltemid -ErrorAction SilentlyContinue |
  Where-Object { $_.Path -eq $meltemid } |
  Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "fixture and logs kept for inspection: $base"
