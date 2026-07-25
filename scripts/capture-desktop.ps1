# SPDX-License-Identifier: Apache-2.0
#
# Captures the desktop surface for the product site (docs/ux/capturas.md).
#
# Everything it touches is a throwaway: a fixture repository, its own endpoint,
# its own data and config directories. It never talks to the maintainer's daemon,
# never opens a real project and never runs a real agent — the sessions in the
# picture are driven by `mock-agent` from this workspace.
#
# Prerequisites: `cargo build --release -p mock-agent -p meltemi -p meltemid` and
# a desktop binary built by Tauri (`tauri build`), because a plain `cargo build`
# does not embed the frontend.
#
# Usage (from the repository root):
#   pwsh -NoProfile -File scripts/capture-desktop.ps1 [-Scale 250] [-Out site/media/desktop-sessions.png]

param(
  # The display scale of the capture machine, in percent; 0 reads it from the
  # system. The persisted window geometry is in PHYSICAL pixels, so the logical
  # viewport the shell was designed for (1280x800) needs this factor applied.
  [int]$Scale = 0,
  [string]$Out = "site/media/desktop-sessions.png",
  # Width of the published PNG. 1600 is 2x the site's display width: crisp on a
  # retina screen, and a few hundred kilobytes rather than several megabytes.
  [int]$Width = 1600
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($Scale -le 0) {
  $dpi = Get-ItemPropertyValue 'HKCU:\Control Panel\Desktop\WindowMetrics' 'AppliedDPI' -ErrorAction SilentlyContinue
  $Scale = if ($dpi) { [int]([double]$dpi / 96 * 100) } else { 100 }
  Write-Output "display scale: ${Scale}% (pass -Scale to override)"
}
$outPath = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }

# Per-monitor DPI awareness BEFORE any window call: otherwise GetWindowRect
# reports virtualized (logical) coordinates while PrintWindow renders at physical
# resolution, and the capture is a cropped top-left corner.
Add-Type @"
using System; using System.Runtime.InteropServices;
public class DpiAware { [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v); }
"@
[DpiAware]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null   # PER_MONITOR_AWARE_V2

foreach ($bin in @("mock-agent.exe", "meltemi.exe", "meltemid.exe", "meltemi-desktop.exe")) {
  if (-not (Test-Path (Join-Path $repo "target\release\$bin"))) {
    throw "target\release\$bin is missing; see the prerequisites at the top of this script"
  }
}

# ---- the fixture ------------------------------------------------------------

$base = Join-Path $env:TEMP "meltemi-capture"
if (Test-Path $base) { Remove-Item -Recurse -Force $base }
$fixture = Join-Path $base "harbour"
$data = Join-Path $base "data"
$config = Join-Path $base "config"
New-Item -ItemType Directory -Force -Path (Join-Path $fixture ".meltemi\changes\dark-mode"), $data, $config | Out-Null

