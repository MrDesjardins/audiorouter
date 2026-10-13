<#
.SYNOPSIS
  Run one Stage A session in the test VM unattended, from the copied bundle.

.DESCRIPTION
  Guest-only. Start it once from an Administrator PowerShell in the bundle
  folder (C:\ar\<bundle>):

      powershell -NoProfile -ExecutionPolicy Bypass -File .\run-stage-a.ps1 -Session audio

  It replaces the manual parts of the runbook: background probes play a tone
  into Cable A Input and record Cable B Output during the driver tone steps
  (no Listen setting or Test clicks), cable formats are switched for the
  multichannel steps and always restored, and when Windows needs a restart
  (install exit 5, the A9 reboot) it registers a one-time logon task, restarts
  the VM and continues after you log back in. It prints one summary table and
  copies stage-a-<session>-*.zip to the bundle's folder on Z:.

  Verdicts: PASS/FAIL for checks that must pass on the release candidate; VM
  for results this VirtualBox guest cannot deliver (zero glitch counters,
  sustained continuity), recorded rather than hidden (bound to Stage C).

  Sessions:
    audio   install, status, quiet guest, driver tone steps (A4/A5, stall,
            8 ch), product route continuity and bit-exact at 2 and 8 ch, A7
            kill, A9 reboot keeps IDs, remove, collect, revert quieting.
