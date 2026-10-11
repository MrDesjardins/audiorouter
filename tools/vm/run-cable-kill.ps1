<# Guest-only A7 check for the product cable route (WP-09): Cable B must go silent
   soon after the backend dies, and a restarted route must carry no old audio.
   The probe plays identity-coded impulses into Cable A Input every 10 ms and
   records Cable B Output. The product route (m03_cable_session) is force-killed
   mid-run (TerminateProcess: no clean stop, leases left to expire), then a new
   route starts after the lease timeout. Kill and restart times are taken on the
   probe's QPC clock. The host review (`tools/vm/review-cable-kill.py`) decides:
   last arrival <= 500 ms after the kill, and after the restart only impulses
   emitted after it, at normal latency. Diagnostic only, never qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 5)][int] $Repeats = 3)
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
$run = 'kill-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$render = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$capture = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
# Probe QPC dates are 100 ns units of the same counter Stopwatch reads.
function Get-Qpc100ns { [long]([Diagnostics.Stopwatch]::GetTimestamp() * (1.0e7 / [Diagnostics.Stopwatch]::Frequency)) }
$results = @()
$failure = $null
$owned = @()
function Start-Route([string] $Name) {
    $out = Join-Path $evidence "$Name.txt"
    $process = Start-Process -FilePath $cableSession -ArgumentList '"--seconds" "40"' -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $out -RedirectStandardError (Join-Path $evidence "$Name-stderr.txt")
    $null = $process.Handle
    $script:owned += $process
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $out) -and ((Get-Content -LiteralPath $out -Raw) -match 'cable route running'))) {
        if ($process.HasExited) { throw "$Name exited before the route ran (exit $($process.ExitCode)); see $Name-stderr.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$Name readiness timed out." }
        Start-Sleep -Milliseconds 20
    }
    return $process
}
try {
    Write-Host "Cable kill test (A7): $Repeats repeats. No manual playback needed."
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $renderMatch = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=' + [regex]::Escape($render) + ' id=')
    $captureMatch = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=' + [regex]::Escape($capture) + ' id=')
    if ($inventory.Code -ne 0 -or $renderMatch.Count -ne 1 -or $captureMatch.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output; no audio test started.' }
    for ($repeat = 1; $repeat -le $Repeats; $repeat++) {
        $name = "repeat$repeat"
        $first = Start-Route "$name-route1"
        Start-Sleep -Milliseconds 1500
        # 15 s of impulses: kill about 4 s in, restart about 3.5 s later
        # (after the 2 s lease timeout), then 7 s of the restarted route.
        $probeProcess = Start-Process -FilePath $probe -WindowStyle Hidden -PassThru `
            -ArgumentList ((@('cable-impulse','1500',$renderMatch[0].Groups[1].Value,$captureMatch[0].Groups[1].Value,('raw=' + (Join-Path $evidence "$name-raw.csv"))) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -join ' ') `
            -RedirectStandardOutput (Join-Path $evidence "$name-probe.txt") -RedirectStandardError (Join-Path $evidence "$name-probe-stderr.txt")
        $null = $probeProcess.Handle
        $script:owned += $probeProcess
        Start-Sleep -Milliseconds 4000
        if ($first.HasExited) { throw "$name route ended before the kill (exit $($first.ExitCode))." }
        $killQpc = Get-Qpc100ns
        $first.Kill()
        if (-not $first.WaitForExit(5000)) { throw "$name route did not terminate." }
        $killedQpc = Get-Qpc100ns
        Start-Sleep -Milliseconds 3500
        $restartRequestQpc = Get-Qpc100ns
        $second = Start-Route "$name-route2"
        $restartQpc = Get-Qpc100ns
        if (-not $probeProcess.WaitForExit(40000)) { throw "$name probe did not finish." }
        $probeCode = $probeProcess.ExitCode
        $second.Kill()
        [void]$second.WaitForExit(5000)
        $results += [pscustomobject][ordered]@{
            Repeat = $repeat; ProbeCode = $probeCode; KillQpc100ns = $killQpc; KilledQpc100ns = $killedQpc
            RestartRequestQpc100ns = $restartRequestQpc; RestartRunningQpc100ns = $restartQpc
            Route1ExitCode = $first.ExitCode
        }
        Write-Host ("{0}: killed route 1 (exit {1}); route 2 running {2:N0} ms after the kill; probe exit {3}" -f `
            $name, $first.ExitCode, (($restartQpc - $killQpc) / 1.0e4), $probeCode)
        foreach ($process in @($first, $second, $probeProcess)) { $process.Dispose() }
        $script:owned = @()
        Start-Sleep -Milliseconds 3000
    }
} catch { $failure = $_ } finally {
    foreach ($process in $owned) {
        try { if (-not $process.HasExited) { $process.Kill(); [void]$process.WaitForExit(5000) } } finally { $process.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Repeats = $Repeats; Results = $results; Qualification = $false
        Scope = 'A7: product cable route (control plane, CableRouteWorker) force-killed mid-run and restarted; Cable A Input impulses, Cable B Output capture; NEM VM, not qualification. Verdict from tools/vm/review-cable-kill.py' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Kill test recorded. Send the output.'
