<# Guest-only latency comparison with VB-Cable (VCAB-25, NFR-02), same VM, same probe.
   VB-Cable must be installed in the guest first (vb-audio.com/Cable; a person installs
   it, then restores the checkpoint afterwards). It is a stand-alone cable, so the probe
   plays impulses into "CABLE Input" and records "CABLE Output" directly. The AudioRouter
   cable carries audio through the engine by design (Cable A Input -> product route ->
   Cable B Output, m03_cable_session). The two are measured alternately, -Rounds times
   each, so guest drift hits both; only the RATIO is meaningful in this VM, the absolute
   numbers are VM numbers. Diagnostic only, never qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 5)][int] $Rounds = 3,
    [ValidateRange(300, 3000)][int] $Impulses = 1000)
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
$run = 'vbcompare-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$arRender = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$arCapture = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
$vbRender = 'CABLE Input (VB-Audio Virtual Cable)'
$vbCapture = 'CABLE Output (VB-Audio Virtual Cable)'
$failure = $null
$owned = @()
$rows = @()
function Find-Index([string] $Text, [string] $Flow, [string] $Name) {
    $found = [regex]::Matches($Text, "(?m)^$Flow\[(\d+)\] name=" + [regex]::Escape($Name) + ' id=')
    if ($found.Count -ne 1) { throw "Expected exactly one active '$Name' ($($found.Count) found)." }
    return $found[0].Groups[1].Value
}
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
function Get-Median([double[]] $Values) {
    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) { return $null }
    $middle = [int][Math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2) { return $sorted[$middle] }
    return ($sorted[$middle - 1] + $sorted[$middle]) / 2
}
try {
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    if ($inventory.Code -ne 0) { throw "Probe inventory failed (exit $($inventory.Code))." }
    $text = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    if ($text -notmatch [regex]::Escape($vbRender)) { throw 'VB-Cable is not installed in this guest (no "CABLE Input (VB-Audio Virtual Cable)"). Install it from vb-audio.com/Cable, restart, run again, then restore the checkpoint.' }
    $arRenderIndex = Find-Index $text 'render' $arRender
    $arCaptureIndex = Find-Index $text 'capture' $arCapture
    $vbRenderIndex = Find-Index $text 'render' $vbRender
    $vbCaptureIndex = Find-Index $text 'capture' $vbCapture
    $probeSeconds = [int][Math]::Ceiling($Impulses / 100.0) + 3
    for ($round = 1; $round -le $Rounds; $round++) {
        foreach ($target in 'vb-cable', 'audiorouter') {
            $name = "$target-$round"
            $route = $null
            if ($target -eq 'audiorouter') {
                $routeOut = Join-Path $evidence "$name-route.txt"
                $route = Start-Process -FilePath $cableSession -ArgumentList ('"--seconds" "' + ($probeSeconds + 10) + '"') -WindowStyle Hidden -PassThru `
                    -RedirectStandardOutput $routeOut -RedirectStandardError (Join-Path $evidence "$name-route-stderr.txt")
                $null = $route.Handle
                $owned += $route
                $watch = [Diagnostics.Stopwatch]::StartNew()
                while (-not ((Test-Path -LiteralPath $routeOut) -and ((Get-Content -LiteralPath $routeOut -Raw) -match 'cable route running'))) {
                    if ($route.HasExited) { throw "$name route exited before it ran (exit $($route.ExitCode))." }
                    if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$name route readiness timed out." }
                    Start-Sleep -Milliseconds 20
                }
                Start-Sleep -Milliseconds 1500
            }
            $renderIndex = if ($target -eq 'vb-cable') { $vbRenderIndex } else { $arRenderIndex }
            $captureIndex = if ($target -eq 'vb-cable') { $vbCaptureIndex } else { $arCaptureIndex }
            $measured = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-impulse',[string]$Impulses,$renderIndex,$captureIndex) `
                -Stdout (Join-Path $evidence "$name-probe.txt") -Stderr (Join-Path $evidence "$name-probe-stderr.txt") -TimeoutSeconds ($probeSeconds + 30)
            if ($route) {
                [void]$route.WaitForExit(30000)
                if (-not $route.HasExited) { $route.Kill(); [void]$route.WaitForExit(5000) }
                $route.Dispose(); $route = $null; $owned = @()
            }
            $probeText = Get-Content -LiteralPath (Join-Path $evidence "$name-probe.txt") -Raw
            $row = [pscustomobject][ordered]@{
                Target = $target; Round = $round; ProbeCode = $measured.Code
                P50Ms = Get-ProbeValue $probeText 'cable_latency_p50_ms'; P95Ms = Get-ProbeValue $probeText 'cable_latency_p95_ms'
                JitterMs = Get-ProbeValue $probeText 'cable_jitter_p99_minus_p1_ms'
                Lost = Get-ProbeValue $probeText 'cable_impulses_lost'; Corrupted = Get-ProbeValue $probeText 'cable_impulses_corrupted'
            }
            $rows += $row
            Write-Host ("{0}: p50 {1} ms, p95 {2} ms, jitter {3} ms, lost {4}, corrupted {5}; probe exit {6}" -f $name, $row.P50Ms, $row.P95Ms, $row.JitterMs, $row.Lost, $row.Corrupted, $row.ProbeCode)
            Start-Sleep -Seconds 2
        }
    }
    $valid = @($rows | Where-Object { $null -ne $_.P50Ms -and $null -ne $_.P95Ms })
    if ($valid.Count -ne $rows.Count) { throw 'At least one probe run produced no latency numbers; see the *-probe.txt files.' }
} catch { $failure = $_ } finally {
    foreach ($process in $owned) {
        try { if (-not $process.HasExited) { $process.Kill(); [void]$process.WaitForExit(5000) } } finally { $process.Dispose() }
    }
    $summary = [ordered]@{}
    foreach ($target in 'vb-cable', 'audiorouter') {
        $set = @($rows | Where-Object { $_.Target -eq $target -and $null -ne $_.P50Ms })
        $summary[$target] = [ordered]@{
            Runs = $set.Count
            MedianP50Ms = Get-Median @($set | ForEach-Object { [double]$_.P50Ms })
            MedianP95Ms = Get-Median @($set | ForEach-Object { [double]$_.P95Ms })
            MedianJitterMs = Get-Median @($set | ForEach-Object { [double]$_.JitterMs })
        }
    }
    $ratio = $null
    if ($summary['vb-cable'].MedianP50Ms -and $summary['audiorouter'].MedianP50Ms) {
        $ratio = [Math]::Round($summary['audiorouter'].MedianP50Ms / $summary['vb-cable'].MedianP50Ms, 2)
        Write-Host ("VB-Cable median p50 {0} ms / p95 {1} ms; AudioRouter median p50 {2} ms / p95 {3} ms; ratio (AudioRouter / VB-Cable, p50) {4}. VM numbers, only the ratio counts." -f `
            $summary['vb-cable'].MedianP50Ms, $summary['vb-cable'].MedianP95Ms, $summary['audiorouter'].MedianP50Ms, $summary['audiorouter'].MedianP95Ms, $ratio)
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Rounds = $Rounds; Impulses = $Impulses; Results = $rows; Summary = $summary; P50Ratio = $ratio; Qualification = $false
        Scope = 'VCAB-25 comparison: probe impulses through VB-Cable directly and through the AudioRouter cable engine route, alternating, same VM; NEM VM, ratio only, not qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'VB-Cable comparison recorded. Send the output.'
