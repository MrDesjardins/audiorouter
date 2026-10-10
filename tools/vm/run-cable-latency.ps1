<# Bounded guest cable latency diagnostic (VCAB-25 preparation) on the already installed driver.
   Cable A Input -> driver bridge -> pass-through relay (an AudioRouter route proxy)
   -> Cable B Output, timed by the native impulse probe. Not product-engine latency. #>
[CmdletBinding()]
param([ValidateRange(100, 3000)][int] $Impulses = 1000)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 1GB) { throw 'Requires 1 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$run = 'latency-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$tone = Join-Path $bundle 'tools\m03_bridge_tone.exe'
$renderName = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$captureName = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
# Each configuration: bridge quantum and WASAPI engine period request, with
# the VCAB-25 p95 target that applies to it.
$configurations = @(
    @{ Name = 'default-480'; Frames = 480; Mode = 'default'; TargetP95Ms = 40.0 },
    @{ Name = 'low-latency-128'; Frames = 128; Mode = 'low-latency'; TargetP95Ms = 20.0 }
)
$results = @()
$failure = $null
$active = $null
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
try {
    Write-Host 'Cable latency diagnostic: Cable A Input -> pass-through -> Cable B Output, two configurations. No manual playback needed.'
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    if ($inventory.Code -ne 0) { throw "Probe inventory failed (exit $($inventory.Code)); no audio test started." }
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $renderMatches = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=' + [regex]::Escape($renderName) + ' id=')
    $captureMatches = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=' + [regex]::Escape($captureName) + ' id=')
    if ($renderMatches.Count -ne 1 -or $captureMatches.Count -ne 1) { throw 'Expected exactly one active Cable A Input and one Cable B Output; no audio test started.' }
    $renderIndex = $renderMatches[0].Groups[1].Value
    $captureIndex = $captureMatches[0].Groups[1].Value
    $probeSeconds = [int][Math]::Ceiling($Impulses / 100.0) + 2
    foreach ($configuration in $configurations) {
        $name = $configuration.Name
        $toneSeconds = $probeSeconds + 8
        $toneOut = Join-Path $evidence "$name-tone.txt"
        $toneArguments = @('--passthrough','--frames',[string]$configuration.Frames,'--seconds',[string]$toneSeconds,
            '--out',(Join-Path $evidence "$name-cable-a.wav")) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
        $active = Start-Process -FilePath $tone -ArgumentList ($toneArguments -join ' ') -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput $toneOut -RedirectStandardError (Join-Path $evidence "$name-tone-stderr.txt")
        $null = $active.Handle
        $watch = [Diagnostics.Stopwatch]::StartNew()
        while (-not ((Test-Path -LiteralPath $toneOut) -and ((Get-Content -LiteralPath $toneOut -Raw) -match 'render worker polling before lease activation'))) {
            if ($active.HasExited) { throw "$name pass-through exited before its leases were active (exit $($active.ExitCode)); see $name-tone-stderr.txt." }
            if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$name pass-through readiness timed out." }
            Start-Sleep -Milliseconds 50
        }
        # Lease activation follows that line within milliseconds; allow the
        # first relay blocks to flow before the probe opens its streams.
        Start-Sleep -Milliseconds 1500
        if ($active.HasExited) { throw "$name pass-through ended before the probe started (exit $($active.ExitCode))." }
        $probeArguments = @('cable-impulse',[string]$Impulses,$renderIndex,$captureIndex) + @(if ($configuration.Mode -eq 'low-latency') { 'low-latency' })
        $measured = Invoke-DriverVmProcess -Executable $probe -Arguments $probeArguments -Stdout (Join-Path $evidence "$name-probe.txt") `
            -Stderr (Join-Path $evidence "$name-probe-stderr.txt") -TimeoutSeconds ($probeSeconds + 30)
        if (-not $active.WaitForExit(($toneSeconds + 30) * 1000)) { throw "$name pass-through did not finish within its watchdog." }
        $toneCode = $active.ExitCode
        $active.Dispose()
        $active = $null
        $probeText = Get-Content -LiteralPath (Join-Path $evidence "$name-probe.txt") -Raw
        $toneText = Get-Content -LiteralPath $toneOut -Raw
        $p95 = Get-ProbeValue $probeText 'cable_latency_p95_ms'
        $jitter = Get-ProbeValue $probeText 'cable_jitter_p99_minus_p1_ms'
        $lost = Get-ProbeValue $probeText 'cable_impulses_lost'
        $corrupted = Get-ProbeValue $probeText 'cable_impulses_corrupted'
        $result = [ordered]@{
            Name = $name; Frames = $configuration.Frames; Mode = $configuration.Mode
            ProbeCode = $measured.Code; ToneCode = $toneCode
            P1Ms = Get-ProbeValue $probeText 'cable_latency_p1_ms'; P50Ms = Get-ProbeValue $probeText 'cable_latency_p50_ms'
            P95Ms = $p95; P99Ms = Get-ProbeValue $probeText 'cable_latency_p99_ms'; MaxMs = Get-ProbeValue $probeText 'cable_latency_max_ms'
            JitterMs = $jitter; Emitted = Get-ProbeValue $probeText 'cable_impulses_emitted'; Lost = $lost; Corrupted = $corrupted
            Relay = ([regex]::Match($toneText, '(?m)^pass-through relay: .*$')).Value
            Counters = @([regex]::Matches($toneText, '(?m)^(capture-sink|render-source) (counters|packet writes) \(.*$') | ForEach-Object { $_.Value.TrimEnd() })
            TargetP95Ms = $configuration.TargetP95Ms; TargetJitterMs = 2.0
            MeetsTargets = ($measured.Code -eq 0 -and $null -ne $p95 -and $p95 -le $configuration.TargetP95Ms -and
                $null -ne $jitter -and $jitter -le 2.0 -and $lost -eq 0 -and $corrupted -eq 0)
        }
        $results += [pscustomobject]$result
        Write-Host ("{0}: p50 {1} ms, p95 {2} ms (target <= {3}), jitter p99-p1 {4} ms (target <= 2), lost {5}, corrupted {6}; probe exit {7}, pass-through exit {8}" -f `
            $name, $result.P50Ms, $p95, $configuration.TargetP95Ms, $jitter, $lost, $corrupted, $measured.Code, $toneCode)
        if ($result.Relay) { Write-Host "  $($result.Relay)" }
        $result.Counters | ForEach-Object { Write-Host "  $_" }
    }
    if (@($results | Where-Object { $_.ProbeCode -ne 0 -or $_.ToneCode -ne 0 }).Count -gt 0) { throw 'A probe or pass-through process failed; see the reports.' }
} catch { $failure = $_ } finally {
    if ($active) {
        try {
            if (-not $active.HasExited) {
                if (-not $active.WaitForExit(30000)) { $active.Kill(); [void]$active.WaitForExit(5000) }
            }
        } finally { $active.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Results = $results; Qualification = $false
        Scope = 'Cable A -> driver bridge -> diagnostic pass-through relay -> Cable B; AudioRouter route proxy, not product-engine latency or VB-Cable comparison' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
if (@($results | Where-Object { -not $_.MeetsTargets }).Count -gt 0) {
    Write-Host 'Latency measured; at least one configuration misses a VCAB-25 target (see above). Send the output.'
} else {
    Write-Host 'Latency measured; both configurations meet the VCAB-25 p95/jitter targets for this proxy route. Send the output.'
}
