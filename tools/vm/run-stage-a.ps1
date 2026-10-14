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
#>
[CmdletBinding()]
param(
    [ValidateSet('audio', 'users', 'eight', 'quality', 'pool', 'hardening', 'all')][string] $Session = 'audio',
    [switch] $Resume,
    [ValidateRange(0, 1800)][int] $SettleSeconds = 300,
    [ValidateRange(60, 3600)][int] $FuzzSeconds = 1800,
    [ValidateRange(5, 120)][int] $PoolMinutes = 60,
    [ValidateRange(1, 60)][int] $EightMinutes = 30
)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Open PowerShell with "Run as administrator".' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw "Shared folder $peer is missing; copy the bundle from Z: first." }

$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$format = Join-Path $bundle 'tools\m03_cable_format.exe'
$toneTool = Join-Path $bundle 'tools\m03_bridge_tone.exe'
$taskName = 'AudioRouter-StageA-Resume'
$evidenceRoot = 'C:\ar\evidence'
$work = Join-Path 'C:\ar\stage-a' $Session
$statePath = Join-Path $work 'state.json'
$sessions = if ($Session -eq 'all') { @('users', 'eight', 'quality', 'pool', 'hardening') } else { @($Session) }
New-Item -ItemType Directory -Force -Path $work | Out-Null
Start-Transcript -LiteralPath (Join-Path $work ('transcript-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.txt')) | Out-Null

