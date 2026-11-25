<#
.SYNOPSIS
  Run the desktop app for hours on a copy of your database and record how its
  processes grow (memory, handles, threads, CPU, log files).

.DESCRIPTION
  The engine soak (crates/engine/examples/soak.rs) covers the audio engine
  alone. This covers what a user keeps open: the shell, its embedded backend
  and service loop, the WebView window and its 20 Hz polling, and the logs.

  Isolation: the shell gets a COPY of the database (taken while no
  AudioRouter runs), its own LOCALAPPDATA and its own WebView2 profile
  (WEBVIEW2_USER_DATA_FOLDER) under the output folder, so it writes no file
  into your real AudioRouter or WebView data and loads no saved API token or
  MCP tunnel. The script checks that the real profile folders were not
  modified. It does use your real audio devices, like the app itself: run it
  when nobody needs the routed devices (overnight).

  Steps: the script launches the shell and waits. Click Play on the session
  you want to soak, or pass -AutoPlay to play the selected session at launch
  (sampling waits up to -PlayWaitMinutes for a "running" session in the
  log). It then samples every -IntervalSeconds for -Hours, stops the shell
  it started, and writes summary.md.

  Verdicts (after a 30-minute warm-up, least-squares slope per hour):
    shell private bytes      PASS < 5 MiB/h, REVIEW < 20, else FAIL
    WebView private (total)  PASS < 20 MiB/h, REVIEW < 60, else FAIL
    shell handles            PASS < 50/h, REVIEW < 200, else FAIL
    shell threads            PASS when max - min <= 8 after warm-up
    logs                     PASS when both logs keep >= 24 h of history at
                             the measured rate (5 MB each before rotation)
    shell alive              FAIL if the shell exited before the end

  Synthetic mode (-Synthetic) launches the same shell on a fresh disposable
  database, profile and embedded backend on the default control pipe (checked
  free), then drives device-free graph edits, idle UI polling and UI reload
  restarts through the WebView's loopback DevTools port. It never presses Play
  and never opens an audio endpoint. A short run is default (SyntheticMinutes
  3); pass a longer value for an opt-in run. Its verdicts use the same per-hour
  limits but are reported as a short synthetic run, not endurance evidence.
  Never start it while another AudioRouter shell is running.
#>
[CmdletBinding()]
param(
    [ValidateRange(0.1, 48)][double] $Hours = 8,
    [ValidateRange(5, 600)][int] $IntervalSeconds = 30,
    [ValidateRange(1, 120)][int] $PlayWaitMinutes = 15,
    # Play the selected session at launch (the app's Advanced > Autoplay),
    # so the run needs no click. Without it nothing plays until you click.
    [switch] $AutoPlay,
    # Device-free driver run with a fresh database; see the description.
    [switch] $Synthetic,
    [ValidateRange(1, 120)][double] $SyntheticMinutes = 3,
    # Default: the repository's release build (set below: Windows PowerShell
    # 5.1 leaves $PSScriptRoot empty while defaults are evaluated).
    [string] $Shell = '',
    # Diagnosis only: open the WebView's DevTools port (127.0.0.1) to profile
    # the renderer during the run. Never on a daily-use launch.
    [ValidateRange(0, 65535)][int] $DebugPort = 0,
    [string] $SourceDatabase = (Join-Path $env:LOCALAPPDATA 'AudioRouter\state.sqlite'),
    [string] $Output = (Join-Path $env:TEMP ('audiorouter-app-soak-' + (Get-Date -Format 'yyyyMMdd-HHmmss')))
)
$ErrorActionPreference = 'Stop'

if (-not $Shell) { $Shell = Join-Path $PSScriptRoot '..\..\src-tauri\target\release\audiorouter-shell.exe' }
$Shell = [IO.Path]::GetFullPath($Shell)
if (-not (Test-Path -LiteralPath $Shell -PathType Leaf)) { throw "Shell not found: $Shell (build it first)" }
if (-not $Synthetic -and -not (Test-Path -LiteralPath $SourceDatabase -PathType Leaf)) { throw "Database not found: $SourceDatabase" }
$running = @(Get-Process -Name 'audiorouter-shell*' -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    throw "AudioRouter is running (PID $($running.Id -join ', ')). Close it first: only one shell may run, and the database copy must be taken while it is closed."
}
if (Test-Path -LiteralPath '\\.\pipe\audiorouter-control') {
    throw "An AudioRouter backend owns the default control pipe. Close the running app before this soak; the shell must own its embedded backend."
}

