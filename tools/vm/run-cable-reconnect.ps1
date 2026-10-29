<# Guest-only check of the product cable route's reconnect (spec 17 7.3, added 2026-10-16):
   a control-loop stall longer than the 2 s lease expires both leases; the route must
   close, reopen on its own after 1 s and carry audio again, with the session still
   playing. The probe plays identity-coded impulses into Cable A Input every 10 ms and
   records Cable B Output. m03_cable_session (the backend's route code through the real
   control plane) pauses its own service loop for -StallMs, so no heartbeat is sent.
   Verdict: the sampled state timeline shows running -> reconnecting -> running and ends
   running, the route exits 0, Cable B is silent during the stall, and audio returns
   within StallMs + 4 s of the stall's start. Diagnostic only, never qualification. #>
[CmdletBinding()]
param([ValidateRange(3000, 8000)][int] $StallMs = 4000)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 1GB) { throw 'Requires 1 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_cable_session','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$cableSession = Join-Path $bundle 'tools\m03_cable_session.exe'
if (-not (Test-Path -LiteralPath $cableSession)) { throw 'This bundle has no m03_cable_session.exe.' }
$run = 'reconnect-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$render = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$capture = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
$stallAtSeconds = 5
$routeSeconds = 24
$impulses = 1500   # 15 s of 10 ms impulses, started about 1.5 s after the route
$failure = $null
$owned = @()
$summary = [ordered]@{}
try {
    Write-Host "Cable reconnect test: stall the route's service loop for $StallMs ms at $stallAtSeconds s. No manual playback needed."
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $renderMatch = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=' + [regex]::Escape($render) + ' id=')
    $captureMatch = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=' + [regex]::Escape($capture) + ' id=')
    if ($inventory.Code -ne 0 -or $renderMatch.Count -ne 1 -or $captureMatch.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output; no audio test started.' }

    $routeOut = Join-Path $evidence 'route.txt'
    $routeArgs = @('--seconds', [string]$routeSeconds, '--stall-at-s', [string]$stallAtSeconds, '--stall-ms', [string]$StallMs, '--tolerate-reconnect')
    $route = Start-Process -FilePath $cableSession -ArgumentList ($routeArgs | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $routeOut -RedirectStandardError (Join-Path $evidence 'route-stderr.txt')
    $null = $route.Handle
    $owned += $route
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $routeOut) -and ((Get-Content -LiteralPath $routeOut -Raw) -match 'cable route running'))) {
        if ($route.HasExited) { throw "The route exited before it ran (exit $($route.ExitCode)); see route-stderr.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw 'Route readiness timed out.' }
        Start-Sleep -Milliseconds 20
    }
    Start-Sleep -Milliseconds 1500
    $probeProcess = Start-Process -FilePath $probe -WindowStyle Hidden -PassThru `
        -ArgumentList ((@('cable-impulse',[string]$impulses,$renderMatch[0].Groups[1].Value,$captureMatch[0].Groups[1].Value,('raw=' + (Join-Path $evidence 'raw.csv'))) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -join ' ') `
        -RedirectStandardOutput (Join-Path $evidence 'probe.txt') -RedirectStandardError (Join-Path $evidence 'probe-stderr.txt')
    $null = $probeProcess.Handle
    $owned += $probeProcess
    if (-not $probeProcess.WaitForExit(60000)) { throw 'The probe did not finish.' }
    if (-not $route.WaitForExit(60000)) { throw 'The route did not finish.' }
    $summary.ProbeCode = $probeProcess.ExitCode
    $summary.RouteCode = $route.ExitCode

    # State timeline printed by the route (sampled every 100 ms).
    $routeText = Get-Content -LiteralPath $routeOut -Raw
    $timelineMatch = [regex]::Match($routeText, '(?m)^state timeline: (.+)$')
    $timeline = if ($timelineMatch.Success) { $timelineMatch.Groups[1].Value.Trim() } else { '' }
    $summary.Timeline = $timeline
    $summary.StalledAndResumed = ($routeText -match '(?m)^stall: service loop paused') -and ($routeText -match '(?m)^stall: service loop resumed')
    $summary.FinalRunning = $routeText -match '(?m)^final cableRoute=.*"state":"running"'
    $summary.SawReconnect = $timeline -match 'running -> reconnecting -> running' -and $timeline.EndsWith('running')

    # Cable B arrivals on the probe's QPC clock (100 ns units).
    $times = [Collections.Generic.List[double]]::new()
    foreach ($line in Get-Content -LiteralPath (Join-Path $evidence 'raw.csv')) {
        $parts = $line.Split(',')
        if ($parts[0] -eq 'arrival' -and $parts.Count -ge 3) { $times.Add([double]$parts[1]) }
    }
    $times.Sort()
    $summary.Arrivals = $times.Count
    $maxGap = 0.0; $gapEnd = 0
    for ($i = 1; $i -lt $times.Count; $i++) {
        $gap = ($times[$i] - $times[$i - 1]) / 1.0e4
        if ($gap -gt $maxGap) { $maxGap = $gap; $gapEnd = $i }
    }
    $summary.LongestSilenceMs = [Math]::Round($maxGap, 1)
    $summary.ArrivalsAfterSilence = if ($gapEnd -gt 0) { $times.Count - $gapEnd } else { 0 }

    $problems = @()
    if (-not $summary.StalledAndResumed) { $problems += 'the stall did not run' }
    if (-not $summary.SawReconnect) { $problems += "state timeline '$timeline' lacks running -> reconnecting -> running" }
    if (-not $summary.FinalRunning) { $problems += 'the route did not end running' }
    if ($summary.RouteCode -ne 0) { $problems += "route exit $($summary.RouteCode)" }
    if ($summary.ProbeCode -ne 0) { $problems += "probe exit $($summary.ProbeCode)" }
    if ($maxGap -lt ($StallMs - 1500)) { $problems += "Cable B was silent only $($summary.LongestSilenceMs) ms during a $StallMs ms stall" }
    if ($maxGap -gt ($StallMs + 4000)) { $problems += "audio took $($summary.LongestSilenceMs) ms to return after a $StallMs ms stall (limit $($StallMs + 4000) ms)" }
    if ($summary.ArrivalsAfterSilence -lt 200) { $problems += "only $($summary.ArrivalsAfterSilence) arrivals after the silence" }
    if ($problems.Count) { throw ('Reconnect check failed: ' + ($problems -join '; ')) }
    Write-Host ("Reconnect PASS: {0}; Cable B silent {1} ms, then {2} arrivals." -f $timeline, $summary.LongestSilenceMs, $summary.ArrivalsAfterSilence)
} catch { $failure = $_ } finally {
    foreach ($process in $owned) {
        try { if (-not $process.HasExited) { $process.Kill(); [void]$process.WaitForExit(5000) } } finally { $process.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        StallMs = $StallMs; Summary = $summary; Qualification = $false
        Scope = 'Product cable route (control plane, CableRouteWorker): service-loop stall longer than the 2 s lease, bounded reopen, audio returns; NEM VM, not qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Reconnect test recorded. Send the output.'