$mock = (Join-Path $repo "target\release\mock-agent.exe").Replace('\', '/')

# A local registry with one provider that resolves to the simulated agent. Each
# candidate path is the binary FILE, never the directory holding it.
Set-Content -Path (Join-Path $fixture ".meltemi\registry.toml") -Value @"
version = "capture-fixture"
[[agents]]
id = "mock-agent"
name = "Mock Agent"
level = 1
bin = "mock-agent"
acp-args = []
candidate-paths = ['$mock']
"@

$registry = (Join-Path $fixture ".meltemi\registry.toml").Replace('\', '/')
# Two launch profiles over the same agent: the picture has to show that a
# subscription is a property of the launch, not of the agent.
Set-Content -Path (Join-Path $fixture ".meltemi\config.toml") -Value @"
[agent]
command = ['$mock']

[fleet]
registry = '$registry'

[[fleet.profile]]
name = "work"
agent = "mock-agent"
env = { MELTEMI_MOCK_MARKER = "work-ctx" }

[[fleet.profile]]
name = "personal"
agent = "mock-agent"
env = { MELTEMI_MOCK_MARKER = "personal-ctx" }
"@

# Allow-all inside the fixture: the CLI owns the sessions it dispatches, so
# without a rule every request would block on a prompt this script cannot answer.
Set-Content -Path (Join-Path $fixture ".meltemi\permissions.toml") -Value "[[rule]]`neffect = `"allow`"`n"
Set-Content -Path (Join-Path $fixture ".meltemi\changes\dark-mode\tasks.md") -Value "## 1. Build`n`n- [ ] 1.1 Add the dark-mode toggle`n- [ ] 1.2 Wire the persisted preference`n- [ ] 1.3 Respect the system preference`n- [ ] 1.4 Cover the toggle with tests`n"
Set-Content -Path (Join-Path $fixture "README.md") -Value "# harbour`n"

git -C $fixture init -q
git -C $fixture config user.email "fixture@meltemi.test"
git -C $fixture config user.name "Meltemi Fixture"
git -C $fixture config commit.gpgsign false
git -C $fixture add -A
git -C $fixture commit -q -m "init"

# ---- isolated environment ---------------------------------------------------

$env:MELTEMI_ENDPOINT = "\\.\pipe\meltemi-capture-$PID"
$env:MELTEMI_DATA_DIR = $data
$env:MELTEMI_CONFIG_DIR = $config
# Onboarding already seen: the figure shows the shell, not the welcome.
Set-Content -Path (Join-Path $data "desktop-onboarding-seen") -Value "seen"

$logicalW = 1280; $logicalH = 800
$physW = [int]($logicalW * $Scale / 100); $physH = [int]($logicalH * $Scale / 100)
$uiState = @{
  theme = "dark"; locale = "en"; lastView = "sessions"
  window = @{ x = 0; y = 0; width = $physW; height = $physH; maximized = $false }
  editorRecents = @{}; paletteUsage = @{}
} | ConvertTo-Json -Depth 5
Set-Content -Path (Join-Path $data "desktop-ui.json") -Value $uiState

# ---- real sessions ----------------------------------------------------------

Push-Location $fixture
try {
  foreach ($dispatch in @("1.1 work", "1.1 personal", "1.2 work", "1.2 personal")) {
    $task, $profile = $dispatch.Split(" ")
    & "$repo\target\release\meltemi.exe" dispatch dark-mode $task $profile 2>&1 | Select-Object -First 1
  }
  & "$repo\target\release\meltemi.exe" propose "add the dark mode toggle to settings" 2>&1 | Select-Object -First 1
} finally { Pop-Location }

# ---- the window -------------------------------------------------------------

Add-Type -AssemblyName System.Drawing
# PW_RENDERFULLCONTENT (2) is required: without it a WebView2 surface comes out
# blank, because its content is not drawn through the window's own DC.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Cap {
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$app = Start-Process -FilePath "$repo\target\release\meltemi-desktop.exe" -WorkingDirectory $fixture -PassThru
try {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  while (-not $app.MainWindowHandle -and $sw.ElapsedMilliseconds -lt 20000) {
    Start-Sleep -Milliseconds 50; $app.Refresh()
  }
  Start-Sleep -Seconds 8   # connect, first render, session list
  $app.Refresh()
  $handle = $app.MainWindowHandle
  if ($handle -eq 0) { throw "the desktop client never opened a window" }
  [Cap]::SetForegroundWindow($handle) | Out-Null
  Start-Sleep -Milliseconds 800

  $rect = New-Object Cap+RECT
  [Cap]::GetWindowRect($handle, [ref]$rect) | Out-Null
  $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
  Write-Output "window ${w}x${h} (physical)"

  $shot = New-Object System.Drawing.Bitmap($w, $h)
  $g = [System.Drawing.Graphics]::FromImage($shot)
  $hdc = $g.GetHdc()
  [Cap]::PrintWindow($handle, $hdc, 2) | Out-Null
  $g.ReleaseHdc($hdc); $g.Dispose()

  $targetH = [int][Math]::Round($h * ($Width / $w))
  $scaled = New-Object System.Drawing.Bitmap($Width, $targetH)
  $gs = [System.Drawing.Graphics]::FromImage($scaled)
  $gs.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $gs.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $gs.DrawImage($shot, 0, 0, $Width, $targetH)
  $gs.Dispose()
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outPath) | Out-Null
  $scaled.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $scaled.Dispose(); $shot.Dispose()
  Write-Output "saved $outPath (${Width}x${targetH}, $((Get-Item $outPath).Length) bytes)"
} finally {
  # Teardown: only the processes this script started, matched by their path.
  Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
  Get-Process -Name meltemid -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -like "$repo*" } |
    Stop-Process -Force -ErrorAction SilentlyContinue
}

Write-Output "Review the image before committing it: no user name in a path, no"
Write-Output "real project name, no third-party product name, no e-mail, no token."