New-Item -ItemType Directory -Force -Path $Output | Out-Null
$appData = Join-Path $Output 'localappdata'
$dataDir = Join-Path $appData 'AudioRouter'
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
$database = Join-Path $dataDir 'state.sqlite'
if (-not $Synthetic) {
    foreach ($suffix in '', '-wal', '-shm') {
        $from = $SourceDatabase + $suffix
        if (Test-Path -LiteralPath $from) { Copy-Item -LiteralPath $from -Destination ($database + $suffix) }
    }
}
# Settings beside the copy: autoplay only when asked; the HTTP API stays off.
@{ autoPlay = [bool]$AutoPlay; apiAutoStart = $false } | ConvertTo-Json |
    Set-Content -LiteralPath (Join-Path $dataDir 'shell-settings.json') -Encoding ASCII
$logDir = Join-Path $dataDir 'logs'
$samplesPath = Join-Path $Output 'samples.csv'
$summaryPath = Join-Path $Output 'summary.md'

# The real profile folders (known-folder path, not the variable) must not be
# written during the run.
$realRoot = [Environment]::GetFolderPath('LocalApplicationData')
$realFolders = @((Join-Path $realRoot 'AudioRouter'), (Join-Path $realRoot 'com.audiorouter.desktop')) | Where-Object { Test-Path -LiteralPath $_ }
function Get-NewestWrite {
    $newest = [DateTime]::MinValue
    foreach ($folder in $realFolders) {
        foreach ($file in Get-ChildItem -LiteralPath $folder -Recurse -File -Force -ErrorAction SilentlyContinue) {
            if ($file.LastWriteTimeUtc -gt $newest) { $newest = $file.LastWriteTimeUtc }
        }
    }
    return $newest
}
$realBefore = Get-NewestWrite
$launchUtc = [DateTime]::UtcNow

# Child-only environment: the caller's environment is restored afterwards.
$saved = @{}
foreach ($name in 'LOCALAPPDATA', 'APPDATA', 'WEBVIEW2_USER_DATA_FOLDER', 'WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS', 'AUDIOROUTER_DATABASE', 'AUDIOROUTER_ALLOW_DEVICE_ADMIN', 'AUDIOROUTER_CONTROL_PIPE') { $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
function Get-FreeLoopbackPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $listener.Stop()
    return $port
}
$devPort = if ($Synthetic) { Get-FreeLoopbackPort } else { $DebugPort }
$driver = $null
try {
    $env:LOCALAPPDATA = $appData
    # Tauri finds its WebView folder through the Windows known-folder API,
    # which ignores LOCALAPPDATA; without this the soak window used the real
    # profile (found 2026-10-14). WebView2 lets this variable override it.
    $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $appData 'webview'
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = if ($devPort) { "--remote-debugging-port=$devPort --remote-debugging-address=127.0.0.1 --remote-allow-origins=http://localhost" } else { $null }
    $env:AUDIOROUTER_DATABASE = $database
    # A test/external-pipe client disables the shell's owned backend; always
    # let this shell own its disposable backend on the default pipe.
    $env:AUDIOROUTER_CONTROL_PIPE = $null
    if ($Synthetic) {
        # The owned backend starts only when the control pipe is the app's
        # default one; setting AUDIOROUTER_CONTROL_PIPE suppresses it. The
        # default-pipe check above guarantees no other instance owns it.
        $env:APPDATA = Join-Path $appData 'roaming'
        New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
        $env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = $null
    } else {
        $env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = '1'
    }
    $shellProcess = Start-Process -FilePath $Shell -PassThru
    if ($Synthetic) {
        $driver = Start-Process -FilePath (Get-Command node).Source -ArgumentList @(
            (Join-Path $PSScriptRoot 'packaged-shell-synthetic.mjs'),
            '--port', "$devPort", '--minutes', "$SyntheticMinutes", '--output', (Join-Path $Output 'activity.json')
        ) -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $Output 'synthetic-driver.log') -RedirectStandardError (Join-Path $Output 'synthetic-driver.err')
    }
} finally {
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
Write-Host "Shell started (PID $($shellProcess.Id)). Output: $Output"
if ($devPort) { Write-Host "WebView DevTools: http://127.0.0.1:$devPort (diagnosis run)" }
if ($Synthetic) { Write-Host "Synthetic driver started: device-free edits, idle polling and UI reloads for $SyntheticMinutes minutes." }
elseif ($AutoPlay) { Write-Host "Autoplay is on: waiting up to $PlayWaitMinutes minutes for the session to report running." }
else { Write-Host "Click Play on the session to soak within $PlayWaitMinutes minutes." -ForegroundColor Yellow }

function Get-Descendants([int] $Root) {
    $all = @(Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name)
    $ids = [Collections.Generic.HashSet[int]]::new(); [void]$ids.Add($Root)
    do {
        $added = 0
        foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId) -and $ids.Add([int]$p.ProcessId)) { $added++ } }
    } while ($added -gt 0)
    return $ids
}
function Get-LogBytes {
    $total = [ordered]@{ backend = 0L; shell = 0L }
    foreach ($kind in 'backend', 'shell') {
        foreach ($name in "$kind.jsonl", "$kind.previous.jsonl") {
            $path = Join-Path $logDir $name
            if (Test-Path -LiteralPath $path) { $total[$kind] += (Get-Item -LiteralPath $path).Length }
        }
    }
    return $total
}
# Bytes ever written, counting rotations (a rotation shrinks the live file).
$logWritten = @{ backend = 0L; shell = 0L }; $logLast = @{ backend = 0L; shell = 0L }
function Update-LogWritten {
    foreach ($kind in 'backend', 'shell') {
        $path = Join-Path $logDir "$kind.jsonl"
        $now = if (Test-Path -LiteralPath $path) { (Get-Item -LiteralPath $path).Length } else { 0L }
        if ($now -ge $logLast[$kind]) { $logWritten[$kind] += $now - $logLast[$kind] } else { $logWritten[$kind] += $now }
        $logLast[$kind] = $now
    }
}