# --- state: steps, results and the running step survive restarts ------------
$done = [Collections.Generic.List[string]]::new()
$results = [Collections.Generic.List[object]]::new()
$state = [pscustomobject]@{ current = $null; crashed = $false; startedUtc = [DateTime]::UtcNow.ToString('o') }
if ($Resume -and (Test-Path -LiteralPath $statePath)) {
    $saved = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    foreach ($name in @($saved.done | ForEach-Object { $_ })) { $done.Add([string]$name) }
    foreach ($row in @($saved.results | ForEach-Object { $_ })) { $results.Add($row) }
    $state.current = $saved.current
    $state.crashed = [bool]$saved.crashed
    $state.startedUtc = $saved.startedUtc
    Write-Host "Resuming '$Session' after a restart; $($done.Count) steps already done." -ForegroundColor Cyan
} elseif (Test-Path -LiteralPath $statePath) {
    Remove-Item -LiteralPath $statePath
}
function Save-State {
    [ordered]@{ session = $Session; done = @($done); results = @($results); current = $state.current
        crashed = $state.crashed; startedUtc = $state.startedUtc } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $statePath -Encoding UTF8
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
    $action = New-ScheduledTaskAction -Execute $powershell `
        -Argument "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$PSCommandPath`" -Session $Session -Resume" `
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
Register-Resume
if ($state.current -and -not $done.Contains($state.current)) {
    # The VM restarted while a step was running and nobody asked for it.
    $dumps = Copy-CrashDumps $state.current
    Add-Result $state.current 'FAIL' "the VM restarted during this step (bugcheck?); $($dumps.Count) crash dump(s) since the run started"
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
function Start-Feeders([int] $Seconds, [switch] $RenderOnly) {
    $milliseconds = [string](($Seconds + 20) * 1000)
    $render = Get-EndpointIndex 'render' 'AudioRouter Cable A Input'
    $feeders = @()
    if (-not $RenderOnly) {
        $capture = Get-EndpointIndex 'capture' 'AudioRouter Cable B Output'
        # Separate statements: a trailing comma after a parameter value would
        # turn the next Start-Process into part of that value.
        $feeders += Start-Process -FilePath $probe -ArgumentList @('capture', $capture, $milliseconds) `
            -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $work 'feeder-capture.txt') `
            -RedirectStandardError (Join-Path $work 'feeder-capture-err.txt')
    }
    $feeders += Start-Process -FilePath $probe -ArgumentList @('tone', $milliseconds, $render) `
        -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $work 'feeder-render.txt') `
        -RedirectStandardError (Join-Path $work 'feeder-render-err.txt')
    Start-Sleep -Seconds 2
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
function Grade-Tone([string] $Label, $Run, [switch] $Stall) {
    $tool = $Run.Checks | Where-Object { $_.check -like 'Tone tool finished*' }
    $clean = $Run.Checks | Where-Object { $_.check -like 'All error counters zero*' }
    $rose = $Run.Checks | Where-Object { $_.check -like 'Counters rose*' }
    $evidence = if ($Run.Dir) { $Run.Dir.Name } else { 'no evidence folder' }
    if (-not $tool -or -not $tool.passed) { Add-Result $Label 'FAIL' "tone tool did not finish ($($tool.detail)); $evidence"; return }
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
    switch ([string]$result.Verdict) {
        'clean' { Add-Result $Label 'PASS' "no lost impulse, no counter increase; $restored" }
        'all-events-vm-wide' { Add-Result $Label 'VM' "every loss coincided with a VM-wide stall (VCAB-24, Stage C); $restored" }
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
        $name = 'artest2'
        $chars = [char[]]'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789'
        $password = -join (1..16 | ForEach-Object { $chars | Get-Random }) + '-7a'
        $secure = ConvertTo-SecureString $password -AsPlainText -Force
        Get-LocalUser -Name $name -ErrorAction SilentlyContinue | Remove-LocalUser
        New-LocalUser -Name $name -Password $secure -Description 'AudioRouter Stage A A11 (temporary)' -AccountNeverExpires | Out-Null
        $public = 'C:\Users\Public\audiorouter-a11'
        New-Item -ItemType Directory -Force -Path $public | Out-Null
        $script:feeders = Start-Feeders 100
        $user1 = $null
        try {
            # User 1 (this account) holds the cable bridge with a 90-second tone.
            $user1 = Start-Process -FilePath $toneTool -ArgumentList @('--seconds', '90', '--out', (Join-Path $work 'user1.wav')) `
                -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $work 'user1.txt') `
                -RedirectStandardError (Join-Path $work 'user1-err.txt')
            Start-Sleep -Seconds 15
            # User 2, a standard account, tries the same cable.
            $credential = [pscredential]::new($name, $secure)
            $user2 = Start-Process -FilePath $toneTool -ArgumentList @('--seconds', '10', '--out', (Join-Path $public 'user2.wav')) `
                -Credential $credential -LoadUserProfile -WorkingDirectory $public -WindowStyle Hidden -PassThru `
                -RedirectStandardOutput (Join-Path $public 'user2.txt') -RedirectStandardError (Join-Path $public 'user2-err.txt')
            if (-not $user2.WaitForExit(90000)) { Stop-Process -Id $user2.Id -Force; throw 'the second user''s tone tool did not exit within 90 s' }
            $text = (Get-Content -LiteralPath (Join-Path $public 'user2.txt'), (Join-Path $public 'user2-err.txt') -Raw -ErrorAction SilentlyContinue) -join "`n"
            Copy-Item -Path (Join-Path $public 'user2*.txt') -Destination $work
            $refused = $text -match 'AccessDenied' -and $text -match 'in use by another Windows user'
            [void]$user1.WaitForExit(120000)
            $user1Code = if ($user1.HasExited) { $user1.ExitCode } else { 'still running' }
            if ($refused -and $user2.ExitCode -ne 0) {
                Add-Result "${P}second user is refused (A11)" 'PASS' "user 2 exit $($user2.ExitCode): AccessDenied, in use by another Windows user; user 1 exit $user1Code"
            } else {
                $line = ([regex]::Match($text, '(?m)^error: .*$')).Value
                Add-Result "${P}second user is refused (A11)" 'FAIL' "user 2 exit $($user2.ExitCode); '$line'; user 1 exit $user1Code"
            }
        } finally {
            if ($user1 -and -not $user1.HasExited) { Stop-Process -Id $user1.Id -Force -ErrorAction SilentlyContinue }
            Stop-Feeders $script:feeders; $script:feeders = $null
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
        'minutes,bytes' | Set-Content -LiteralPath $csv
        $start = Get-Date
        try {
            while (((Get-Date) - $start).TotalMinutes -lt $PoolMinutes) {
                $bytes = [double](Get-CimInstance Win32_PerfRawData_PerfOS_Memory).PoolNonpagedBytes
                $minutes = [Math]::Round(((Get-Date) - $start).TotalMinutes, 2)
                $samples.Add([pscustomobject]@{ Minutes = $minutes; Bytes = $bytes })
                "$minutes,$bytes" | Add-Content -LiteralPath $csv
                Start-Sleep -Seconds 30
            }
            $alive = @($script:feeders | Where-Object { -not $_.HasExited }).Count
        } finally { Stop-Feeders $script:feeders; $script:feeders = $null }
        if ($alive -eq 0) { Add-Result "${P}non-paged pool flat for $PoolMinutes min (A8)" 'FAIL' 'the tone into Cable A Input stopped early'; return }
        Grade-Pool "${P}non-paged pool flat for $PoolMinutes min (A8)" $samples
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