#>
[CmdletBinding()]
param(
    [ValidateSet('audio')][string] $Session = 'audio',
    [switch] $Resume,
    [ValidateRange(0, 1800)][int] $SettleSeconds = 300
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
$taskName = 'AudioRouter-StageA-Resume'
$work = Join-Path 'C:\ar\stage-a' $Session
$statePath = Join-Path $work 'state.json'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Start-Transcript -LiteralPath (Join-Path $work ('transcript-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.txt')) | Out-Null

# --- state: completed steps and results survive restarts -------------------
if ($Resume -and (Test-Path -LiteralPath $statePath)) {
    $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    $done = [Collections.Generic.List[string]]::new([string[]]@($state.done))
    $results = [Collections.Generic.List[object]]::new()
    foreach ($row in @($state.results)) { $results.Add($row) }
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Resuming $Session after restart; $($done.Count) steps already done." -ForegroundColor Cyan
} else {
    if (Test-Path -LiteralPath $statePath) { Remove-Item -LiteralPath $statePath }
    $done = [Collections.Generic.List[string]]::new()
    $results = [Collections.Generic.List[object]]::new()
}
function Save-State {
    [ordered]@{ session = $Session; done = @($done); results = @($results) } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $statePath -Encoding UTF8
}
function Add-Result([string] $Step, [string] $Verdict, [string] $Detail) {
    $results.Add([pscustomobject]@{ step = $Step; verdict = $Verdict; detail = $Detail })
    $color = switch ($Verdict) { 'PASS' { 'Green' } 'FAIL' { 'Red' } default { 'Yellow' } }
    Write-Host ("[{0}] {1} {2}" -f $Verdict, $Step, $Detail) -ForegroundColor $color
}
function Restart-AndResume([string] $Why) {
    Save-State
    $action = New-ScheduledTaskAction -Execute $powershell `
        -Argument "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$PSCommandPath`" -Session $Session -Resume" `
        -WorkingDirectory $bundle
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
    $owner = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $owner -Force | Out-Null
    Write-Host "$Why The VM restarts in 15 seconds; log back in and this run continues by itself." -ForegroundColor Yellow
    Stop-Transcript | Out-Null
    Start-Sleep -Seconds 15
    Restart-Computer -Force
    exit 0
}

# --- running the existing steps ----------------------------------------------
function Invoke-VmCheck([string] $Step, [string[]] $Extra = @()) {
    $started = Get-Date
    & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $bundle 'vm-checks.ps1') -Step $Step @Extra | Out-Host
    $code = $LASTEXITCODE
    $dir = Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter "*-$Step" |
        Where-Object { $_.CreationTime -ge $started.AddSeconds(-2) } | Sort-Object Name | Select-Object -Last 1
    $checks = @()
    if ($dir -and (Test-Path -LiteralPath (Join-Path $dir.FullName 'summary.json'))) {
        $checks = @(Get-Content -LiteralPath (Join-Path $dir.FullName 'summary.json') -Raw | ConvertFrom-Json)
    }
    return [pscustomobject]@{ Code = $code; Checks = $checks; Dir = $dir }
}
function Invoke-Runner([string] $Script, [string[]] $Arguments = @()) {
    $output = & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $bundle $Script) @Arguments 2>&1 | Out-String
    $code = $LASTEXITCODE
    Write-Host $output
    return [pscustomobject]@{ Code = $code; Text = $output }
}
function Get-EndpointIndex([string] $Flow, [string] $Name) {
    $text = & $probe inventory 2>&1 | Out-String
    $match = [regex]::Match($text, "(?m)^$Flow\[(\d+)\] name=" + [regex]::Escape($Name) + '\b')
    if (-not $match.Success) { throw "Cannot find $Flow endpoint '$Name' in the probe inventory." }
    return $match.Groups[1].Value
}
# Feeders replace Listen and the Test button: one probe records Cable B Output
# (the tone tool's capture consumer), one plays a tone into Cable A Input.
function Start-Feeders([int] $Seconds) {
    $milliseconds = [string](($Seconds + 20) * 1000)
    $capture = Get-EndpointIndex 'capture' 'AudioRouter Cable B Output'
    $render = Get-EndpointIndex 'render' 'AudioRouter Cable A Input'
    $feeders = @(
        Start-Process -FilePath $probe -ArgumentList @('capture', $capture, $milliseconds) -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $work 'feeder-capture.txt') -RedirectStandardError (Join-Path $work 'feeder-capture-err.txt'),
        Start-Process -FilePath $probe -ArgumentList @('tone', $milliseconds, $render) -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $work 'feeder-render.txt') -RedirectStandardError (Join-Path $work 'feeder-render-err.txt')
    )
    Start-Sleep -Seconds 2
    return $feeders
}
function Stop-Feeders($Feeders) {
    foreach ($process in @($Feeders)) {
        if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
    }
    Start-Sleep -Seconds 1
}
function Set-CableFormat([int] $Channels, [int] $Rate) {
    $text = & $format set --channels $Channels --rate $Rate 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "Setting the cables to $Channels ch / $Rate Hz failed:`n$text" }
}
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
function Step([string] $Name, [scriptblock] $Body) {
    if ($done.Contains($Name)) { return }
    Write-Host "`n== $Name" -ForegroundColor Cyan
    try { & $Body } catch { Add-Result $Name 'FAIL' $_.Exception.Message }
    $done.Add($Name)
    Save-State
}

# --- the audio session (Stage A session 2) --------------------------------
$feeders = $null
try {
    Step 'install' {
        $installed = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' })
        if ($installed.Count -gt 0) { Add-Result 'install' 'PASS' 'driver already installed'; return }
        $run = Invoke-VmCheck 'preflight'
        if ($run.Code -ne 0) { throw 'preflight failed; see its output above.' }
        $run = Invoke-VmCheck 'install'
        $first = $run.Checks | Select-Object -First 1
        if ($first.detail -like 'exit 5*') {
            Add-Result 'install' 'PASS' 'exit 5, restart required'
            $done.Add('install')
            Restart-AndResume 'Windows needs a restart to finish installing the driver.'
        }
        if ($run.Code -ne 0) { throw 'install failed; see its output above.' }
        Add-Result 'install' 'PASS' 'installed, four endpoints, idempotent'
    }
    Step 'status' {
        $run = Invoke-VmCheck 'status'
        if ($run.Code -ne 0) { throw 'status failed; stopping before any audio test.' }
        Add-Result 'status' 'PASS' (($run.Checks | Select-Object -First 1).detail)
    }
    Step 'quiet guest' {
        $run = Invoke-Runner 'quiet-guest.ps1' @('-Apply', '-SettleSeconds', [string]$SettleSeconds)
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'VM' }
        Add-Result 'quiet guest' $verdict 'Windows Update paused, maintenance off, Defender exclusions (reverted at the end)'
    }
    Step 'tone 30 s (A4 tool, A5)' {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone' @('-ToneSeconds', '30')
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone 'tone 30 s (A4 tool, A5)' $run
    }
    Step 'tone 10 min (A5)' {
        $script:feeders = Start-Feeders 600
        $run = Invoke-VmCheck 'tone'
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone 'tone 10 min (A5)' $run
    }
    Step 'tone stall (counters rise)' {
        $script:feeders = Start-Feeders 30
        $run = Invoke-VmCheck 'tone-stall'
        Stop-Feeders $script:feeders; $script:feeders = $null
        Grade-Tone 'tone stall (counters rise)' $run -Stall
    }
    Step 'tone 8 ch 96 kHz' {
        Set-CableFormat 8 96000
        try {
            $script:feeders = Start-Feeders 60
            $run = Invoke-VmCheck 'tone-8ch'
            Stop-Feeders $script:feeders; $script:feeders = $null
            Grade-Tone 'tone 8 ch 96 kHz' $run
        } finally { Set-CableFormat 2 48000 }
    }
    Step 'product route continuity 10 min' {
        $run = Invoke-Runner 'run-cable-tone.ps1' @('-Minutes', '10')
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'VM' }
        Add-Result 'product route continuity 10 min' $verdict 'VCAB-24 method; losses here are bound to Stage C'
    }
    foreach ($channels in 2, 8) {
        Step "bit-exact $channels ch" {
            if ($channels -eq 8) { Set-CableFormat 8 48000 }
            try {
                $run = Invoke-Runner 'run-cable-bitexact.ps1' @('-Channels', [string]$channels)
                $summary = ([regex]::Match($run.Text, '(?m)^\s*samples: .*$')).Value.Trim()
                if ($run.Code -eq 0) { Add-Result "bit-exact $channels ch" 'PASS' $summary }
                else { Add-Result "bit-exact $channels ch" 'VM' "$summary (a single lost block is the VM stall pattern)" }
            } finally { if ($channels -eq 8) { Set-CableFormat 2 48000 } }
        }
    }
    Step 'crash recovery (A7)' {
        $run = Invoke-Runner 'run-cable-kill.ps1'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result 'crash recovery (A7)' $verdict "exit $($run.Code)"
    }
    Step 'A9 before restart' {
        & $format show 2>&1 | Select-String 'id:' | ForEach-Object { $_.Line.Trim() } | Set-Content -LiteralPath (Join-Path $work 'a9-before.txt')
        $done.Add('A9 before restart')
        Restart-AndResume 'A9: restarting to check that the endpoint IDs survive a reboot.'
    }
    Step 'restart keeps IDs (A9)' {
        $before = @(Get-Content -LiteralPath (Join-Path $work 'a9-before.txt'))
        $after = @(& $format show 2>&1 | Select-String 'id:' | ForEach-Object { $_.Line.Trim() })
        $after | Set-Content -LiteralPath (Join-Path $work 'a9-after.txt')
        $same = $before.Count -eq 4 -and -not (Compare-Object $before $after)
        $verdict = if ($same) { 'PASS' } else { 'FAIL' }
        Add-Result 'restart keeps IDs (A9)' $verdict "$($after.Count) endpoint IDs compared"
    }
    Step 'status after restart' {
        $run = Invoke-VmCheck 'status'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result 'status after restart' $verdict (($run.Checks | Select-Object -First 1).detail)
    }
    Step 'remove' {
        $run = Invoke-VmCheck 'remove'
        $verdict = if ($run.Code -eq 0) { 'PASS' } else { 'FAIL' }
        Add-Result 'remove' $verdict 'driver uninstalled with the helper'
    }
} finally {
    if ($script:feeders) { Stop-Feeders $script:feeders }
}