# Wait for Play: the shell logs `session.start` with state "running".
$playing = $false
if (-not $Synthetic) {
    $deadline = (Get-Date).AddMinutes($PlayWaitMinutes)
    while ((Get-Date) -lt $deadline -and -not $shellProcess.HasExited) {
        $shellLog = Join-Path $logDir 'shell.jsonl'
        if ((Test-Path -LiteralPath $shellLog) -and
            @(Select-String -LiteralPath $shellLog -Pattern '"method":"session.start"' | Where-Object { $_.Line -match '"state":"running"' }).Count -gt 0) {
            $playing = $true; break
        }
        Start-Sleep -Seconds 5
    }
    if (-not $playing) { Write-Warning 'No running session seen; sampling anyway (idle soak).' }
}

'utc,elapsed_s,shell_alive,shell_private_mib,shell_ws_mib,shell_handles,shell_threads,shell_cpu_s,webview_count,webview_private_mib,webview_cpu_s,backend_log_written,shell_log_written,backend_errors,shell_errors' | Set-Content -LiteralPath $samplesPath
$start = Get-Date
$end = if ($Synthetic) { $start.AddMinutes($SyntheticMinutes) } else { $start.AddHours($Hours) }
$sampleSeconds = if ($Synthetic) { 10 } else { $IntervalSeconds }
$rows = [Collections.Generic.List[object]]::new()
while ((Get-Date) -lt $end) {
    if ($Synthetic -and $driver -and $driver.HasExited) { break }
    $shellProcess.Refresh()
    $alive = -not $shellProcess.HasExited
    $sp = $null; $webPrivate = 0.0; $webCpu = 0.0; $webCount = 0
    if ($alive) {
        $sp = Get-Process -Id $shellProcess.Id -ErrorAction SilentlyContinue
        $ids = Get-Descendants $shellProcess.Id
        foreach ($p in Get-Process -Name msedgewebview2 -ErrorAction SilentlyContinue) {
            if ($ids.Contains($p.Id)) { $webCount++; $webPrivate += $p.PrivateMemorySize64 / 1MB; $webCpu += $p.TotalProcessorTime.TotalSeconds }
        }
    }
    Update-LogWritten
    $errors = @{ backend = 0; shell = 0 }
    foreach ($kind in 'backend', 'shell') {
        foreach ($name in "$kind.jsonl", "$kind.previous.jsonl") {
            $path = Join-Path $logDir $name
            if (Test-Path -LiteralPath $path) { $errors[$kind] += @(Select-String -LiteralPath $path -Pattern '"outcome":"error"').Count }
        }
    }
    $row = [pscustomobject]@{
        utc = [DateTime]::UtcNow.ToString('o'); elapsed = [int]((Get-Date) - $start).TotalSeconds; alive = $alive
        shellPrivate = if ($sp) { $sp.PrivateMemorySize64 / 1MB } else { 0 }; shellWs = if ($sp) { $sp.WorkingSet64 / 1MB } else { 0 }
        handles = if ($sp) { $sp.HandleCount } else { 0 }; threads = if ($sp) { $sp.Threads.Count } else { 0 }
        shellCpu = if ($sp) { $sp.TotalProcessorTime.TotalSeconds } else { 0 }
        webCount = $webCount; webPrivate = $webPrivate; webCpu = $webCpu
        backendLog = $logWritten.backend; shellLog = $logWritten.shell; backendErrors = $errors.backend; shellErrors = $errors.shell
    }
    $rows.Add($row)
    ('{0},{1},{2},{3:F1},{4:F1},{5},{6},{7:F1},{8},{9:F1},{10:F1},{11},{12},{13},{14}' -f $row.utc, $row.elapsed, $row.alive, $row.shellPrivate, $row.shellWs,
        $row.handles, $row.threads, $row.shellCpu, $row.webCount, $row.webPrivate, $row.webCpu, $row.backendLog, $row.shellLog, $row.backendErrors, $row.shellErrors) |
        Add-Content -LiteralPath $samplesPath
    if (-not $alive) { break }
    Start-Sleep -Seconds $sampleSeconds
}
if ($driver -and -not $driver.HasExited) {
    # Let the driver finish its final heap sample and activity file before
    # the wrapper stops it. Its own deadline is the same as the run's.
    $null = $driver.WaitForExit(90000)
}
if ($driver -and -not $driver.HasExited) {
    Stop-Process -Id $driver.Id -ErrorAction SilentlyContinue
}
if (-not $shellProcess.HasExited) {
    # Only the shell this script started (and its WebView children) is stopped.
    Stop-Process -Id $shellProcess.Id -ErrorAction SilentlyContinue
}

