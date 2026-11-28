<#
.SYNOPSIS
  Run Stage A sessions in the test VM unattended, from the copied bundle.

.DESCRIPTION
  Guest-only. Start it once from an Administrator PowerShell in the bundle
  folder (C:\ar\<bundle>), for example:

      powershell -NoProfile -ExecutionPolicy Bypass -File .\run-stage-a.ps1 -Session all

  It replaces the manual parts of the runbooks: background probes play into
  Cable A Input and record Cable B Output during the driver tone steps (no
  Listen setting or Test clicks), cable formats are switched per step and
  always restored, and a logon task stays registered for the whole run, so
  after any restart (install exit 5, A9, Driver Verifier, or a crash) you
  log back in and the run continues by itself. A restart nobody planned is
  recorded as FAIL for the step that was running, with the crash dumps. It
  prints one summary table and copies stage-a-<session>-*.zip to the
  bundle's folder on Z:.

  Verdicts: PASS/FAIL for checks that must pass on the release candidate;
  VM for results this VirtualBox guest cannot deliver (zero glitch counters,
  sustained continuity; user decision 2026-10-13, proven in Stage C); N/A
  when Windows cannot exercise the case in shared mode; REVIEW when a
  measurement needs a person to look; INFO for values only recorded.

  Sessions (with -Session all: users, eight, quality, pool, hardening):
    audio      A4-A7, A9: tone steps, stall, 8 ch, product continuity,
               bit-exact 2/8 ch, kill, reboot keeps IDs (about 60 min)
    users      A11: a second Windows user is refused a cable in use (5 min)
    eight      A15: cable count 2-8-2, eight cables for 30 min, crosstalk on
               all 56 ordered pairs (about 45 min)
    quality    A16: float32 bit-exact at 44.1/48/96 kHz x 1/2/8 ch, PCM16/
               24/32, sweeps, THD+N, latency recorded (about 30 min)
    pool       A8: play into a cable for 60 min with nothing reading; the
               non-paged pool must stay flat (about 65 min)
    hardening  A10, A11: Driver Verifier on, A4-A7 again, 30-min IOCTL
               fuzz, Verifier off (about 50 min, two restarts)
    extras     NOT part of 'all'; run it on its own, last (about 35 min, two
               restarts): the product route riding a lease-
               expiring stall, A13 update/repair and a route that binds again
               after the new endpoint IDs, and the VB-Cable latency ratio
               (needs VB-Cable installed first; skipped with a note if not).
               A12 (Discord/OBS) stays an attended step.
    round2     NOT part of 'all'. The second Stage A round on driver 0.2.5
               (2026-11-21 gap analysis; needs a bundle from
               prepare-round2-bundle.ps1, which carries RC3 as driver-rc3).
               Before any block it requires a live PC sampler at
               Z:\host-load.csv (start tools\vm\round2-host-preflight.ps1 on
               the PC) and stops with a SKIP when there is none: a night on a
               busy host cannot be told apart from a driver defect. Each block
               records the host CPU/memory it ran under.
               -Blocks picks and orders nothing, it only selects among:
                 ab        paired RC3 / 0.2.5 render comparison, alternating
                           installs, steady and restarting render streams
                           (-AbSameStart: restart after every install and
                           wait -AbSettleSeconds, so both start alike)
                 pause     (only with -WithHostPause) a cable route across a
                           90 s host-side VM pause; needs round2-pause-watcher.ps1
                 live      driver update and removal while a cable route plays
                 pool      60 min, the driver's own pool tags, not the whole pool
                 verifier  Driver Verifier, IOCTL fuzz twice with a controlled
                           restart between, a guest-stall watch after each
               Default: ab,pool,verifier,live (about 4 h, many restarts). The
               run keeps its state on Z: after every step, so a hang does not
               lose the results already collected.
#>
[CmdletBinding()]
param(
    [ValidateSet('audio', 'users', 'eight', 'quality', 'pool', 'hardening', 'extras', 'round2', 'all')][string] $Session = 'audio',
    [switch] $Resume,
    [ValidateRange(0, 1800)][int] $SettleSeconds = 300,
    [ValidateRange(60, 3600)][int] $FuzzSeconds = 1800,
    [ValidateRange(5, 1440)][int] $PoolMinutes = 120,
    [ValidateRange(1, 60)][int] $EightMinutes = 30,
    # round2 only
    [ValidatePattern('^(ab|pause|live|pool|verifier|control|repro)(,(ab|pause|live|pool|verifier|control|repro))*$')][string] $Blocks = 'ab,pool,verifier,live',
    [ValidateRange(2, 8)][int] $AbPhases = 6,
    # A/B: restart after every install, whichever driver, and wait the same time
    # before the tone runs. Without it only 0.2.5 (exit 5 on install) started
    # from a fresh boot, and its worst steady runs were the post-boot ones
    # (2026-11-27 D3).
    [switch] $AbSameStart,
    [ValidateRange(0, 1800)][int] $AbSettleSeconds = 180,
    [ValidateRange(1, 12)][int] $ReproBoots = 5,
    [switch] $WithHostPause,
    # round2: sign in by itself after every planned restart (asks for the guest
    # password once; stored in the Winlogon key for this run, removed at the end).
    [switch] $AutoLogon
)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Open PowerShell with "Run as administrator".' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw "Shared folder $peer is missing; copy the bundle from Z: first." }
# Round 2 only: the PC sampler (tools\vm\host-load.ps1) writes the share root.
$hostLoadPeer = Join-Path (Split-Path -Parent $peer) 'host-load.csv'

$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$format = Join-Path $bundle 'tools\m03_cable_format.exe'
$toneTool = Join-Path $bundle 'tools\m03_bridge_tone.exe'
$cableSession = Join-Path $bundle 'tools\m03_cable_session.exe'
$taskName = 'AudioRouter-StageA-Resume'
if ($Session -eq 'round2') {
    foreach ($needed in 'round2-support.ps1', 'round2-restart-render.ps1', 'driver-rc3\audioroutervirtual.sys', 'driver\package.json') {
        if (-not (Test-Path -LiteralPath (Join-Path $bundle $needed))) { throw "This is not a round-2 bundle: $needed is missing (build it with tools\vm\prepare-round2-bundle.ps1)." }
    }
    . (Join-Path $bundle 'round2-support.ps1')
}
$evidenceRoot = 'C:\ar\evidence'
$work = Join-Path 'C:\ar\stage-a' $Session
$statePath = Join-Path $work 'state.json'
$sessions = if ($Session -eq 'all') { @('users', 'eight', 'quality', 'pool', 'hardening') } else { @($Session) }
New-Item -ItemType Directory -Force -Path $work | Out-Null
Start-Transcript -LiteralPath (Join-Path $work ('transcript-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.txt')) | Out-Null