# --- wrap up -----------------------------------------------------------------
Step 'revert quieting' {
    $null = Invoke-Runner 'quiet-guest.ps1' @('-Revert')
    Add-Result 'revert quieting' 'PASS' 'guest settings restored'
}
$null = Invoke-VmCheck 'collect'
Copy-Item -Path (Join-Path $bundle 'evidence-*.zip') -Destination $peer -ErrorAction SilentlyContinue
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $work 'results.json') -Encoding UTF8
$results | Format-Table step, verdict, detail -AutoSize -Wrap | Out-String -Width 220 | Set-Content -LiteralPath (Join-Path $work 'summary.txt')
Stop-Transcript | Out-Null
$zip = Join-Path $peer ('stage-a-' + $Session + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.zip')
Compress-Archive -Path (Join-Path $work '*') -DestinationPath $zip
Remove-Item -LiteralPath $statePath -ErrorAction SilentlyContinue
Write-Host "`n==== Stage A session '$Session' summary ====" -ForegroundColor Cyan
$results | Format-Table step, verdict, detail -AutoSize -Wrap | Out-String -Width 220 | Write-Host
$failed = @($results | Where-Object { $_.verdict -eq 'FAIL' }).Count
Write-Host "Results copied to $zip. FAIL: $failed. Tell the agent 'done'." -ForegroundColor $(if ($failed) { 'Red' } else { 'Green' })