$realAfter = Get-NewestWrite
$isolated = $realAfter -le $realBefore -or $realAfter -lt $launchUtc
$isolation = if ($isolated) {
    'no file changed during the run'
} else {
    "not isolated, writer unknown: newest change $($realAfter.ToLocalTime().ToString('HH:mm:ss')) (anything else that ran, such as a CLI, also counts)"
}
$shellLabel = "$Shell (SHA-256 $((Get-FileHash -LiteralPath $Shell).Hash.Substring(0, 16))...); $(if ($Synthetic) { 'fresh disposable database' } else { "database copy of $SourceDatabase" })"
$mode = if ($Synthetic) { 'short' } else { 'endurance' }
$warmUp = if ($Synthetic) { 60 } else { 1800 }
$analyzer = Join-Path $PSScriptRoot 'app-soak-analysis.mjs'
$analysisArgs = @($analyzer, '--samples', $samplesPath, '--summary', $summaryPath, '--warm-up', "$warmUp", '--mode', $mode, '--shell', $shellLabel, '--isolation', $isolation)
if ($Synthetic) { $analysisArgs += @('--activity', (Join-Path $Output 'activity.json')) }
& (Get-Command node).Source @analysisArgs
$analysisExit = $LASTEXITCODE
if ($analysisExit -eq 1) { throw "App soak checks failed; see $summaryPath" }
if ($analysisExit -ne 0) { throw "App soak analysis failed (exit $analysisExit)" }
if ($Synthetic -and -not $isolated) { throw "Real AudioRouter or WebView folders changed during the synthetic run (not isolated); see $summaryPath" }
if ($Synthetic) {
    $activityPath = Join-Path $Output 'activity.json'
    if (-not (Test-Path -LiteralPath $activityPath)) { throw "Synthetic driver produced no activity file; see $(Join-Path $Output 'synthetic-driver.err')" }
    $activity = Get-Content -LiteralPath $activityPath -Raw | ConvertFrom-Json
    if ($activity.error) { throw "Synthetic driver failed: $($activity.error)" }
    if (@($activity.pageErrors).Count -gt 0) { throw "Synthetic driver saw uncaught page errors; see $activityPath" }
    Write-Host "Synthetic activity: $($activity.edits) graph edits, $($activity.reloads) UI reloads, $($activity.rpcErrors) RPC errors, last node count $($activity.lastNodeCount)."
}
Write-Host "Samples: $samplesPath; app logs: $logDir."
