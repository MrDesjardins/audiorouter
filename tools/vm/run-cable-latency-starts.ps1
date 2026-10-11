<# Guest-only repeated-start cable latency (VCAB-25 step 2). Each start opens fresh streams,
   so the steady end-to-end level a start settles on is sampled -Starts times per period mode.
   -Route engine: Cable A Input -> driver bridge -> AudioRouter engine (compiled cable-only
   session) -> Cable B Output. -Route vbcable: VB-Cable's own CABLE Input -> CABLE Output, the
   same-VM comparison. The probe reports steady levels in impulse order, so a VM stall shows
   as a second segment instead of jitter. Diagnostic only, never qualification. #>
[CmdletBinding()]
param([Parameter(Mandatory = $true)][ValidateSet('engine', 'vbcable')][string] $Route,
    [ValidateRange(1, 10)][int] $Starts = 5,
    [ValidateRange(200, 1000)][int] $Impulses = 400)
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
$run = "starts-$Route-" + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$tone = Join-Path $bundle 'tools\m03_bridge_tone.exe'
$endpoints = @{
    engine = @{ Render = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'; Capture = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)' }
    vbcable = @{ Render = 'CABLE Input (VB-Audio Virtual Cable)'; Capture = 'CABLE Output (VB-Audio Virtual Cable)' }
}
$modes = @(
    @{ Name = 'default'; Frames = 480; TargetP95Ms = 40.0 },
    @{ Name = 'low-latency'; Frames = 128; TargetP95Ms = 20.0 }
)
$results = @()
$summary = @()
$failure = $null
$active = $null
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
function Get-Median([double[]] $Values) {
    if (-not $Values -or $Values.Count -eq 0) { return $null }
    $sorted = @($Values | Sort-Object)
    return $sorted[[int][Math]::Floor(($sorted.Count - 1) / 2)]
}
try {
    Write-Host "Cable latency starts ($Route): $Starts starts x 2 period modes, $Impulses impulses each. No manual playback needed."
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($Route -eq 'engine' -and $status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    if ($inventory.Code -ne 0) { throw "Probe inventory failed (exit $($inventory.Code)); no audio test started." }
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $renderMatches = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=' + [regex]::Escape($endpoints[$Route].Render) + ' id=')
    $captureMatches = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=' + [regex]::Escape($endpoints[$Route].Capture) + ' id=')
    if ($renderMatches.Count -ne 1 -or $captureMatches.Count -ne 1) {
        if ($Route -eq 'vbcable') { throw 'Expected exactly one active CABLE Input and CABLE Output (VB-Cable). Install VB-Cable in this VM first; no audio test started.' }
        throw 'Expected exactly one active Cable A Input and one Cable B Output; no audio test started.'
    }
    $renderIndex = $renderMatches[0].Groups[1].Value
    $captureIndex = $captureMatches[0].Groups[1].Value
    $probeSeconds = [int][Math]::Ceiling($Impulses / 100.0) + 2
    for ($start = 1; $start -le $Starts; $start++) {
        foreach ($mode in $modes) {
            $name = "start$start-$($mode.Name)"
            $toneCode = $null
            if ($Route -eq 'engine') {
                $toneSeconds = $probeSeconds + 8
                $toneOut = Join-Path $evidence "$name-tone.txt"
                $toneArguments = @('--engine','--frames',[string]$mode.Frames,'--seconds',[string]$toneSeconds,
                    '--out',(Join-Path $evidence "$name-cable-a.wav")) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
                $active = Start-Process -FilePath $tone -ArgumentList ($toneArguments -join ' ') -WindowStyle Hidden -PassThru `
                    -RedirectStandardOutput $toneOut -RedirectStandardError (Join-Path $evidence "$name-tone-stderr.txt")
                $null = $active.Handle
                $watch = [Diagnostics.Stopwatch]::StartNew()
                while (-not ((Test-Path -LiteralPath $toneOut) -and ((Get-Content -LiteralPath $toneOut -Raw) -match 'render worker polling before lease activation'))) {
                    if ($active.HasExited) { throw "$name engine route exited before its leases were active (exit $($active.ExitCode)); see $name-tone-stderr.txt." }
                    if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$name engine route readiness timed out." }
                    Start-Sleep -Milliseconds 50
                }
                Start-Sleep -Milliseconds 1500
                if ($active.HasExited) { throw "$name engine route ended before the probe started (exit $($active.ExitCode))." }
            }
            $probeArguments = @('cable-impulse',[string]$Impulses,$renderIndex,$captureIndex) + @(if ($mode.Name -eq 'low-latency') { 'low-latency' })
            $measured = Invoke-DriverVmProcess -Executable $probe -Arguments $probeArguments -Stdout (Join-Path $evidence "$name-probe.txt") `
                -Stderr (Join-Path $evidence "$name-probe-stderr.txt") -TimeoutSeconds ($probeSeconds + 30)
            $toneText = ''
            if ($active) {
                if (-not $active.WaitForExit(($toneSeconds + 30) * 1000)) { throw "$name engine route did not finish within its watchdog." }
                $toneCode = $active.ExitCode
                $active.Dispose()
                $active = $null
                $toneText = Get-Content -LiteralPath $toneOut -Raw
            }
            $probeText = Get-Content -LiteralPath (Join-Path $evidence "$name-probe.txt") -Raw
            $result = [pscustomobject][ordered]@{
                Start = $start; Mode = $mode.Name; Frames = $mode.Frames; ProbeCode = $measured.Code; ToneCode = $toneCode
                SteadyLevelMs = Get-ProbeValue $probeText 'cable_steady_level_ms'; SteadySpreadMs = Get-ProbeValue $probeText 'cable_steady_spread_ms'
                SteadyFirstImpulse = Get-ProbeValue $probeText 'cable_steady_first_impulse'; Segments = Get-ProbeValue $probeText 'cable_segments'
                P50Ms = Get-ProbeValue $probeText 'cable_latency_p50_ms'; P95Ms = Get-ProbeValue $probeText 'cable_latency_p95_ms'
                JitterMs = Get-ProbeValue $probeText 'cable_jitter_p99_minus_p1_ms'
                Lost = Get-ProbeValue $probeText 'cable_impulses_lost'; Corrupted = Get-ProbeValue $probeText 'cable_impulses_corrupted'
                BufferFrames = Get-ProbeValue $probeText 'cable_render_buffer_frames'
                CaptureDropped = Get-ProbeValue $probeText 'cable_capture_dropped_frames'
                RenderDiscontinuities = Get-ProbeValue $probeText 'cable_render_discontinuities'
                RenderDiscontinuityMs = Get-ProbeValue $probeText 'cable_render_discontinuity_ms'
                EndAnchorP50Ms = Get-ProbeValue $probeText 'cable_end_anchor_p50_ms'
                Engine = ([regex]::Match($toneText, '(?m)^engine route quanta: .*$')).Value
                Counters = @([regex]::Matches($toneText, '(?m)^(capture-sink|render-source) (counters|packet writes) \(.*$') | ForEach-Object { $_.Value.TrimEnd() })
            }
            # Clean: nothing lost or dropped and the render stream never stalled
            # or skipped; only clean starts enter the comparison median.
            $result | Add-Member NoteProperty Clean ($measured.Code -eq 0 -and $result.Lost -eq 0 -and $result.CaptureDropped -eq 0 -and
                $result.RenderDiscontinuities -eq 0 -and $null -ne $result.P50Ms)
            $results += $result
            Write-Host ("{0}: steady {1} ms (spread {2} ms, from impulse {3}), segments {4}, p50 {5} ms, p95 {6} ms, lost {7}, dropped {8}, render stalls {9} ({10} ms){11}; probe exit {12}{13}" -f `
                $name, $result.SteadyLevelMs, $result.SteadySpreadMs, $result.SteadyFirstImpulse, $result.Segments, $result.P50Ms, $result.P95Ms,
                $result.Lost, $result.CaptureDropped, $result.RenderDiscontinuities, $result.RenderDiscontinuityMs, $(if ($result.Clean) { ', clean' } else { '' }), $measured.Code, $(if ($null -ne $toneCode) { ", route exit $toneCode" } else { '' }))
            if ($result.Engine) { Write-Host "  $($result.Engine)" }
        }
    }
    foreach ($mode in $modes) {
        $runs = @($results | Where-Object { $_.Mode -eq $mode.Name })
        $levels = @($runs | Where-Object { $null -ne $_.SteadyLevelMs } | ForEach-Object { [double]$_.SteadyLevelMs })
        $cleanMedians = @($runs | Where-Object { $_.Clean } | ForEach-Object { [double]$_.P50Ms })
        $entry = [pscustomobject][ordered]@{
            Mode = $mode.Name; Starts = $runs.Count; StartsWithSteadyLevel = $levels.Count
            SteadyLevelsMs = $levels; MedianSteadyMs = Get-Median $levels
            MinSteadyMs = $(if ($levels.Count) { ($levels | Measure-Object -Minimum).Minimum }); MaxSteadyMs = $(if ($levels.Count) { ($levels | Measure-Object -Maximum).Maximum })
            StartsWithShift = @($runs | Where-Object { $_.Segments -gt 1 }).Count
            CleanStarts = $cleanMedians.Count; CleanP50sMs = $cleanMedians; MedianCleanP50Ms = Get-Median $cleanMedians
            LostTotal = ($runs | Measure-Object -Property Lost -Sum).Sum; CorruptedTotal = ($runs | Measure-Object -Property Corrupted -Sum).Sum
            TargetP95Ms = $mode.TargetP95Ms
        }
        $summary += $entry
        Write-Host ("{0} periods: clean starts {1} of {2}, their p50s {3} ms, median {4} ms (VCAB-25 target <= {5}); steady levels {6} ms; {7} starts shifted level mid-run; lost {8}, corrupted {9}" -f `
            $mode.Name, $entry.CleanStarts, $entry.Starts, ($cleanMedians -join ', '), $entry.MedianCleanP50Ms, $mode.TargetP95Ms,
            ($levels -join ', '), $entry.StartsWithShift, $entry.LostTotal, $entry.CorruptedTotal)
    }
    if (@($results | Where-Object { $_.ProbeCode -ne 0 -or ($null -ne $_.ToneCode -and $_.ToneCode -ne 0) }).Count -gt 0) {
        throw 'A probe or route process failed; see the reports.'
    }
} catch { $failure = $_ } finally {
    if ($active) {
        try {
            if (-not $active.HasExited) {
                if (-not $active.WaitForExit(30000)) { $active.Kill(); [void]$active.WaitForExit(5000) }
            }
        } finally { $active.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Route = $Route; Passed = (-not [bool]$failure); Error = [string]$failure
        Starts = $Starts; Impulses = $Impulses; Results = $results; Summary = $summary; Qualification = $false
        Scope = $(if ($Route -eq 'engine') { 'Cable A Input -> driver bridge -> AudioRouter engine (compiled cable-only session) -> Cable B Output, repeated starts; NEM VM, not qualification' }
            else { 'VB-Cable CABLE Input -> CABLE Output (no route), repeated starts; same-VM comparison for VCAB-25, NEM VM, not qualification' }) }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Repeated-start latency measured. Send the output.'
