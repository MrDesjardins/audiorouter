<# Bounded guest cable latency (VCAB-25), bit-exactness (VCAB-20) and isolation (VCAB-26) diagnostic
   on the already installed driver. Cable A Input -> driver bridge -> route -> Cable B Output,
   measured by the native probe. -Route proxy: diagnostic pass-through relay. -Route engine:
   the AudioRouter engine runs a compiled cable-only session between the cables (not yet the
   backend lifecycle). Neither is qualification. #>
[CmdletBinding()]
param([ValidateRange(100, 3000)][int] $Impulses = 1000,
    [ValidateSet('proxy', 'engine')][string] $Route = 'proxy')
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
$otherRenderName = 'AudioRouter Cable B Input (AudioRouter Virtual Cable)'
$quietCaptureName = 'AudioRouter Cable A Output (AudioRouter Virtual Cable)'
# Each configuration: bridge quantum and WASAPI engine period request, with
# the VCAB-25 p95 target that applies to it.
$configurations = @(
    @{ Name = 'default-480'; Frames = 480; Mode = 'default'; TargetP95Ms = 40.0 },
    @{ Name = 'low-latency-128'; Frames = 128; Mode = 'low-latency'; TargetP95Ms = 20.0 }
)
$routeFlag = if ($Route -eq 'engine') { '--engine' } else { '--passthrough' }
$routeLabel = if ($Route -eq 'engine') { 'engine route' } else { 'proxy route' }
$results = @()
$bitExact = $null
$isolation = @()
$failure = $null
$active = $null
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
try {
    Write-Host "Cable latency diagnostic (): Cable A Input -> route -> Cable B Output, two configurations. No manual playback needed."
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
    $otherRenderMatches = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=' + [regex]::Escape($otherRenderName) + ' id=')
    $quietCaptureMatches = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=' + [regex]::Escape($quietCaptureName) + ' id=')
    if ($otherRenderMatches.Count -ne 1 -or $quietCaptureMatches.Count -ne 1) { throw 'Expected exactly one active Cable B Input and one Cable A Output; no audio test started.' }
    $otherRenderIndex = $otherRenderMatches[0].Groups[1].Value
    $quietCaptureIndex = $quietCaptureMatches[0].Groups[1].Value
    $probeSeconds = [int][Math]::Ceiling($Impulses / 100.0) + 2
    foreach ($configuration in $configurations) {
        $name = $configuration.Name
        $toneSeconds = $probeSeconds + 8
        $toneOut = Join-Path $evidence "$name-tone.txt"
        $toneArguments = @($routeFlag,'--frames',[string]$configuration.Frames,'--seconds',[string]$toneSeconds,
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
            Engine = ([regex]::Match($toneText, '(?m)^engine route quanta: .*$')).Value
            Counters = @([regex]::Matches($toneText, '(?m)^(capture-sink|render-source) (counters|packet writes) \(.*$') | ForEach-Object { $_.Value.TrimEnd() })
            TargetP95Ms = $configuration.TargetP95Ms; TargetJitterMs = 2.0
            MeetsTargets = ($measured.Code -eq 0 -and $null -ne $p95 -and $p95 -le $configuration.TargetP95Ms -and
                $null -ne $jitter -and $jitter -le 2.0 -and $lost -eq 0 -and $corrupted -eq 0)
        }
        $results += [pscustomobject]$result
        Write-Host ("{0}: p50 {1} ms, p95 {2} ms (target <= {3}), jitter p99-p1 {4} ms (target <= 2), lost {5}, corrupted {6}; probe exit {7}, pass-through exit {8}" -f `
            $name, $result.P50Ms, $p95, $configuration.TargetP95Ms, $jitter, $lost, $corrupted, $measured.Code, $toneCode)
        if ($result.Relay) { Write-Host "  $($result.Relay)" }
        if ($result.Engine) { Write-Host "  $($result.Engine)" }
        $result.Counters | ForEach-Object { Write-Host "  $_" }
    }
    # Bit-exactness through the same 480-frame pass-through: 10 s of seeded
    # noise then 0.5 s of silence; every sample must match, silence must be +0.0.
    $name = 'bitexact-480'
    $toneOut = Join-Path $evidence "$name-tone.txt"
    $toneArguments = @($routeFlag,'--frames','480','--seconds','20','--out',(Join-Path $evidence "$name-cable-a.wav")) |
        ForEach-Object { ConvertTo-DriverProcessArgument $_ }
    $active = Start-Process -FilePath $tone -ArgumentList ($toneArguments -join ' ') -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $toneOut -RedirectStandardError (Join-Path $evidence "$name-tone-stderr.txt")
    $null = $active.Handle
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $toneOut) -and ((Get-Content -LiteralPath $toneOut -Raw) -match 'render worker polling before lease activation'))) {
        if ($active.HasExited) { throw "$name pass-through exited before its leases were active (exit $($active.ExitCode)); see $name-tone-stderr.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$name pass-through readiness timed out." }
        Start-Sleep -Milliseconds 50
    }
    Start-Sleep -Milliseconds 1500
    if ($active.HasExited) { throw "$name pass-through ended before the probe started (exit $($active.ExitCode))." }
    $measured = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-bitexact','10',$renderIndex,$captureIndex) `
        -Stdout (Join-Path $evidence "$name-probe.txt") -Stderr (Join-Path $evidence "$name-probe-stderr.txt") -TimeoutSeconds 45
    if (-not $active.WaitForExit(50000)) { throw "$name pass-through did not finish within its watchdog." }
    $toneCode = $active.ExitCode
    $active.Dispose()
    $active = $null
    $probeText = Get-Content -LiteralPath (Join-Path $evidence "$name-probe.txt") -Raw
    $bitExact = [pscustomobject][ordered]@{
        Name = $name; ProbeCode = $measured.Code; ToneCode = $toneCode
        Passed = ($measured.Code -eq 0 -and (Get-ProbeValue $probeText 'bitexact_pass') -eq 1)
        Aligned = Get-ProbeValue $probeText 'bitexact_aligned'; ComparedFrames = Get-ProbeValue $probeText 'bitexact_compared_frames'
        MismatchedSamples = Get-ProbeValue $probeText 'bitexact_mismatched_samples'; MaxAbsDiff = Get-ProbeValue $probeText 'bitexact_max_abs_diff'
        SilenceNonzero = Get-ProbeValue $probeText 'bitexact_silence_nonzero_samples'
        Relay = ([regex]::Match((Get-Content -LiteralPath $toneOut -Raw), '(?m)^pass-through relay: .*$')).Value
        Engine = ([regex]::Match((Get-Content -LiteralPath $toneOut -Raw), '(?m)^engine route quanta: .*$')).Value
    }
    Write-Host ("{0}: {1}; compared {2} frames, mismatched samples {3}, max diff {4}, non-zero silence samples {5}; probe exit {6}, pass-through exit {7}" -f `
        $name, $(if ($bitExact.Passed) { 'bit-exact' } else { 'NOT bit-exact' }), $bitExact.ComparedFrames, $bitExact.MismatchedSamples,
        $bitExact.MaxAbsDiff, $bitExact.SilenceNonzero, $measured.Code, $toneCode)
    # Isolation: the tone tool in its normal mode (tone on Cable B Output,
    # Cable A Input consumed). Nothing routes into Cable A Output, so it must
    # stay exact silence while noise plays into Cable A Input and Cable B Input.
    $name = 'isolation'
    $toneOut = Join-Path $evidence "$name-tone.txt"
    $toneArguments = @('--frames','480','--seconds','25','--out',(Join-Path $evidence "$name-cable-a.wav")) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
    $active = Start-Process -FilePath $tone -ArgumentList ($toneArguments -join ' ') -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $toneOut -RedirectStandardError (Join-Path $evidence "$name-tone-stderr.txt")
    $null = $active.Handle
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $toneOut) -and ((Get-Content -LiteralPath $toneOut -Raw) -match 'render worker polling before lease activation'))) {
        if ($active.HasExited) { throw "$name tone tool exited before its leases were active (exit $($active.ExitCode)); see $name-tone-stderr.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$name tone tool readiness timed out." }
        Start-Sleep -Milliseconds 50
    }
    Start-Sleep -Milliseconds 1500
    foreach ($pair in @(@{ Label = 'A-Input-to-A-Output'; Render = $renderIndex }, @{ Label = 'B-Input-to-A-Output'; Render = $otherRenderIndex })) {
        if ($active.HasExited) { throw "$name tone tool ended before the $($pair.Label) probe (exit $($active.ExitCode))." }
        # Record Cable B Output too (discarded) so Cable B genuinely carries its tone.
        $measured = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-isolation','5',$pair.Render,$quietCaptureIndex,$captureIndex) `
            -Stdout (Join-Path $evidence "$name-$($pair.Label).txt") -Stderr (Join-Path $evidence "$name-$($pair.Label)-stderr.txt") -TimeoutSeconds 30
        $probeText = Get-Content -LiteralPath (Join-Path $evidence "$name-$($pair.Label).txt") -Raw
        $isolation += [pscustomobject][ordered]@{
            Path = $pair.Label; ProbeCode = $measured.Code
            Passed = ($measured.Code -eq 0 -and (Get-ProbeValue $probeText 'isolation_pass') -eq 1 -and
                (Get-ProbeValue $probeText 'isolation_drain_active_frames') -gt 0)
            CableBActiveFrames = Get-ProbeValue $probeText 'isolation_drain_active_frames'
            Frames = Get-ProbeValue $probeText 'isolation_frames'; NonzeroSamples = Get-ProbeValue $probeText 'isolation_nonzero_samples'
            PeakDbfs = Get-ProbeValue $probeText 'isolation_peak_dbfs'
        }
        Write-Host ("isolation {0}: {1}; frames {2}, non-zero samples {3}, Cable B active frames {4}; probe exit {5}" -f $pair.Label,
            $(if ($isolation[-1].Passed) { 'exact silence while Cable B was active' } elseif ($isolation[-1].NonzeroSamples -gt 0) { 'LEAK' } else { 'Cable B was not active; inconclusive' }),
            $isolation[-1].Frames, $isolation[-1].NonzeroSamples, $isolation[-1].CableBActiveFrames, $measured.Code)
    }
    if (-not $active.WaitForExit(60000)) { throw "$name tone tool did not finish within its watchdog." }
    $isolationToneCode = $active.ExitCode
    $active.Dispose()
    $active = $null
    if (@($results | Where-Object { $_.ProbeCode -ne 0 -or $_.ToneCode -ne 0 }).Count -gt 0 -or $bitExact.ProbeCode -ne 0 -or $bitExact.ToneCode -ne 0 -or
        @($isolation | Where-Object { $_.ProbeCode -ne 0 }).Count -gt 0 -or $isolationToneCode -ne 0) {
        throw 'A probe or pass-through process failed; see the reports.'
    }
} catch { $failure = $_ } finally {
    if ($active) {
        try {
            if (-not $active.HasExited) {
                if (-not $active.WaitForExit(30000)) { $active.Kill(); [void]$active.WaitForExit(5000) }
            }
        } finally { $active.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Route = $Route; Results = $results; BitExact = $bitExact; Isolation = $isolation; Qualification = $false
        Scope = $(if ($Route -eq 'engine') { 'Cable A -> driver bridge -> AudioRouter engine (compiled cable-only session, 128-frame quanta) -> Cable B; not the backend lifecycle or a VB-Cable comparison' }
            else { 'Cable A -> driver bridge -> diagnostic pass-through relay -> Cable B; AudioRouter route proxy, not product-engine latency or VB-Cable comparison' }) }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
if (@($results | Where-Object { -not $_.MeetsTargets }).Count -gt 0) {
    Write-Host 'Latency measured; at least one configuration misses a VCAB-25 target (see above).'
} else {
    Write-Host "Latency measured; both configurations meet the VCAB-25 p95/jitter targets for this $routeLabel."
}
if ($bitExact.Passed) { Write-Host "Bit-exact through the $routeLabel (VCAB-20 check at the endpoint format)." }
else { Write-Host "NOT bit-exact through the $routeLabel (see above)." }
if (@($isolation | Where-Object { -not $_.Passed }).Count -eq 0 -and $isolation.Count -eq 2) { Write-Host 'Cable A Output stayed exact silence (VCAB-26 isolation check).' }
else { Write-Host 'Isolation check failed (see above).' }
Write-Host 'Send the output.'