# --- state: steps, results and the running step survive restarts ------------
$done = [Collections.Generic.List[string]]::new()
$results = [Collections.Generic.List[object]]::new()
$state = [pscustomobject]@{ current = $null; crashed = $false; startedUtc = [DateTime]::UtcNow.ToString('o'); hostRows = 0 }
if ($Resume -and (Test-Path -LiteralPath $statePath)) {
    $saved = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    foreach ($name in @($saved.done | ForEach-Object { $_ })) { $done.Add([string]$name) }
    foreach ($row in @($saved.results | ForEach-Object { $_ })) { $results.Add($row) }
    $state.current = $saved.current
    $state.crashed = [bool]$saved.crashed
    $state.startedUtc = $saved.startedUtc
    $state.hostRows = if ($null -ne $saved.hostRows) { [int]$saved.hostRows } else { 0 }
    Write-Host "Resuming '$Session' after a restart; $($done.Count) steps already done." -ForegroundColor Cyan
} elseif (Test-Path -LiteralPath $statePath) {
    Remove-Item -LiteralPath $statePath
}
function Save-State {
    [ordered]@{ session = $Session; done = @($done); results = @($results); current = $state.current
        crashed = $state.crashed; startedUtc = $state.startedUtc; hostRows = $state.hostRows } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $statePath -Encoding UTF8
    # Keep a copy on the shared folder after every step: a guest that hangs for
    # hours (the 2026-11-11 Verifier freeze) must not take the results with it.
    try {
        Copy-Item -LiteralPath $statePath -Destination (Join-Path $peer "$Session-state.json") -Force -ErrorAction Stop
        $abRows = Join-Path $work 'ab-rows.jsonl'
        if (Test-Path -LiteralPath $abRows) { Copy-Item -LiteralPath $abRows -Destination (Join-Path $peer 'round2-ab-rows.jsonl') -Force -ErrorAction Stop }
    } catch { }
}
function Add-Result([string] $Step, [string] $Verdict, [string] $Detail) {
    $results.Add([pscustomobject]@{ step = $Step; verdict = $Verdict; detail = $Detail })
    $color = switch ($Verdict) { 'PASS' { 'Green' } 'FAIL' { 'Red' } default { 'Yellow' } }
    Write-Host ("[{0}] {1} {2}" -f $Verdict, $Step, $Detail) -ForegroundColor $color
}
function Copy-CrashDumps([string] $Label) {
    $since = [DateTime]::Parse($state.startedUtc).ToLocalTime()
    $dumps = @(Get-ChildItem -Path 'C:\Windows\Minidump\*.dmp', 'C:\Windows\MEMORY.DMP' -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTime -ge $since })
    foreach ($dump in $dumps | Where-Object Length -lt 64MB) { Copy-Item -LiteralPath $dump.FullName -Destination $work -ErrorAction SilentlyContinue }
    return $dumps
}
# The logon task stays registered for the whole run, so any restart resumes.
function Register-Resume {
    # Every option is repeated: a resumed run must behave like the one that started.
    $extra = "-SettleSeconds $SettleSeconds -FuzzSeconds $FuzzSeconds -PoolMinutes $PoolMinutes -EightMinutes $EightMinutes -Blocks $Blocks -AbPhases $AbPhases -ReproBoots $ReproBoots"
    if ($WithHostPause) { $extra += ' -WithHostPause' }
    if ($AbSameStart) { $extra += " -AbSameStart -AbSettleSeconds $AbSettleSeconds" }
    $action = New-ScheduledTaskAction -Execute $powershell `
        -Argument "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$PSCommandPath`" -Session $Session -Resume $extra" `
        -WorkingDirectory $bundle
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
    $owner = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $owner -Force | Out-Null
}
function Restart-AndResume([string] $Why) {
    $state.current = $null
    Save-State
    Write-Host "$Why The VM restarts in 15 seconds; log back in and this run continues by itself." -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    Start-Sleep -Seconds 15
    Restart-Computer -Force
    exit 0
}
$autoLogonBefore = 'C:\ar\stage-a\autologon-before.json'
if ($AutoLogon) {
    if ($Session -ne 'round2') { throw '-AutoLogon is only for -Session round2.' }
    if (-not $Resume) {
        $secure = Read-Host "Password of $env:USERDOMAIN\$env:USERNAME (kept in the guest's Winlogon key for this run only, removed at the end)" -AsSecureString
        if ($secure.Length -eq 0) { throw 'An empty password cannot be used for auto-logon; run again without -AutoLogon, or set a guest password first.' }
        Add-Type -AssemblyName System.DirectoryServices.AccountManagement
        $context = New-Object System.DirectoryServices.AccountManagement.PrincipalContext('Machine')
        $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
        if (-not $context.ValidateCredentials($env:USERNAME, $plain)) { throw 'That password is not accepted for this account; nothing was changed.' }
        New-Item -ItemType Directory -Path (Split-Path $autoLogonBefore) -Force | Out-Null
        $previousLogon = Set-GuestAutoLogon -UserName $env:USERNAME -Domain $env:USERDOMAIN -Password $secure
        $previousLogon | ConvertTo-Json | Set-Content -LiteralPath $autoLogonBefore -Encoding UTF8
        Write-Host 'Auto-logon is on for this run.' -ForegroundColor Cyan
    }
}
# --- host load (round2) --------------------------------------------------------
# The PC sampler writes C:\VMs\ar-share\host-load.csv, which the guest sees as
# Z:\host-load.csv. Blocks are judged by row position; the guest clock is not
# compared with the host clock (the 2026-11-22 core showed they differ by hours).
# Defined before the resume check below, which calls Add-HostLoadResult after an
# unplanned restart (2026-11-27: defined later, the resumed run died on it).
function Read-HostLoadRows {
    param([string] $Path = $hostLoadPeer)
    if (-not (Test-Path -LiteralPath $Path)) { return @() }
    try { return @(Get-HostLoadRows @(Get-Content -LiteralPath $Path -ErrorAction Stop)) } catch { return @() }
}
function Add-HostLoadResult {
    param([string] $Name, [int] $From = 0, [string] $Note = '')
    $window = Get-HostLoadWindow -Rows (Read-HostLoadRows) -From $From
    $verdict = Get-HostLoadVerdict $window
    $detail = $verdict.Detail
    if ($Note) { $detail = "$detail; $Note" }
    if ($verdict.Verdict -eq 'Idle') { Add-Result "host load ($Name)" 'INFO' $detail }
    elseif ($verdict.Verdict -eq 'Starved') { Add-Result "host load ($Name)" 'REVIEW' "$detail; host starvation, so this window is not a driver result" }
    else { Add-Result "host load ($Name)" 'REVIEW' $detail }
}

Register-Resume
if ($state.current -and -not $done.Contains($state.current)) {
    # The VM restarted while a step was running and nobody asked for it.
    $dumps = Copy-CrashDumps $state.current
    Add-Result $state.current 'FAIL' "the VM restarted during this step (bugcheck?); $($dumps.Count) crash dump(s) since the run started"
    # The PC sampler kept writing through the freeze/restart. Rows from the
    # step's start (row positions, never clocks) say whether the host was busy.
    if ($Session -eq 'round2') { Add-HostLoadResult "$($state.current), until the restart" $state.hostRows 'this window decides whether the restart is host starvation or a driver/Verifier candidate' }
    $done.Add($state.current)
    $state.current = $null
    $state.crashed = $true
    Save-State
}

# --- running the existing steps ----------------------------------------------
# Child scripts write warnings to stderr; under PowerShell 5.1 with "Stop"
# those become terminating errors (validated lesson 2026-10-03). Run children
# with "Continue" and decide on their exit codes only.
function Invoke-Child([string] $Script, [string[]] $Arguments) {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $text = & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $bundle $Script) @Arguments 2>&1 |
            ForEach-Object { "$_" } | Out-String
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
    Write-Host $text
    return [pscustomobject]@{ Code = $code; Text = $text }
}
# PowerShell 5.1 ConvertFrom-Json emits a JSON array as ONE object; without
# unrolling, "$checks | Where-Object" saw every check at once and graded a
# failed counter check as passed (VM run 2026-10-13).
function Read-Checks($Dir) {
    if (-not $Dir) { return @() }
    $path = Join-Path $Dir.FullName 'summary.json'
    if (-not (Test-Path -LiteralPath $path)) { return @() }
    return @((Get-Content -LiteralPath $path -Raw | ConvertFrom-Json) | ForEach-Object { $_ })
}
function Invoke-VmCheck([string] $Step, [string[]] $Extra = @()) {
    $started = Get-Date
    $code = (Invoke-Child 'vm-checks.ps1' (@('-Step', $Step) + $Extra)).Code
    $dir = Get-ChildItem -LiteralPath $evidenceRoot -Directory -Filter "*-$Step" |
        Where-Object { $_.CreationTime -ge $started.AddSeconds(-2) } | Sort-Object Name | Select-Object -Last 1
    $checks = Read-Checks $dir
    return [pscustomobject]@{ Code = $code; Checks = $checks; Dir = $dir }
}
function Invoke-Native([string] $Executable, [string[]] $Arguments) {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $text = & $Executable @Arguments 2>&1 | ForEach-Object { "$_" } | Out-String
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
    return [pscustomobject]@{ Code = $code; Text = $text }
}
# -Prefix names the runner's evidence folders (C:\ar\evidence\<prefix><guid>)
# for runners that do not print where they copied their reports.
function Invoke-Runner([string] $Script, [string[]] $Arguments = @(), [string] $Prefix = '') {
    $started = Get-Date
    $run = Invoke-Child $Script $Arguments
    $run | Add-Member NoteProperty Started $started
    $run | Add-Member NoteProperty Prefix $Prefix
    return $run
}
# The run-cable-* scripts keep C:\ar\evidence\<run>\result.json; grade from
# that file, never the exit code (run-cable-tone exits 0 after a measured
# "fail"). The run comes from "...copied to Z:\<bundle>\<run>.zip" or, failing
# that, the newest folder with the runner's prefix created during the step.
function Find-RunFile($Run, [string] $File) {
    $match = [regex]::Match([string]$Run.Text, '(?m)copied to Z:\\[^\\]+\\([^\\\r\n]+)\.zip')
    if ($match.Success) {
        $path = Join-Path (Join-Path $evidenceRoot $match.Groups[1].Value) $File
        if (Test-Path -LiteralPath $path) { return $path }
    }
    if ($Run.PSObject.Properties['Prefix'] -and $Run.Prefix) {
        $dir = Get-ChildItem -LiteralPath $evidenceRoot -Directory -Filter "$($Run.Prefix)*" -ErrorAction SilentlyContinue |
            Where-Object { $_.CreationTime -ge $Run.Started.AddSeconds(-2) -and (Test-Path -LiteralPath (Join-Path $_.FullName $File)) } |
            Sort-Object CreationTime | Select-Object -Last 1
        if ($dir) { return (Join-Path $dir.FullName $File) }
    }
    return $null
}
function Get-RunResult($Run) {
    $path = Find-RunFile $Run 'result.json'
    if (-not $path) { return $null }
    return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
}
function Get-RunMetrics($Run) {
    $path = Find-RunFile $Run 'metrics.json'
    if (-not $path) { return $null }
    return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
}
function Get-EndpointIndex([string] $Flow, [string] $Name) {
    $text = (Invoke-Native $probe @('inventory')).Text
    $match = [regex]::Match($text, "(?m)^$Flow\[(\d+)\] name=" + [regex]::Escape($Name) + '\b')
    if (-not $match.Success) { throw "Cannot find $Flow endpoint '$Name' in the probe inventory." }
    return $match.Groups[1].Value
}
# Feeders replace Listen and the Test button: one probe records Cable B Output
# (the tone tool's capture consumer), one plays a tone into Cable A Input.
# Each feeder gets its own log (kept in the results) and counts as started
# only once the probe prints its stream-start line: a fixed wait was too
# short right after a Driver Verifier restart (r7 hardening tone).
function Start-Feeder([string] $Kind, [string[]] $Arguments) {
    $name = "feeder-$(Get-Date -Format 'yyyyMMdd-HHmmss')-$Kind"
    $out = Join-Path $work "$name.txt"
    $process = Start-Process -FilePath $probe -ArgumentList $Arguments -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $out -RedirectStandardError (Join-Path $work "$name-err.txt")
    $null = $process.Handle
    $process | Add-Member NoteProperty FeederLog $name
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $out) -and ((Get-Content -LiteralPath $out -Raw) -match "(?m)^${Kind}_start_tick_ms="))) {
        if ($process.HasExited) { throw "The $Kind feeder exited before its stream started (exit $($process.ExitCode)); see $name.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 30) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            throw "The $Kind feeder did not start its stream within 30 s; see $name.txt."
        }
        Start-Sleep -Milliseconds 200
    }
    return $process
}
function Start-Feeders([int] $Seconds, [switch] $RenderOnly, [switch] $CaptureOnly) {
    $milliseconds = [string](($Seconds + 20) * 1000)
    $feeders = @()
    try {
        if (-not $RenderOnly) {
            $capture = Get-EndpointIndex 'capture' 'AudioRouter Cable B Output'
            $feeders += Start-Feeder 'capture' @('capture', $capture, $milliseconds)
        }
        if (-not $CaptureOnly) {
            $render = Get-EndpointIndex 'render' 'AudioRouter Cable A Input'
            $feeders += Start-Feeder 'render' @('tone', $milliseconds, $render)
        }
    } catch { Stop-Feeders $feeders; throw }
    Start-Sleep -Seconds 1
    return $feeders
}
function Stop-Feeders($Feeders) {
    foreach ($process in @($Feeders)) {
        if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    }
    Start-Sleep -Seconds 1
}
function Set-CableFormat([int] $Channels, [int] $Rate, [string] $Encoding = 'float32') {
    $run = Invoke-Native $format @('set', '--channels', [string]$Channels, '--rate', [string]$Rate, '--encoding', $Encoding)
    if ($run.Code -ne 0) { throw "Setting the cables to $Channels ch / $Rate Hz / $Encoding failed:`n$($run.Text)" }
}

# --- grading -------------------------------------------------------------------
# A tone step passes when the tool ran (exit 0) and, for the stall, counters
# rose. Non-zero counters in a clean run are the VM deviation (A5, Stage C).
# m03_bridge_tone holds 2 s leases. When the whole tool process froze for
# longer than that (control loop and both pumps), the driver expires the lease
# by design and the next call fails (r9 under Verifier: 4.1 s freeze,
# 0x80070016). Returns the explanation, or $null when that is not the case.
function Get-ToneLeaseFreeze([string] $Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    $text = Get-Content -LiteralPath $Path -Raw
    $control = [regex]::Match($text, 'maximum control loop gap: (\d+) us')
    $pumps = [regex]::Match($text, 'maximum pump gaps: capture=(\d+) us render=(\d+) us')
    if (-not $control.Success -or -not $pumps.Success) { return $null }
    $leaseUs = 2000000
    $gaps = @([long]$control.Groups[1].Value, [long]$pumps.Groups[1].Value, [long]$pumps.Groups[2].Value)
    if (@($gaps | Where-Object { $_ -le $leaseUs }).Count -gt 0) { return $null }
    if ($text -match 'format_mismatches: [1-9]') { return $null }
    $toolError = ([regex]::Match($text, '(?m)^error: .*$')).Value.Trim()
    $seconds = '{0:N1}' -f (($gaps | Measure-Object -Minimum).Minimum / 1e6)
    return "the whole tool process froze for $seconds s (control and both pumps), longer than its 2 s lease, so the driver expired the lease by design ($toolError)"
}
function Grade-Tone([string] $Label, $Run, [switch] $Stall) {
    $tool = $Run.Checks | Where-Object { $_.check -like 'Tone tool finished*' }
    $clean = $Run.Checks | Where-Object { $_.check -like 'All error counters zero*' }
    $rose = $Run.Checks | Where-Object { $_.check -like 'Counters rose*' }
    $evidence = if ($Run.Dir) { $Run.Dir.Name } else { 'no evidence folder' }
    if (-not $tool -or -not $tool.passed) {
        $freeze = if ($tool -and $Run.Dir) { Get-ToneLeaseFreeze (Join-Path $Run.Dir.FullName 'tone.txt') } else { $null }
        if ($freeze) { Add-Result $Label 'VM' "$freeze; $evidence"; return }
        Add-Result $Label 'FAIL' "tone tool did not finish ($($tool.detail)); $evidence"
        return
    }
    if ($Stall) {
        if ($rose -and $rose.passed) { Add-Result $Label 'PASS' "counters rose during the stall; $evidence" }
        else { Add-Result $Label 'FAIL' "counters did not rise during the stall; $evidence" }
        return
    }
    $mismatch = Select-String -LiteralPath (Join-Path $Run.Dir.FullName 'tone.txt') -Pattern 'format_mismatches: [1-9]' -Quiet
    if ($mismatch) { Add-Result $Label 'FAIL' "format mismatches counted; $evidence"; return }
    if ($clean -and $clean.passed) { Add-Result $Label 'PASS' "tool exit 0, all counters zero; $evidence" }
    else { Add-Result $Label 'VM' "tool exit 0, no corruption; non-zero glitch counters (VM scheduling, Stage C); $evidence" }
}
function Grade-Continuity([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no continuity result.json (exit $($Run.Code))"; return }
    $rows = @($result.Results | ForEach-Object { $_ })
    $detail = ($rows | ForEach-Object { "$($_.ToneHz) Hz $($_.Verdict): $($_.Glitches) glitches, reference $($_.ReferenceGlitches)" }) -join '; '
    $clean = $rows.Count -gt 0 -and @($rows | Where-Object { $_.Verdict -ne 'pass' }).Count -eq 0
    $referenceClean = $rows.Count -gt 0 -and @($rows | Where-Object { $_.ReferenceGlitches -ne 0 }).Count -eq 0
    if ($clean) { Add-Result $Label 'PASS' $detail }
    elseif ($referenceClean) { Add-Result $Label 'VM' "$detail (VCAB-24, Stage C)" }
    else { Add-Result $Label 'FAIL' "$detail (the reference capture glitched too)" }
}
function Grade-BitExact([string] $Label, $Run) {
    $result = Get-RunResult $Run
    $metrics = Get-RunMetrics $Run
    if (-not $result -or -not $metrics) { Add-Result $Label 'FAIL' "no bit-exact result (exit $($Run.Code))"; return }
    $r = $result.Result
    $detail = "bit-exact $($metrics.bitExact), max error $($metrics.maxAbsoluteError), $($metrics.alignedFrames) frames; dropped $($r.CaptureDropped), flagged $($r.CaptureFlagged), overflow $($r.CaptureOverflow)"
    # A route that moved nothing is a defect, never a VM stall (r7: every
    # 44.1/96 kHz run had renderBlocks 0 while reporting "running").
    $route = if ($r.CableRoute) { try { $r.CableRoute | ConvertFrom-Json } catch { $null } } else { $null }
    if ($route -and [long]$route.renderBlocks -eq 0) {
        Add-Result $Label 'FAIL' "$detail; the product route moved no audio (renderBlocks 0, failure '$($route.failure)')"
        return
    }
    # One flagged packet is Windows' discontinuity flag on the first capture
    # packet; samples, drops and overflows decide.
    if ($metrics.bitExact -and $r.CaptureDropped -eq 0 -and $r.CaptureOverflow -eq 0 -and $r.CaptureFlagged -le 1) {
        Add-Result $Label 'PASS' $detail
    } else {
        Add-Result $Label 'VM' "$detail (a lost block is the VM stall pattern)"
    }
}
function Grade-Kill([string] $Label, $Run) {
    $result = Get-RunResult $Run
    $verdict = if ($result -and $result.Passed) { 'PASS' } else { 'FAIL' }
    $note = if ($result) { "$(@($result.Results).Count) repeats, error '$($result.Error)'" } else { "no result.json (exit $($Run.Code))" }
    Add-Result $Label $verdict $note
}
# Reconnect: the runner causes the stall itself, so a failure is a real FAIL.
function Grade-Reconnect([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    $s = $result.Summary
    $detail = "timeline '$($s.Timeline)'; Cable B silent $($s.LongestSilenceMs) ms; $($s.ArrivalsAfterSilence) arrivals after"
    if ($result.Passed) { Add-Result $Label 'PASS' $detail } else { Add-Result $Label 'FAIL' "$($result.Error) ($detail)" }
}
# VCAB-25 comparison: only the ratio means anything in this VM, so it is INFO.
function Grade-VbCompare([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result -and ([string]$Run.Text -match 'VB-Cable is not installed')) {
        Add-Result $Label 'SKIP' 'VB-Cable is not installed in this guest; install it from vb-audio.com/Cable, restart, run -Session extras again'
        return
    }
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    if (-not $result.Passed) { Add-Result $Label 'FAIL' ([string]$result.Error); return }
    $vb = $result.Summary.'vb-cable'; $ar = $result.Summary.audiorouter
    Add-Result $Label 'INFO' ("VB-Cable p50 $($vb.MedianP50Ms) / p95 $($vb.MedianP95Ms) ms; AudioRouter p50 $($ar.MedianP50Ms) / p95 $($ar.MedianP95Ms) ms; ratio $($result.P50Ratio) (VM numbers, judged in Stage C)")
}
# VCAB-21: integer device formats must round-trip within 1 LSB. When Windows
# keeps mixing in float32 the runner refuses ("negotiated float32"): the case
# is then not measurable through shared mode, which is recorded, not failed.
function Grade-Pcm([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    $metrics = Get-RunMetrics $Run
    if ([string]$result.Error -match 'negotiated (\w+)') {
        Add-Result $Label 'N/A' "Windows mixes this device format as $($Matches[1]) in shared mode; the integer path is not measurable here"
        return
    }
    $lsb = if ($metrics) { $metrics.maximumErrorLsb } else { $null }
    if ($result.Passed) { Add-Result $Label 'PASS' "maximum error $lsb LSB" }
    else { Add-Result $Label 'REVIEW' "maximum error $lsb LSB; $($result.Error)" }
}
# Sweep, THD+N: Passed from result.json; a failure may be a VM loss, so it is
# kept for review with the runner's own error.
function Grade-Measurement([string] $Label, $Run, [string] $Value) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    $shown = if ($Value -and $result.Result) { "$Value $($result.Result.$Value)" } else { '' }
    if ($result.Passed) { Add-Result $Label 'PASS' $shown }
    else { Add-Result $Label 'REVIEW' "$shown $($result.Error)".Trim() }
}
function Grade-EightCables([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    $restored = if ($result.CableCountRestored) { 'cable count restored' } else { 'cable count NOT restored' }
    if (-not $result.CableCountRestored) { Add-Result $Label 'FAIL' "verdict $($result.Verdict); $restored"; return }
    if (-not $result.Verdict) {
        # The runner stopped before a verdict: report its own error, not a guess.
        Add-Result $Label 'REVIEW' "no verdict: $($result.Error); $restored"
        return
    }
    switch ([string]$result.Verdict) {
        'clean' { Add-Result $Label 'PASS' "no lost impulse, no counter increase; $restored" }
        'all-events-vm-wide' { Add-Result $Label 'VM' "every loss coincided with a VM-wide stall (VCAB-24, Stage C); $restored" }
        'vm-correlated' {
            $c = $result.Causes
            Add-Result $Label 'VM' "events: VM-wide $($c.'vm-wide'), all four routes at once $($c.'cross-route'), relay trims after stalls $($c.'relay-trim'), unexplained 0 (VCAB-24, Stage C); $restored"
        }
        default { Add-Result $Label 'FAIL' "verdict '$($result.Verdict)': losses outside VM-wide stalls; $restored" }
    }
}
function Grade-Isolation([string] $Label, $Run) {
    $result = Get-RunResult $Run
    if (-not $result) { Add-Result $Label 'FAIL' "no result.json (exit $($Run.Code))"; return }
    $pairs = @($result.Results | ForEach-Object { $_ })
    $worst = ($pairs | Measure-Object -Property PeakDbfs -Maximum).Maximum
    $detail = "$($pairs.Count) ordered pairs, worst $worst dBFS (VCAB-26 limit -140, A15 limit -90)"
    if ($result.Passed -and $pairs.Count -gt 0) { Add-Result $Label 'PASS' $detail }
    else { Add-Result $Label 'FAIL' "$detail; $($result.Error) $($result.RestoreError)".Trim() }
}
# A8: least-squares growth of the non-paged pool after a 5-minute warm-up.
function Grade-Pool([string] $Label, $Samples) {
    $points = @($Samples | Where-Object { $_.Minutes -ge 5 })
    if ($points.Count -lt 10) { Add-Result $Label 'FAIL' "only $($points.Count) samples after warm-up"; return }
    $n = $points.Count
    $mx = ($points | Measure-Object -Property Minutes -Average).Average
    $my = ($points | Measure-Object -Property Bytes -Average).Average
    $sxy = 0.0; $sxx = 0.0
    foreach ($p in $points) { $sxy += ($p.Minutes - $mx) * ($p.Bytes - $my); $sxx += ($p.Minutes - $mx) * ($p.Minutes - $mx) }
    $perHourMiB = [Math]::Round(($sxy / $sxx) * 60 / 1MB, 3)
    $range = ($points | Measure-Object -Property Bytes -Minimum -Maximum)
    $detail = "$n samples; growth $perHourMiB MiB/h; range $([Math]::Round(($range.Maximum - $range.Minimum) / 1MB, 2)) MiB"
    if ($perHourMiB -le 1) { Add-Result $Label 'PASS' $detail }
    elseif ($perHourMiB -le 8) { Add-Result $Label 'REVIEW' "$detail (whole-system pool; check pool.csv)" }
    else { Add-Result $Label 'FAIL' $detail }
}

# --- steps ---------------------------------------------------------------------
# A step runs once. After an unplanned restart only -AlwaysRun steps (turning
# Driver Verifier off, the crash check, removal) still run.
function Step([string] $Name, [scriptblock] $Body, [switch] $AlwaysRun) {
    if ($done.Contains($Name)) { return }
    if ($state.crashed -and -not $AlwaysRun) {
        Add-Result $Name 'SKIP' 'not run after an unplanned restart'
        $done.Add($Name); Save-State; return
    }
    Write-Host "`n== $Name" -ForegroundColor Cyan
    $state.current = $Name
    # Remember where the PC sampler was when this step began. If the guest
    # freezes, the resumed run classifies the rows written since then.
    if ($Session -eq 'round2') { $state.hostRows = @(Read-HostLoadRows).Count }
    Save-State
    try { & $Body } catch { Add-Result $Name 'FAIL' $_.Exception.Message }
    $state.current = $null
    $done.Add($Name)
    Save-State
}
# A planned restart: the step counts as done before the VM goes down.
function Complete-AndRestart([string] $Name, [string] $Why) {
    $done.Add($Name)
    Restart-AndResume $Why
}
function Step-Setup([string] $P) {
    Step "${P}install" {
        $installed = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' -and $_.Status -eq 'OK' })
        if ($installed.Count -gt 0) { Add-Result "${P}install" 'PASS' 'driver already installed'; return }
        $run = Invoke-VmCheck 'preflight'
        if ($run.Code -ne 0) { throw 'preflight failed; see its output above.' }
        $run = Invoke-VmCheck 'install'
        $first = $run.Checks | Select-Object -First 1
        if ($first.detail -like 'exit 5*') {
            Add-Result "${P}install" 'PASS' 'exit 5, restart required'
            Complete-AndRestart "${P}install" 'Windows needs a restart to finish installing the driver.'
        }
        if ($run.Code -ne 0) { throw 'install failed; see its output above.' }
        Add-Result "${P}install" 'PASS' 'installed, four endpoints, idempotent'
    }
    Step "${P}status" {
        $run = Invoke-VmCheck 'status'
        if ($run.Code -ne 0) { throw 'status failed; stopping before any audio test.' }
        Add-Result "${P}status" 'PASS' (($run.Checks | Select-Object -First 1).detail)
    }
    Step "${P}quiet guest" {
        if (Test-Path -LiteralPath 'C:\ar\quiet-guest-state.json') { Add-Result "${P}quiet guest" 'PASS' 'already applied (reverted at the end)'; return }
        $run = Invoke-Runner 'quiet-guest.ps1' @('-Apply', '-SettleSeconds', [string]$SettleSeconds)
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'VM' }
        Add-Result "${P}quiet guest" $verdict "exit $($run.Code); Windows Update paused, maintenance off, Defender exclusions (reverted at the end)"
    }
}
function Step-Remove([string] $P) {
    Step "${P}remove" -AlwaysRun {
        $installed = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' })
        if ($installed.Count -eq 0) { Add-Result "${P}remove" 'PASS' 'no driver installed'; return }
        $run = Invoke-VmCheck 'remove'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}remove" $verdict 'driver uninstalled with the helper'
        # Exit 5 leaves the removal pending until a restart; the next session's
        # install then fails (exit 4, state needsRestart), as in the 2026-11-12 run.
        if ($run.Code -eq 0 -and (($run.Checks | Select-Object -First 1).detail -like 'exit 5*')) {
            Complete-AndRestart "${P}remove" 'Windows needs a restart to finish removing the driver.'
        }
    }
}
function Step-NoBugcheck([string] $P) {
    Step "${P}no bugcheck" -AlwaysRun {
        $since = [DateTime]::Parse($state.startedUtc).ToLocalTime()
        $events = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; Id = 1001, 41; StartTime = $since } -ErrorAction SilentlyContinue |
            Where-Object { $_.ProviderName -match 'WER-SystemErrorReporting|Kernel-Power' })
        $dumps = Copy-CrashDumps "${P}no bugcheck"
        if ($events.Count -eq 0 -and $dumps.Count -eq 0) { Add-Result "${P}no bugcheck" 'PASS' 'no bugcheck event and no crash dump since the run started' }
        else { Add-Result "${P}no bugcheck" 'FAIL' "$($events.Count) bugcheck/unexpected-power event(s), $($dumps.Count) dump(s); copied to the results" }
    }
}

# --- sessions ------------------------------------------------------------------
function Session-Audio([string] $P) {
    Step-Setup $P
    Step "${P}tone 30 s (A4 tool, A5)" {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone' @('-ToneSeconds', '30')
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone "${P}tone 30 s (A4 tool, A5)" $run
    }
    Step "${P}tone 10 min (A5)" {
        $script:feeders = Start-Feeders 600
        $run = Invoke-VmCheck 'tone'
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone "${P}tone 10 min (A5)" $run
    }
    Step "${P}tone stall (counters rise)" {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone-stall'
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone "${P}tone stall (counters rise)" $run -Stall
    }
    Step "${P}tone 8 ch 96 kHz" {
        Set-CableFormat 8 96000
        try {
            $script:feeders = Start-Feeders 60
            $run = Invoke-VmCheck 'tone-8ch'
            Stop-Feeders $script:feeders; $script:feeders = $null
            Grade-Tone "${P}tone 8 ch 96 kHz" $run
        } finally { Set-CableFormat 2 48000 }
    }
    Step "${P}product route continuity 10 min" {
        Grade-Continuity "${P}product route continuity 10 min" (Invoke-Runner 'run-cable-tone.ps1' @('-Minutes', '10') 'tone-')
    }
    foreach ($channels in 2, 8) {
        Step "${P}bit-exact $channels ch" {
            if ($channels -eq 8) { Set-CableFormat 8 48000 }
            try {
                Grade-BitExact "${P}bit-exact $channels ch" (Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', [string]$channels) 'bitexact-')
            } finally { if ($channels -eq 8) { Set-CableFormat 2 48000 } }
        }
    }
    Step "${P}crash recovery (A7)" {
        Grade-Kill "${P}crash recovery (A7)" (Invoke-Runner 'run-cable-kill.ps1' @() 'kill-')
    }
    Step "${P}A9 before restart" {
        (Invoke-Native $format @('show')).Text -split "`r?`n" | Select-String 'id:' | ForEach-Object { $_.Line.Trim() } |
            Set-Content -LiteralPath (Join-Path $work 'a9-before.txt')
        Complete-AndRestart "${P}A9 before restart" 'A9: restarting to check that the endpoint IDs survive a reboot.'
    }
    Step "${P}restart keeps IDs (A9)" {
        $before = @(Get-Content -LiteralPath (Join-Path $work 'a9-before.txt'))
        $after = @((Invoke-Native $format @('show')).Text -split "`r?`n" | Select-String 'id:' | ForEach-Object { $_.Line.Trim() })
        $after | Set-Content -LiteralPath (Join-Path $work 'a9-after.txt')
        $same = $before.Count -eq 4 -and -not (Compare-Object $before $after)
        $verdict = if ($same) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}restart keeps IDs (A9)" $verdict "$($after.Count) endpoint IDs compared"
    }
    Step "${P}status after restart" {
        $run = Invoke-VmCheck 'status'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}status after restart" $verdict (($run.Checks | Select-Object -First 1).detail)
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Users([string] $P) {
    Step-Setup $P
    Step "${P}second user is refused (A11)" {
        # The driver ties a lease to the Windows session (IoGetRequestorSessionId):
        # a rival in the same session gets STATUS_SHARING_VIOLATION (0x80070020,
        # "lease held"); a rival in another session gets STATUS_ACCESS_DENIED
        # (0x80070005, "in use by another Windows user"). A runas process stays
        # in this session, so the other-session case runs as a scheduled task
        # (session 0). The bridge device admits SYSTEM, Administrators and
        # interactive users only, so that task runs elevated: the device opens
        # and only the lease's session rule can refuse it.
        $name = 'artest2'
        $chars = [char[]]'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789'
        $password = -join (1..16 | ForEach-Object { $chars | Get-Random }) + '-7a'
        $secure = ConvertTo-SecureString $password -AsPlainText -Force
        $task = 'AudioRouter-StageA-A11'
        $public = 'C:\Users\Public\audiorouter-a11'
        Get-LocalUser -Name $name -ErrorAction SilentlyContinue | Remove-LocalUser
        New-LocalUser -Name $name -Password $secure -Description 'AudioRouter Stage A A11 (temporary)' -AccountNeverExpires | Out-Null
        New-Item -ItemType Directory -Force -Path $public | Out-Null
        $script:feeders = Start-Feeders 120
        $user1 = $null
        try {
            # User 1 (this account, session of the console) holds the bridge.
            $user1 = Start-Process -FilePath $toneTool -ArgumentList @('--seconds', '100', '--out', (Join-Path $work 'user1.wav')) `
                -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $work 'user1.txt') `
                -RedirectStandardError (Join-Path $work 'user1-err.txt')
            $null = $user1.Handle   # keep a handle so ExitCode is readable later
            Start-Sleep -Seconds 15

            # (a) Same session, another account (runas): must be "lease held".
            $credential = [pscredential]::new($name, $secure)
            $same = Start-Process -FilePath $toneTool -ArgumentList @('--seconds', '10', '--out', (Join-Path $public 'same.wav')) `
                -Credential $credential -LoadUserProfile -WorkingDirectory $public -WindowStyle Hidden -PassThru `
                -RedirectStandardOutput (Join-Path $public 'same.txt') -RedirectStandardError (Join-Path $public 'same-err.txt')
            $null = $same.Handle
            if (-not $same.WaitForExit(90000)) { Stop-Process -Id $same.Id -Force }
            $sameText = (Get-Content -LiteralPath (Join-Path $public 'same.txt'), (Join-Path $public 'same-err.txt') -Raw -ErrorAction SilentlyContinue) -join "`n"
            $sameLine = ([regex]::Match($sameText, '(?m)^error: .*$')).Value
            if ($sameText -match '0x80070020' -and $same.ExitCode -ne 0) {
                Add-Result "${P}same-session second user is refused" 'PASS' "lease held (0x80070020), exit $($same.ExitCode)"
            } else {
                Add-Result "${P}same-session second user is refused" 'FAIL' "exit $($same.ExitCode); '$sameLine'"
            }

            # (b) Another session (scheduled task, elevated): must be access denied
            # by the lease rule, after the device opened ("driver:" line).
            Add-LocalGroupMember -Group 'Administrators' -Member $name -ErrorAction SilentlyContinue
            $out = Join-Path $public 'other.txt'
            $command = "`"$toneTool`" --seconds 10 --out `"$(Join-Path $public 'other.wav')`" > `"$out`" 2>&1"
            $action = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument "/d /c `"$command`"" -WorkingDirectory $public
            Register-ScheduledTask -TaskName $task -Action $action -User $name -Password $password -RunLevel Highest -Force | Out-Null
            Start-ScheduledTask -TaskName $task
            $deadline = (Get-Date).AddSeconds(90)
            do { Start-Sleep -Seconds 2; $info = Get-ScheduledTask -TaskName $task } while ($info.State -eq 'Running' -and (Get-Date) -lt $deadline)
            Start-Sleep -Seconds 2
            $otherText = Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue
            $otherLine = ([regex]::Match([string]$otherText, '(?m)^error: .*$')).Value
            $opened = [string]$otherText -match '(?m)^driver: '
            if ([string]$otherText -match '0x80070005' -and $opened) {
                Add-Result "${P}second user is refused (A11)" 'PASS' 'another Windows session: access denied by the lease''s session rule (device opened, lease refused)'
            } elseif ([string]$otherText -match '0x80070005') {
                Add-Result "${P}second user is refused (A11)" 'REVIEW' "access denied before the device opened (device ACL); the lease rule was not reached: '$otherLine'"
            } else {
                Add-Result "${P}second user is refused (A11)" 'FAIL' "'$otherLine' (task state $($info.State))"
            }
            [void]$user1.WaitForExit(150000)
            $user1Code = if ($user1.HasExited) { $user1.ExitCode } else { 'still running' }
            Add-Result "${P}first user kept the cable" $(if ($user1Code -eq 0) { 'PASS' } else { 'REVIEW' }) "user 1 tone tool exit $user1Code"
        } finally {
            if ($user1 -and -not $user1.HasExited) { Stop-Process -Id $user1.Id -Force -ErrorAction SilentlyContinue }
            Stop-Feeders $script:feeders; $script:feeders = $null
            Copy-Item -Path (Join-Path $public '*.txt') -Destination $work -ErrorAction SilentlyContinue
            Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
            Remove-LocalGroupMember -Group 'Administrators' -Member $name -ErrorAction SilentlyContinue
            Get-LocalUser -Name $name -ErrorAction SilentlyContinue | Remove-LocalUser
            Get-CimInstance Win32_UserProfile -ErrorAction SilentlyContinue | Where-Object { $_.LocalPath -like "*\$name" } |
                Remove-CimInstance -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $public -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Eight([string] $P) {
    Step-Setup $P
    Step "${P}cable count 2-8-2 (A15)" {
        $run = Invoke-VmCheck 'cables'
        $failed = @($run.Checks | Where-Object { -not $_.passed })
        $verdict = if ($run.Code -eq 0 -and $failed.Count -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}cable count 2-8-2 (A15)" $verdict "$($run.Checks.Count) checks; failed: $(($failed | ForEach-Object { $_.check }) -join '; ')"
    }
    Step "${P}eight cables $EightMinutes min (A15)" {
        Grade-EightCables "${P}eight cables $EightMinutes min (A15)" (Invoke-Runner 'run-cable-continuity.ps1' @('-Minutes', [string]$EightMinutes) 'continuity-')
    }
    Step "${P}crosstalk, 56 pairs (A15, VCAB-26)" {
        Grade-Isolation "${P}crosstalk, 56 pairs (A15, VCAB-26)" (Invoke-Runner 'run-cable-isolation-matrix.ps1' @('-Seconds', '5') 'quality-isolation-matrix-')
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Quality([string] $P) {
    Step-Setup $P
    try {
        foreach ($rate in 44100, 48000, 96000) {
            foreach ($channels in 1, 2, 8) {
                $label = "${P}float32 $rate Hz $channels ch (VCAB-20)"
                Step $label {
                    Set-CableFormat $channels $rate
                    Grade-BitExact $label (Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', [string]$channels, '-Rate', [string]$rate) 'bitexact-')
                }
            }
        }
        foreach ($bits in 16, 24, 32) {
            foreach ($rate in 44100, 48000, 96000) {
                $label = "${P}pcm$bits $rate Hz 2 ch (VCAB-21)"
                Step $label {
                    Set-CableFormat 2 $rate "pcm$bits"
                    Grade-Pcm $label (Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', '2', '-Rate', [string]$rate, '-PcmBits', [string]$bits) 'bitexact-')
                }
            }
        }
        foreach ($rate in 44100, 48000, 96000) {
            $label = "${P}sweep $rate Hz (VCAB-22)"
            Step $label {
                Set-CableFormat 2 $rate
                Grade-Measurement $label (Invoke-Runner 'run-cable-sweep.ps1' @('-Channels', '2', '-Rate', [string]$rate) 'sweep-')
            }
        }
        Step "${P}THD+N 997 Hz 48 kHz" {
            Set-CableFormat 2 48000
            Grade-Measurement "${P}THD+N 997 Hz 48 kHz" (Invoke-Runner 'run-cable-thdn.ps1' @('-Channels', '2') 'thdn-') 'WorstThdnDb'
        }
        Step "${P}latency recorded (judged in Stage C)" {
            Set-CableFormat 2 48000
            $run = Invoke-Runner 'run-cable-latency.ps1' @('-Route', 'engine') 'latency-'
            $line = ([regex]::Match([string]$run.Text, '(?m)^.*p95.*$')).Value.Trim()
            Add-Result "${P}latency recorded (judged in Stage C)" 'INFO' "exit $($run.Code); $line"
        }
    } finally {
        try { Set-CableFormat 2 48000 } catch { Add-Result "${P}restore 48 kHz stereo" 'FAIL' $_.Exception.Message }
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Pool([string] $P) {
    Step-Setup $P
    Step "${P}non-paged pool flat for $PoolMinutes min (A8)" {
        # Play into Cable A Input with nothing reading Cable B (A8, VDEV-04).
        $script:feeders = Start-Feeders ($PoolMinutes * 60) -RenderOnly
        $samples = [Collections.Generic.List[object]]::new()
        $csv = Join-Path $work 'pool.csv'
        'minutes,bytes,feeder' | Set-Content -LiteralPath $csv
        $start = Get-Date
        # r7: the tone exited before the end, cause unknown (its log was
        # overwritten). Record each exit and restart it, so the stream
        # continues and the exits stay in the results.
        $restarts = [Collections.Generic.List[string]]::new()
        try {
            while (((Get-Date) - $start).TotalMinutes -lt $PoolMinutes) {
                $minutes = [Math]::Round(((Get-Date) - $start).TotalMinutes, 2)
                $feeder = @($script:feeders)[0]
                if ($feeder.HasExited) {
                    $restarts.Add("$minutes min exit $($feeder.ExitCode) ($($feeder.FeederLog).txt)")
                    if ($restarts.Count -gt 5) { throw "the tone into Cable A Input stopped $($restarts.Count) times: $($restarts -join '; ')" }
                    $script:feeders = Start-Feeders ([int][Math]::Ceiling(($PoolMinutes - $minutes) * 60)) -RenderOnly
                    $feeder = @($script:feeders)[0]
                }
                $bytes = [double](Get-CimInstance Win32_PerfRawData_PerfOS_Memory).PoolNonpagedBytes
                $samples.Add([pscustomobject]@{ Minutes = $minutes; Bytes = $bytes })
                "$minutes,$bytes,$($feeder.FeederLog)" | Add-Content -LiteralPath $csv
                Start-Sleep -Seconds 30
            }
        } finally { Stop-Feeders $script:feeders; $script:feeders = $null }
        Grade-Pool "${P}non-paged pool flat for $PoolMinutes min (A8)" $samples
        if ($restarts.Count -gt 0) {
            $last = $results[$results.Count - 1]
            $last.detail = "$($last.detail); tone restarted after: $($restarts -join '; ')"
            if ($last.verdict -eq 'PASS') { $last.verdict = 'REVIEW' }
            Write-Host "[$($last.verdict)] $($last.step) $($last.detail)" -ForegroundColor Yellow
        }
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Hardening([string] $P) {
    Step-Setup $P
    Step "${P}Driver Verifier on" {
        $run = Invoke-VmCheck 'verifier-on'
        if ($run.Code -ne 0) { throw 'verifier-on failed' }
        Add-Result "${P}Driver Verifier on" 'PASS' 'standard checks for audioroutervirtual.sys'
        Complete-AndRestart "${P}Driver Verifier on" 'Driver Verifier needs a restart.'
    }
    Step "${P}Driver Verifier active" {
        $settings = (Invoke-Native 'verifier.exe' @('/querysettings')).Text
        if ($settings -notmatch 'audioroutervirtual\.sys') { throw 'Driver Verifier is not watching audioroutervirtual.sys' }
        Add-Result "${P}Driver Verifier active" 'PASS' 'audioroutervirtual.sys verified'
    }
    Step "${P}A10 tone 30 s under Verifier" {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone' @('-ToneSeconds', '30')
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone "${P}A10 tone 30 s under Verifier" $run
    }
    Step "${P}A10 tone stall under Verifier" {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone-stall'
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone "${P}A10 tone stall under Verifier" $run -Stall
    }
    Step "${P}A10 bit-exact 2 ch under Verifier" {
        Grade-BitExact "${P}A10 bit-exact 2 ch under Verifier" (Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', '2') 'bitexact-')
    }
    Step "${P}A10 crash recovery under Verifier" {
        Grade-Kill "${P}A10 crash recovery under Verifier" (Invoke-Runner 'run-cable-kill.ps1' @() 'kill-')
    }
    Step "${P}A11 IOCTL fuzz $([int]($FuzzSeconds / 60)) min under Verifier" {
        $run = Invoke-VmCheck 'fuzz' @('-FuzzSeconds', [string]$FuzzSeconds)
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}A11 IOCTL fuzz $([int]($FuzzSeconds / 60)) min under Verifier" $verdict (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
    }
    Step "${P}Driver Verifier off" -AlwaysRun {
        $run = Invoke-VmCheck 'verifier-off'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}Driver Verifier off" $verdict 'verifier /reset'
        Complete-AndRestart "${P}Driver Verifier off" 'Turning Driver Verifier off needs a restart.'
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Session-Extras([string] $P) {
    Step-Setup $P
    foreach ($stall in 4000, 6500) {
        Step "${P}route rides a ${stall} ms stall" {
            Grade-Reconnect "${P}route rides a ${stall} ms stall (lease renewed)" (Invoke-Runner 'run-cable-reconnect.ps1' @('-StallMs', [string]$stall) 'reconnect-')
        }
    }
    # A13: update to the next package (same source, new version), then repair.
    Step "${P}A13 update" {
        $run = Invoke-VmCheck 'update'
        $helper = @($run.Checks | Where-Object { $_.check -like 'helper update*' }) | Select-Object -First 1
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}A13 update" $verdict "$($helper.detail)"
        if ($run.Code -eq 0 -and $helper.detail -like 'exit 5*') {
            Complete-AndRestart "${P}A13 update" 'Windows needs a restart to finish the driver update.'
        }
    }
    Step "${P}A13 update-verify" {
        $run = Invoke-VmCheck 'update-verify'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}A13 update-verify" $verdict (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
    }
    Step "${P}A13 repair" {
        $run = Invoke-VmCheck 'repair'
        $helper = @($run.Checks | Where-Object { $_.check -like 'helper repair*' }) | Select-Object -First 1
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result "${P}A13 repair" $verdict "$($helper.detail)"
        if ($run.Code -eq 0 -and $helper.detail -like 'exit 5*') {
            Complete-AndRestart "${P}A13 repair" 'Windows needs a restart to finish the driver repair.'
        }
    }
    # The cables have new endpoint IDs now. A route that starts afterwards binds
    # them by bus (filter reference), so audio must flow without any setting.
    Step "${P}route binds again after the update (new endpoint IDs)" {
        $label = "${P}route binds again after the update (new endpoint IDs)"
        Set-CableFormat 2 48000
        Grade-BitExact $label (Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', '2') 'bitexact-')
    }
    Step "${P}route rides a 4000 ms stall (updated driver)" {
        Grade-Reconnect "${P}route rides a 4000 ms stall (updated driver)" (Invoke-Runner 'run-cable-reconnect.ps1' @('-StallMs', '4000') 'reconnect-')
    }
    Step "${P}VB-Cable latency comparison (VCAB-25)" {
        Grade-VbCompare "${P}VB-Cable latency comparison (VCAB-25)" (Invoke-Runner 'run-cable-vbcompare.ps1' @() 'vbcompare-')
    }
    Step "${P}A12 Discord/OBS" {
        Add-Result "${P}A12 Discord/OBS" 'INFO' 'attended, not automated: follow the A12 steps in docs/operations/virtual-cable-vm-guide.md after this run'
    }
    Step-NoBugcheck $P
    Step-Remove $P
}

# --- round 2 (2026-11-21 gap analysis) -----------------------------------------
# Which package is really loaded: the service's image file against the package's
# SYS hash. A same-version package silently keeps the stored binary (2026-11-11),
# so the version string alone is not proof.
function Test-ExpectedDriverLoaded([string] $PackageName) {
    $expected = (Get-FileHash -LiteralPath (Join-Path $bundle "$PackageName\audioroutervirtual.sys") -Algorithm SHA256).Hash
    $service = Get-CimInstance Win32_SystemDriver -Filter "Name='AudioRouterVirtual'" -ErrorAction SilentlyContinue
    if (-not $service) { return [pscustomobject]@{ Ok = $false; Detail = 'no AudioRouterVirtual driver service' } }
    $path = [string]$service.PathName
    $path = $path -replace '^\\\?\?\\', '' -replace '^\\SystemRoot', $env:SystemRoot
    if (-not [IO.Path]::IsPathRooted($path)) { $path = Join-Path $env:SystemRoot $path }
    if (-not (Test-Path -LiteralPath $path)) { return [pscustomobject]@{ Ok = $false; Detail = "driver image not found: $path" } }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    $detail = "service state $($service.State), image SHA-256 $($actual.Substring(0, 12)), expected $PackageName $($expected.Substring(0, 12))"
    return [pscustomobject]@{ Ok = ($actual -ieq $expected -and $service.State -eq 'Running'); Detail = $detail }
}
# Round 2 judges 0.2.5 and nothing else: stop a block before any audio if another image is loaded.
function Step-DriverIdentity([string] $P) {
    Step "${P}driver identity" {
        $loaded = Test-ExpectedDriverLoaded 'driver'
        if (-not $loaded.Ok) { throw "the loaded driver is not the 0.2.5 package; stopping this block. $($loaded.Detail)" }
        Add-Result "${P}driver identity" 'PASS' $loaded.Detail
    }
}
# True when 0.2.5 is loaded; otherwise records a "not counted" REVIEW and returns false.
function Test-DriverGate([string] $Label) {
    $loaded = Test-ExpectedDriverLoaded 'driver'
    if ($loaded.Ok) { return $true }
    Add-Result $Label 'REVIEW' "not counted: 0.2.5 is not the loaded driver. $($loaded.Detail)"
    return $false
}
function Get-CableEndpointIds {
    $text = (Invoke-Native $probe @('inventory')).Text
    $render = [regex]::Matches($text, '(?m)^render\[\d+\] name=AudioRouter Cable A Input \(AudioRouter Virtual Cable\) id=(\S+)')
    $capture = [regex]::Matches($text, '(?m)^capture\[\d+\] name=AudioRouter Cable B Output \(AudioRouter Virtual Cable\) id=(\S+)')
    if ($render.Count -ne 1 -or $capture.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output endpoint.' }
    return [pscustomobject]@{ Render = $render[0].Groups[1].Value; Capture = $capture[0].Groups[1].Value }
}
# A product cable route in the background (tone into Cable A, recorded from Cable B).
function Start-CableRoute([int] $Seconds, [string] $Tag) {
    $ids = Get-CableEndpointIds
    $out = Join-Path $work "route-$Tag.txt"
    $process = Start-Process -FilePath $cableSession -WindowStyle Hidden -PassThru `
        -ArgumentList @('--seconds', [string]$Seconds, '--tone', '997', '--render-endpoint', $ids.Render, '--capture-endpoint', $ids.Capture) `
        -RedirectStandardOutput $out -RedirectStandardError "$out.err"
    $null = $process.Handle
    $process | Add-Member NoteProperty OutFile $out
    Start-Sleep -Seconds 30
    if ($process.HasExited) { throw "The cable route exited within 30 s (exit $($process.ExitCode)); see route-$Tag.txt. The test is invalid." }
    return $process
}
function Get-RouteFinal($Process) {
    $text = if (Test-Path -LiteralPath $Process.OutFile) { Get-Content -LiteralPath $Process.OutFile -Raw } else { '' }
    $line = ([regex]::Match([string]$text, '(?m)^final cableRoute=(.*)$')).Groups[1].Value
    $route = if ($line) { try { $line | ConvertFrom-Json } catch { $null } } else { $null }
    $routeError = ([regex]::Match([string]$text, '(?m)^error: .*$')).Value.Trim()
    return [pscustomobject]@{ Route = $route; Error = $routeError }
}
# The route process keeps writing its output file; read it without taking a lock.
function Read-SharedText([string] $Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try { return (New-Object IO.StreamReader($stream)).ReadToEnd() } finally { $stream.Dispose() }
}
# True only when the route process is alive and its newest progress line says 'running'.
function Test-RouteRunning($Process) {
    if ($Process.HasExited) { return [pscustomobject]@{ Ok = $false; Detail = "the route process already exited (exit $($Process.ExitCode))" } }
    $lines = [regex]::Matches((Read-SharedText $Process.OutFile), '(?m)^progress (\d+) ms: cableRoute=(\{.*\}) max_service_gap_us=')
    if ($lines.Count -eq 0) { return [pscustomobject]@{ Ok = $false; Detail = 'no progress line from the route yet' } }
    $last = $lines[$lines.Count - 1]
    $route = try { $last.Groups[2].Value | ConvertFrom-Json } catch { $null }
    $state = if ($route) { [string]$route.state } else { 'unreadable' }
    return [pscustomobject]@{ Ok = ($state -eq 'running'); Detail = "state '$state' at $($last.Groups[1].Value) ms" }
}
# The driver helper with the test-driver opt-in set for this child only (17 section 9.1).
function Invoke-DriverHelper([string[]] $Arguments, [string] $Name) {
    $resultFile = Join-Path $work "$Name.json"
    Remove-Item -LiteralPath $resultFile -Force -ErrorAction SilentlyContinue
    $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
    try { $run = Invoke-Native (Join-Path $bundle 'tools\audiorouter-driver-helper.exe') ($Arguments + @('--result', $resultFile)) }
    finally { Remove-Item Env:\AUDIOROUTER_ALLOW_TEST_DRIVER -ErrorAction SilentlyContinue }
    $run.Text | Set-Content -LiteralPath (Join-Path $work "$Name.txt") -Encoding UTF8
    $json = if (Test-Path -LiteralPath $resultFile) { try { Get-Content -LiteralPath $resultFile -Raw | ConvertFrom-Json } catch { $null } } else { $null }
    return [pscustomobject]@{ Code = $run.Code; Json = $json; Text = $run.Text; Error = $(if ($json -and $json.error) { [string]$json.error } else { '' }) }
}
function Step-Quiet([string] $P) {
    Step "${P}quiet guest" {
        if (Test-Path -LiteralPath 'C:\ar\quiet-guest-state.json') { Add-Result "${P}quiet guest" 'PASS' 'already applied (reverted at the end)'; return }
        $run = Invoke-Runner 'quiet-guest.ps1' @('-Apply', '-SettleSeconds', [string]$SettleSeconds)
        Add-Result "${P}quiet guest" $(if ($run.Code -eq 0) { 'PASS' } else { 'VM' }) "exit $($run.Code); Windows Update paused, maintenance off, Defender exclusions (reverted at the end)"
    }
}
# One 30 s bridge-tone run for the A/B, with the two render shapes:
#   steady    a continuous tone into Cable A (as every earlier Stage A tone step)
#   restarts  four short streams one after another (the Windows Test button)
function Invoke-AbTone([string] $Label, [string] $Driver, [string] $Package, [string] $Variant) {
    $loaded = Test-ExpectedDriverLoaded $Package
    if (-not $loaded.Ok) { Add-Result $Label 'FAIL' "the expected $Driver driver is not the one loaded; not counted. $($loaded.Detail)"; return }
    $restarter = $null
    try {
        if ($Variant -eq 'steady') {
            $script:feeders = Start-Feeders 30
        } else {
            $script:feeders = Start-Feeders 30 -CaptureOnly
            $index = Get-EndpointIndex 'render' 'AudioRouter Cable A Input'
            $restarter = Start-Process -FilePath $powershell -WindowStyle Hidden -PassThru `
                -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $bundle 'round2-restart-render.ps1'),
                    '-Probe', $probe, '-RenderIndex', $index, '-DelayMs', '4000', '-Count', '4', '-ToneMs', '4000', '-GapMs', '1500') `
                -RedirectStandardOutput (Join-Path $work "restarter-$(Get-Date -Format 'HHmmss').txt")
            $null = $restarter.Handle
        }
        $run = Invoke-VmCheck 'tone' @('-ToneSeconds', '30')
        if ($restarter -and -not $restarter.WaitForExit(30000)) { Stop-Process -Id $restarter.Id -Force -ErrorAction SilentlyContinue }
    } finally { Stop-Feeders $script:feeders; $script:feeders = $null }
    $evidence = if ($run.Dir) { $run.Dir.Name } else { 'no evidence folder' }
    $text = if ($run.Dir -and (Test-Path -LiteralPath (Join-Path $run.Dir.FullName 'tone.txt'))) { Get-Content -LiteralPath (Join-Path $run.Dir.FullName 'tone.txt') -Raw } else { '' }
    $tool = $run.Checks | Where-Object { $_.check -like 'Tone tool finished*' }
    $c = Get-ToneAbCounters $text
    # The tone tool names the driver it talked to: a run that met another version is not counted.
    $expectedVersion = [string](Get-Content -LiteralPath (Join-Path $bundle "$Package\package.json") -Raw | ConvertFrom-Json).driverVersion
    $versionOk = ($c.DriverVersion -eq $expectedVersion)
    $valid = [bool]($tool -and $tool.passed -and $c.Complete -and $versionOk)
    $row = [ordered]@{ Label = $Label; Driver = $Driver; Variant = $Variant; Valid = $valid; VersionOk = $versionOk; Evidence = $evidence }
    foreach ($property in $c.PSObject.Properties) { $row[$property.Name] = $property.Value }
    ($row | ConvertTo-Json -Compress) | Add-Content -LiteralPath (Join-Path $work 'ab-rows.jsonl')
    $detail = "valid=$valid (driver $($c.DriverVersion), expected $expectedVersion), render underrun $($c.RenderUnderrun) frames, late $($c.Late), capture underrun $($c.CaptureUnderrun), max service gap $($c.MaxGapMs) ms; $evidence"
    Add-Result $Label 'INFO' $detail
}
function Phase-Ab([int] $Index, [string] $Driver) {
    $package = if ($Driver -eq 'new') { 'driver' } else { 'driver-rc3' }
    $name = if ($Driver -eq 'new') { '0.2.5' } else { 'RC3 0.2.4' }
    $P = "ab$Index $name "
    Step "${P}install" {
        $now = Test-ExpectedDriverLoaded $package
        if ($now.Ok) { Add-Result "${P}install" 'PASS' "already loaded; $($now.Detail)"; return }
        if (@(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' }).Count -gt 0) {
            throw 'another AudioRouter driver is still installed; the previous phase did not remove it.'
        }
        $run = Invoke-VmCheck 'install' @('-PackageName', $package)
        $first = $run.Checks | Select-Object -First 1
        if ($first.detail -like 'exit 5*') {
            Add-Result "${P}install" 'PASS' 'exit 5, restart required'
            Complete-AndRestart "${P}install" 'Windows needs a restart to finish installing the driver.'
        }
        if ($run.Code -ne 0) { throw 'install failed; see its output above.' }
        if ($AbSameStart) {
            Add-Result "${P}install" 'PASS' 'installed; restarting so both drivers start from a fresh boot (-AbSameStart)'
            Complete-AndRestart "${P}install" 'Restarting so both drivers start their tone runs from a fresh boot.'
        }
        Add-Result "${P}install" 'PASS' 'installed'
    }
    Step "${P}status" {
        $run = Invoke-VmCheck 'status'
        $loaded = Test-ExpectedDriverLoaded $package
        $ok = ($run.Code -eq 0) -and $loaded.Ok
        Add-Result "${P}status" $(if ($ok) { 'PASS' } else { 'FAIL' }) "$(($run.Checks | Select-Object -First 1).detail); $($loaded.Detail)"
    }
    if ($AbSameStart) {
        Step "${P}settle" {
            Start-Sleep -Seconds $AbSettleSeconds
            Add-Result "${P}settle" 'INFO' "waited $AbSettleSeconds s after the restart, the same for both drivers"
        }
    }
    foreach ($shape in @(@('steady', 1), @('steady', 2), @('restarts', 1), @('restarts', 2))) {
        $variant = $shape[0]
        $label = "${P}tone $variant #$($shape[1])"
        Step $label { Invoke-AbTone $label $Driver $package $variant }
    }
    Step-Remove $P
}
function Block-Ab {
    Step-Quiet 'ab: '
    foreach ($index in 1..$AbPhases) {
        Phase-Ab $index $(if ($index % 2 -eq 1) { 'new' } else { 'rc3' })
    }
    Step 'ab: RC3 against 0.2.5 (review item 6, R4, R5)' {
        $path = Join-Path $work 'ab-rows.jsonl'
        $rows = if (Test-Path -LiteralPath $path) { @(Get-Content -LiteralPath $path | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json }) } else { @() }
        $rows | Select-Object Label, Driver, Variant, Valid, VersionOk, DriverVersion, RenderUnderrun, Late, PacketOverrun, CaptureUnderrun, MaxGapMs, Evidence |
            Export-Csv -LiteralPath (Join-Path $work 'ab-rows.csv') -NoTypeInformation
        $verdict = Compare-RenderAb $rows
        Add-Result 'ab: RC3 against 0.2.5 (review item 6, R4, R5)' $verdict.Verdict $verdict.Detail
        $valid = @($rows | Where-Object { $_.Valid }).Count
        Add-Result 'ab: valid runs' 'INFO' "$valid of $(@($rows).Count) tone runs were valid (complete report and the expected driver version); ab-rows.csv"
    }
}
function Block-Pause {
    Step-Setup 'pause: '
    Step-DriverIdentity 'pause: '
    Step 'pause: route rides a 90 s host-side VM pause' {
        if (-not (Test-DriverGate 'pause: route rides a 90 s host-side VM pause')) { return }
        $request = Join-Path $peer 'round2-pause-request.txt'
        $doneFile = Join-Path $peer 'round2-pause-done.txt'
        Remove-Item -LiteralPath $request, $doneFile -Force -ErrorAction SilentlyContinue
        $route = Start-CableRoute 330 'pause'
        $before = Get-Date
        'pause requested by the guest runner' | Set-Content -LiteralPath $request -Encoding ASCII
        $deadline = (Get-Date).AddSeconds(150)
        while (-not (Test-Path -LiteralPath $doneFile) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
        $paused = Test-Path -LiteralPath $doneFile
        if (-not $paused) { Remove-Item -LiteralPath $request -Force -ErrorAction SilentlyContinue }
        [void]$route.WaitForExit(420000)
        $final = Get-RouteFinal $route
        $routeState = if ($final.Route) { [string]$final.Route.state } else { 'no final route line' }
        $jump = [Math]::Round(((Get-Date) - $before).TotalSeconds, 1)
        if (-not $paused) { Add-Result 'pause: route rides a 90 s host-side VM pause' 'SKIP' 'no host watcher answered within 150 s (start tools\vm\round2-pause-watcher.ps1 on the PC); not a result'; return }
        if ($route.HasExited -and $route.ExitCode -eq 0 -and $routeState -eq 'running') {
            Add-Result 'pause: route rides a 90 s host-side VM pause' 'PASS' "route state '$routeState', exit 0 after the pause ($jump s of wall time); NOT a real sleep/resume"
        } else {
            Add-Result 'pause: route rides a 90 s host-side VM pause' 'REVIEW' "route state '$routeState', exit $(if ($route.HasExited) { $route.ExitCode } else { 'still running' }), $($final.Error); see route-pause.txt; NOT a real sleep/resume"
        }
    }
    Step-NoBugcheck 'pause: '
    Step-Remove 'pause: '
}
function Block-Live {
    Step-Setup 'live: '
    Step-DriverIdentity 'live: '
    # Review item 3: the update and the removal must finish without a restart even
    # while a client holds the cable. The route cannot survive a driver update; the
    # question is only whether Windows can unload the old image.
    # The helper is called directly (not through vm-checks 'update', which first sets a
    # 96 kHz format on Cable B Output and could end the route before the update).
    # Immediately before the call the route must be alive and report state 'running';
    # otherwise the test is invalid and is reported as not counted.
    $label = 'live: driver update while a route plays'
    Step $label {
        $loaded = Test-ExpectedDriverLoaded 'driver'
        if (-not $loaded.Ok) { Add-Result $label 'REVIEW' "not counted: 0.2.5 is not the loaded driver. $($loaded.Detail)"; return }
        $route = Start-CableRoute 300 'update'
        $ready = Test-RouteRunning $route
        if (-not $ready.Ok) {
            if (-not $route.HasExited) { Stop-Process -Id $route.Id -Force -ErrorAction SilentlyContinue }
            Add-Result $label 'REVIEW' "not counted: the route was not running before the update. $($ready.Detail)"
            return
        }
        $update = Invoke-DriverHelper @('update', '--package', (Join-Path $bundle 'driver-next')) 'live-update'
        [void]$route.WaitForExit(330000)
        if (-not $route.HasExited) { Stop-Process -Id $route.Id -Force -ErrorAction SilentlyContinue }
        $final = Get-RouteFinal $route
        $detail = "route running before the update ($($ready.Detail)); helper exit $($update.Code) $($update.Error); route ended with state '$(if ($final.Route) { $final.Route.state } else { 'unknown' })' $($final.Error); live-update.txt"
        if ($update.Code -eq 0) { Add-Result $label 'PASS' "$detail; no restart needed (review item 3)" }
        elseif ($update.Code -eq 5) {
            Add-Result $label 'REVIEW' "$detail; the update still needs a restart with a client attached (review item 3 NOT verified)"
            Complete-AndRestart $label 'Windows needs a restart to finish the driver update.'
        } else { Add-Result $label 'FAIL' $detail }
    }
    Step 'live: version after the update' {
        $status = Invoke-DriverHelper @('status') 'live-status-after-update'
        $next = Get-Content -LiteralPath (Join-Path $bundle 'driver-next\package.json') -Raw | ConvertFrom-Json
        $ok = ($status.Json -and $status.Json.state -eq 'installed' -and [string]$status.Json.installedDriverVersion -eq [string]$next.driverVersion)
        $found = if ($status.Json) { "$($status.Json.state), driver $($status.Json.installedDriverVersion)" } else { 'no status' }
        $untouched = ($status.Json -and [string]$status.Json.installedDriverVersion -eq '0.2.5.0')
        $verdict = if ($ok) { 'PASS' } elseif ($untouched) { 'REVIEW' } else { 'FAIL' }
        $why = if ($untouched) { '; still 0.2.5, so the update did not run or did not finish (see the update result above)' } else { '' }
        Add-Result 'live: version after the update' $verdict "expected $($next.driverVersion); $found$why"
    }
    $removeLabel = 'live: driver removal while a route plays'
    Step $removeLabel {
        $loaded = Test-ExpectedDriverLoaded 'driver-next'
        if (-not $loaded.Ok) { Add-Result $removeLabel 'REVIEW' "not counted: the updated driver is not the one loaded. $($loaded.Detail)"; return }
        $route = Start-CableRoute 240 'remove'
        $ready = Test-RouteRunning $route
        if (-not $ready.Ok) {
            if (-not $route.HasExited) { Stop-Process -Id $route.Id -Force -ErrorAction SilentlyContinue }
            Add-Result $removeLabel 'REVIEW' "not counted: the route was not running before the removal. $($ready.Detail)"
            return
        }
        $remove = Invoke-DriverHelper @('remove') 'live-remove'
        [void]$route.WaitForExit(270000)
        if (-not $route.HasExited) { Stop-Process -Id $route.Id -Force -ErrorAction SilentlyContinue }
        $left = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' }).Count
        $detail = "route running before the removal ($($ready.Detail)); helper exit $($remove.Code) $($remove.Error); $left AudioRouter device(s) left; live-remove.txt"
        if ($remove.Code -eq 0 -and $left -eq 0) { Add-Result $removeLabel 'PASS' "$detail; no restart needed" }
        elseif ($remove.Code -eq 5) {
            Add-Result $removeLabel 'REVIEW' "$detail; the removal still needs a restart with a client attached (review item 3 NOT verified)"
            Complete-AndRestart $removeLabel 'Windows needs a restart to finish removing the driver.'
        } else { Add-Result $removeLabel 'FAIL' $detail }
    }
    Step-NoBugcheck 'live: '
    Step-Remove 'live: '
}
function Block-Pool {
    Step-Setup 'pool: '
    $label = "pool: driver pool tags for $PoolMinutes min (A8, VCAB-28)"
    Step-DriverIdentity 'pool: '
    Step $label {
        if (-not (Test-DriverGate $label)) { return }
        $script:feeders = Start-Feeders ($PoolMinutes * 60) -RenderOnly
        $csv = Join-Path $work 'pool-tags.csv'
        $whole = [Collections.Generic.List[object]]::new()
        $tagged = [Collections.Generic.List[object]]::new()
        $tagAllocs = [Collections.Generic.List[object]]::new()
        $restarts = 0
        $started = Get-Date
        try {
            while (((Get-Date) - $started).TotalMinutes -lt $PoolMinutes) {
                $minutes = [Math]::Round(((Get-Date) - $started).TotalMinutes, 2)
                if (@($script:feeders)[0].HasExited) {
                    $restarts++
                    if ($restarts -gt 5) { throw "the tone into Cable A Input stopped $restarts times" }
                    $script:feeders = Start-Feeders ([int][Math]::Ceiling(($PoolMinutes - $minutes) * 60)) -RenderOnly
                }
                $s = Get-DriverPoolTagSample
                if (-not (Test-Path -LiteralPath $csv)) { ($s.PSObject.Properties.Name + 'minutes') -join ',' | Set-Content -LiteralPath $csv }
                (@($s.PSObject.Properties | ForEach-Object { $_.Value }) + $minutes) -join ',' | Add-Content -LiteralPath $csv
                # Keep the samples on the share while the block runs: a 24 h run
                # that loses the guest must not lose its evidence with it.
                try { Copy-Item -LiteralPath $csv -Destination (Join-Path $peer 'pool-tags.csv') -Force -ErrorAction Stop } catch { }
                $sum = 0.0; $allocs = 0.0
                foreach ($tag in $script:DriverPoolTags) { $sum += $s.$tag; $allocs += $s."$tag-allocs" }
                $tagged.Add([pscustomobject]@{ Minutes = $minutes; Bytes = $sum })
                $tagAllocs.Add([pscustomobject]@{ Minutes = $minutes; Bytes = $allocs })
                $whole.Add([pscustomobject]@{ Minutes = $minutes; Bytes = $s.TotalNonPagedBytes })
                Start-Sleep -Seconds 30
            }
        } finally { Stop-Feeders $script:feeders; $script:feeders = $null }
        $tagGrowth = Get-SeriesGrowthMiBPerHour $tagged
        $allocGrowth = Get-SeriesGrowthMiBPerHour $tagAllocs
        $wholeGrowth = Get-SeriesGrowthMiBPerHour $whole
        $first = $tagged[0].Bytes; $last = $tagged[$tagged.Count - 1].Bytes
        $detail = "driver tags (MNWR, MNTR, SyAu, AUDR): $([Math]::Round($first / 1KB)) KiB to $([Math]::Round($last / 1KB)) KiB, growth $tagGrowth MiB/h; live allocations $($tagAllocs[0].Bytes) to $($tagAllocs[$tagAllocs.Count - 1].Bytes); whole non-paged pool growth $wholeGrowth MiB/h; $($tagged.Count) samples; tone restarted $restarts time(s); pool-tags.csv (copied to the share each sample)"
        if ($null -eq $tagGrowth) { Add-Result $label 'FAIL' "too few samples; $detail" }
        elseif ($first -eq 0 -and $last -eq 0) { Add-Result $label 'FAIL' "the driver tags were never seen (wrong tag names?); $detail" }
        elseif ($tagGrowth -le 0.25) { Add-Result $label 'PASS' $detail }
        elseif ($tagGrowth -le 2) { Add-Result $label 'REVIEW' $detail }
        else { Add-Result $label 'FAIL' $detail }
    }
    Step-NoBugcheck 'pool: '
    Step-Remove 'pool: '
}
# Watches the guest for a stall: the 2026-11-11 freeze began after a Verifier fuzz.
function Invoke-StallWatch([string] $Label, [int] $Minutes) {
    $last = Get-Date
    $maxGap = 0.0
    $until = $last.AddMinutes($Minutes)
    while ((Get-Date) -lt $until) {
        Start-Sleep -Seconds 5
        $now = Get-Date
        $gap = ($now - $last).TotalSeconds
        if ($gap -gt $maxGap) { $maxGap = $gap }
        $last = $now
    }
    $maxGap = [Math]::Round($maxGap, 1)
    if ($maxGap -le 30) { Add-Result $Label 'PASS' "guest stayed responsive for $Minutes min; longest gap between 5 s samples $maxGap s" }
    else { Add-Result $Label 'FAIL' "the guest stalled: longest gap between 5 s samples $maxGap s (compare with VBox.log on the host)" }
}
function Block-Verifier {
    $P = 'verifier: '
    Step-Setup $P
    Step-DriverIdentity $P
    Step "${P}Driver Verifier on" {
        $run = Invoke-VmCheck 'verifier-on'
        if ($run.Code -ne 0) { throw 'verifier-on failed' }
        Add-Result "${P}Driver Verifier on" 'PASS' 'standard checks for audioroutervirtual.sys'
        Complete-AndRestart "${P}Driver Verifier on" 'Driver Verifier needs a restart.'
    }
    foreach ($round in 1, 2) {
        Step "${P}Driver Verifier active (round $round)" {
            $settings = (Invoke-Native 'verifier.exe' @('/querysettings')).Text
            if ($settings -notmatch 'audioroutervirtual\.sys') { throw 'Driver Verifier is not watching audioroutervirtual.sys' }
            Add-Result "${P}Driver Verifier active (round $round)" 'PASS' 'audioroutervirtual.sys verified'
        }
        Step "${P}IOCTL fuzz $([int]($FuzzSeconds / 60)) min under Verifier (round $round)" {
            if (-not (Test-DriverGate "${P}IOCTL fuzz $([int]($FuzzSeconds / 60)) min under Verifier (round $round)")) { return }
            $run = Invoke-VmCheck 'fuzz' @('-FuzzSeconds', [string]$FuzzSeconds)
            Add-Result "${P}IOCTL fuzz $([int]($FuzzSeconds / 60)) min under Verifier (round $round)" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
        }
        Step "${P}guest stall watch after fuzz (round $round)" { Invoke-StallWatch "${P}guest stall watch after fuzz (round $round)" 10 }
        if ($round -eq 1) {
            Step "${P}controlled restart under Verifier" {
                Add-Result "${P}controlled restart under Verifier" 'PASS' 'restarting on purpose; the guest must come back and the logon task continue'
                Complete-AndRestart "${P}controlled restart under Verifier" 'Planned restart between the two fuzz rounds.'
            }
        }
    }
    Step "${P}Driver Verifier off" -AlwaysRun {
        $run = Invoke-VmCheck 'verifier-off'
        Add-Result "${P}Driver Verifier off" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) 'verifier /reset'
        Complete-AndRestart "${P}Driver Verifier off" 'Turning Driver Verifier off needs a restart.'
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Block-Repro {
    # Does the Verifier freeze (2026-10-30, 2026-11-11, 2026-11-22) reproduce? Verifier stays on for
    # audioroutervirtual.sys; each boot runs one short fuzz, then restarts. A frozen guest is reset by the operator
    # (the logon task resumes the step) and counted as a freeze in the notes.
    $P = 'repro: '
    Step-Setup $P
    Step-DriverIdentity $P
    Step "${P}Driver Verifier on" {
        $run = Invoke-VmCheck 'verifier-on'
        if ($run.Code -ne 0) { throw 'verifier-on failed' }
        Add-Result "${P}Driver Verifier on" 'PASS' 'standard checks for audioroutervirtual.sys'
        Complete-AndRestart "${P}Driver Verifier on" 'Driver Verifier needs a restart.'
    }
    foreach ($boot in 1..$ReproBoots) {
        $name = "${P}boot $boot fuzz $([int]($FuzzSeconds / 60)) min under Verifier"
        Step $name {
            $settings = (Invoke-Native 'verifier.exe' @('/querysettings')).Text
            if ($settings -notmatch 'audioroutervirtual.sys') { throw 'Driver Verifier is not watching audioroutervirtual.sys' }
            if (-not (Test-DriverGate $name)) { return }
            $run = Invoke-VmCheck 'fuzz' @('-FuzzSeconds', [string]$FuzzSeconds)
            Add-Result $name $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
        }
        Step "${P}boot $boot guest stall watch" { Invoke-StallWatch "${P}boot $boot guest stall watch" 2 }
        if ($boot -lt $ReproBoots) {
            Step "${P}boot $boot restart" {
                Add-Result "${P}boot $boot restart" 'PASS' 'restarting on purpose for the next boot'
                Complete-AndRestart "${P}boot $boot restart" 'Planned restart between repro boots.'
            }
        }
    }
    Step "${P}Driver Verifier off" -AlwaysRun {
        $run = Invoke-VmCheck 'verifier-off'
        Add-Result "${P}Driver Verifier off" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) 'verifier /reset'
        Complete-AndRestart "${P}Driver Verifier off" 'Turning Driver Verifier off needs a restart.'
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Block-Control {
    # Control runs for the Verifier freezes (2026-10-30, 2026-11-11, 2026-11-22). The fuzz is identical in both
    # parts; only Driver Verifier changes. A: Verifier off. B: Verifier on Microsoft's ks.sys and portcls.sys,
    # AudioRouter's driver loaded but not verified. A freeze here points at the VM, not at our driver.
    $P = 'control: '
    Step-Setup $P
    Step-DriverIdentity $P
    Step "${P}A Verifier off: fuzz" {
        if (-not (Test-DriverGate "${P}A Verifier off: fuzz")) { return }
        $run = Invoke-VmCheck 'fuzz-plain' @('-FuzzSeconds', [string]$FuzzSeconds)
        Add-Result "${P}A Verifier off: fuzz" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
    }
    Step "${P}A guest stall watch" { Invoke-StallWatch "${P}A guest stall watch" 10 }
    Step "${P}B Verifier on ks.sys and portcls.sys" {
        $run = Invoke-VmCheck 'verifier-ms-on'
        if ($run.Code -ne 0) { throw 'verifier-ms-on failed' }
        Add-Result "${P}B Verifier on ks.sys and portcls.sys" 'PASS' 'standard checks for the Microsoft audio stack only'
        Complete-AndRestart "${P}B Verifier on ks.sys and portcls.sys" 'Driver Verifier needs a restart.'
    }
    Step "${P}B Verifier active on Microsoft drivers only" {
        $settings = (Invoke-Native 'verifier.exe' @('/querysettings')).Text
        if ($settings -notmatch 'portcls.sys') { throw 'Driver Verifier is not watching portcls.sys' }
        if ($settings -match 'audioroutervirtual.sys') { throw 'Driver Verifier is watching audioroutervirtual.sys: not a control run' }
        Add-Result "${P}B Verifier active on Microsoft drivers only" 'PASS' 'ks.sys and portcls.sys verified, audioroutervirtual.sys not'
    }
    Step "${P}B Verifier on Microsoft drivers: fuzz" {
        if (-not (Test-DriverGate "${P}B Verifier on Microsoft drivers: fuzz")) { return }
        $run = Invoke-VmCheck 'fuzz-plain' @('-FuzzSeconds', [string]$FuzzSeconds)
        Add-Result "${P}B Verifier on Microsoft drivers: fuzz" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) (($run.Checks | ForEach-Object { "$($_.check): $($_.passed)" }) -join '; ')
    }
    Step "${P}B guest stall watch" { Invoke-StallWatch "${P}B guest stall watch" 10 }
    Step "${P}Driver Verifier off" -AlwaysRun {
        $run = Invoke-VmCheck 'verifier-off'
        Add-Result "${P}Driver Verifier off" $(if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }) 'verifier /reset'
        Complete-AndRestart "${P}Driver Verifier off" 'Turning Driver Verifier off needs a restart.'
    }
    Step-NoBugcheck $P
    Step-Remove $P
}
function Step-HostLoadSampler([string] $P) {
    # A night without the PC sampler cannot tell host starvation from a driver
    # defect (2026-11-22). Growing rows prove the sampler is alive without any
    # agreement between the guest clock and the host clock.
    Step "${P}host-load sampler" {
        if (-not (Test-Path -LiteralPath $hostLoadPeer)) {
            Add-Result "${P}host-load sampler" 'FAIL' "no $hostLoadPeer on the share. Start tools\vm\round2-host-preflight.ps1 on the PC first: it checks that the PC is idle and starts the sampler, which writes C:\VMs\ar-share\host-load.csv (Z:\host-load.csv here). Nothing was measured."
            return
        }
        $rows = @(Read-HostLoadRows)
        if ($rows.Count -lt 3) {
            Add-Result "${P}host-load sampler" 'FAIL' "only $($rows.Count) row(s) in $hostLoadPeer. Start tools\vm\round2-host-preflight.ps1 on the PC, wait a minute, then rerun. Nothing was measured."
            return
        }
        Start-Sleep -Seconds 25
        $after = @(Read-HostLoadRows)
        if ($after.Count -le $rows.Count) {
            Add-Result "${P}host-load sampler" 'FAIL' "no new row in 25 s ($($rows.Count) rows): the sampler stopped. Start tools\vm\round2-host-preflight.ps1 again. Nothing was measured."
            return
        }
        Add-Result "${P}host-load sampler" 'PASS' "$($after.Count) rows and growing; a freeze can be classified against the host"
    }
}
function Session-Round2([string] $P) {
    Step-HostLoadSampler $P
    $gate = @($results | Where-Object { $_.step -eq "${P}host-load sampler" } | Select-Object -Last 1)
    if (-not $gate -or $gate.verdict -ne 'PASS') {
        Add-Result "${P}round2 blocks" 'SKIP' 'the host-load sampler check did not pass, so no block was run and no driver result is claimed'
        return
    }
    $processRows = @(Read-HostLoadRows).Count
    $selected = @($Blocks -split ',')
    if ($selected -contains 'pause' -and -not $WithHostPause) {
        Step 'pause: skipped' { Add-Result 'pause: skipped' 'SKIP' 'the pause block needs -WithHostPause and the host watcher' }
        $selected = @($selected | Where-Object { $_ -ne 'pause' })
    }
    if ($WithHostPause -and $selected -notcontains 'pause') { $selected = @('pause') + $selected }
    foreach ($block in @('ab', 'pause', 'pool', 'verifier', 'control', 'repro', 'live')) {
        if ($selected -notcontains $block) { continue }
        $from = @(Read-HostLoadRows).Count
        switch ($block) {
            'ab' { Block-Ab }
            'pause' { Block-Pause }
            'live' { Block-Live }
            'pool' { Block-Pool }
            'verifier' { Block-Verifier }
            'control' { Block-Control }
            'repro' { Block-Repro }
        }
        Add-HostLoadResult "$block block" $from
    }
    Add-HostLoadResult 'this runner process' $processRows
}

# --- run -----------------------------------------------------------------------
$script:feeders = $null
try {
    foreach ($name in $sessions) {
        $prefix = if ($sessions.Count -gt 1) { "${name}: " } else { '' }
        switch ($name) {
            'audio' { Session-Audio $prefix }
            'users' { Session-Users $prefix }
            'eight' { Session-Eight $prefix }
            'quality' { Session-Quality $prefix }
            'pool' { Session-Pool $prefix }
            'hardening' { Session-Hardening $prefix }
            'extras' { Session-Extras $prefix }
            'round2' { Session-Round2 $prefix }
        }
    }
} finally {
    if ($script:feeders) { Stop-Feeders $script:feeders }
}

# --- wrap up -------------------------------------------------------------------
Step 'revert quieting' -AlwaysRun {
    if (-not (Test-Path -LiteralPath 'C:\ar\quiet-guest-state.json')) { Add-Result 'revert quieting' 'PASS' 'nothing to revert'; return }
    $run = Invoke-Runner 'quiet-guest.ps1' @('-Revert')
    $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
    Add-Result 'revert quieting' $verdict 'guest settings restored'
}
Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
if (Test-Path -LiteralPath $autoLogonBefore) {
    Clear-GuestAutoLogon (Get-Content -LiteralPath $autoLogonBefore -Raw | ConvertFrom-Json)
    Remove-Item -LiteralPath $autoLogonBefore -Force
    Write-Host 'Auto-logon removed.' -ForegroundColor Cyan
}
$null = Invoke-VmCheck 'collect'
Copy-Item -Path (Join-Path $bundle 'evidence-*.zip') -Destination $peer -ErrorAction SilentlyContinue
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $work 'results.json') -Encoding UTF8
$results | Format-Table step, verdict, detail -AutoSize -Wrap | Out-String -Width 220 | Set-Content -LiteralPath (Join-Path $work 'summary.txt')
Stop-Transcript | Out-Null
$zip = Join-Path $peer ('stage-a-' + $Session + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.zip')
Compress-Archive -Path (Join-Path $work '*') -DestinationPath $zip
Remove-Item -LiteralPath $statePath -ErrorAction SilentlyContinue
Write-Host "`n==== Stage A '$Session' summary ====" -ForegroundColor Cyan
$results | Format-Table step, verdict, detail -AutoSize -Wrap | Out-String -Width 220 | Write-Host
$failed = @($results | Where-Object { $_.verdict -eq 'FAIL' }).Count
Write-Host "Results copied to $zip. FAIL: $failed. Tell the agent 'done'." -ForegroundColor $(if ($failed) { 'Red' } else { 'Green' })
