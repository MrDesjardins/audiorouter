<# Measure relative 20 Hz–20 kHz passband magnitude through the product cable route.
   Diagnostic subset of VCAB-22; phase and alias products are not measured. #>
[CmdletBinding()]
param([ValidateRange(1,8)][int] $Channels = 2,
    [ValidateSet(44100,48000,96000)][int] $Rate = 48000,
    [ValidateRange(3,60)][int] $Seconds = 10,
    [ValidateRange(0.01,1.0)][double] $MaxDeviationDb = 0.05)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if (Get-Process -Name 'm03_bridge_tone','m03_cable_session','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$probe = Join-Path $bundle 'tools\m00-probe.exe'; $quality = Join-Path $bundle 'tools\m03-cable-quality.exe'
$cableSession = Join-Path $bundle 'tools\m03_cable_session.exe'
foreach ($file in @($probe,$quality,$cableSession)) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Bundle tool missing: $file" } }
$run = "sweep-$Rate-hz-$Channels-ch-" + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run; New-Item -ItemType Directory -Path $evidence | Out-Null
$route = $null; $failure = $null; $result = $null
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
try {
    $status = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $render = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=AudioRouter Cable A Input \(AudioRouter Virtual Cable\) id=')
    $capture = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=AudioRouter Cable B Output \(AudioRouter Virtual Cable\) id=')
    if ($inventory.Code -ne 0 -or $render.Count -ne 1 -or $capture.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output; no audio test started.' }
    $routeOut = Join-Path $evidence 'route.txt'
    $route = Start-Process -FilePath $cableSession -ArgumentList ((@('--channels',[string]$Channels,'--seconds',[string]($Seconds + 15)) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -join ' ') `
        -WindowStyle Hidden -PassThru -RedirectStandardOutput $routeOut -RedirectStandardError (Join-Path $evidence 'route-stderr.txt')
    $null = $route.Handle; $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $routeOut) -and ((Get-Content -LiteralPath $routeOut -Raw) -match 'cable route running'))) {
        if ($route.HasExited) { throw "Product route exited early ($($route.ExitCode))." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw 'Product route readiness timed out.' }
        Start-Sleep -Milliseconds 50
    }
    Start-Sleep -Milliseconds 1500
    $probeRun = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-sweep',[string]$Seconds,$render[0].Groups[1].Value,$capture[0].Groups[1].Value,$evidence,'20','20000') `
        -Stdout (Join-Path $evidence 'probe.txt') -Stderr (Join-Path $evidence 'probe-stderr.txt') -TimeoutSeconds ($Seconds + 60)
    if (-not $route.WaitForExit(($Seconds + 60) * 1000)) { throw 'Product route did not finish within its watchdog.' }
    $probeText = Get-Content -LiteralPath (Join-Path $evidence 'probe.txt') -Raw
    $reference = Join-Path $evidence 'cable-sweep-reference.wav'; $captureWav = Join-Path $evidence 'cable-sweep-capture.wav'
    if ($probeRun.Code -ne 0 -or -not (Test-Path -LiteralPath $reference) -or -not (Test-Path -LiteralPath $captureWav)) { throw 'Native probe failed or did not produce both sweep WAVs.' }
    $formatRate = Get-ProbeValue $probeText 'bitexact_rate'
    $formatChannels = Get-ProbeValue $probeText 'bitexact_channels'
    $formatEncoding = ([regex]::Match($probeText, '(?m)\bbitexact_encoding=([a-z0-9-]+)')).Groups[1].Value
    $dropped = Get-ProbeValue $probeText 'bitexact_capture_dropped_frames'
    $flagged = Get-ProbeValue $probeText 'bitexact_capture_flagged_packets'
    $overflow = Get-ProbeValue $probeText 'bitexact_capture_overflow_packets'
    if ($formatRate -ne $Rate -or $formatChannels -ne $Channels -or $formatEncoding -ne 'float32') { throw "Probe negotiated $formatChannels channels, $formatRate Hz, $formatEncoding; expected $Channels channels, $Rate Hz, float32." }
    # Every capture flags its first packet (start discontinuity); more is a loss.
    if ($dropped -ne 0 -or $flagged -gt 1 -or $overflow -ne 0) { throw "Capture was not clean (dropped $dropped, flagged $flagged, overflow $overflow)." }
    $metricsPath = Join-Path $evidence 'metrics.json'
    $analysis = Invoke-DriverVmProcess -Executable $quality -Arguments @('sweep',$reference,$captureWav,$metricsPath,'20','20000',[string]$MaxDeviationDb) `
        -Stdout (Join-Path $evidence 'analysis.txt') -Stderr (Join-Path $evidence 'analysis-stderr.txt') -TimeoutSeconds 120
    if ($analysis.Code -ne 0 -or -not (Test-Path -LiteralPath $metricsPath)) { throw 'Rust sweep response analyzer failed.' }
    $metrics = Get-Content -LiteralPath $metricsPath -Raw | ConvertFrom-Json
    $result = [pscustomobject]@{ Rate=$formatRate; Channels=$formatChannels; Encoding=$formatEncoding; Seconds=$Seconds
        MaxDeviationDb=$MaxDeviationDb; WorstDeviationDb=$metrics.worstDeviationDb; AnalyzerPass=$metrics.pass
        AnalysisPoints=$metrics.analysisPointsPerChannel; Metrics='metrics.json'; CaptureDropped=$dropped; CaptureFlagged=$flagged
        CaptureOverflow=$overflow; ProbeExit=$probeRun.Code; RouteExit=$route.ExitCode; Qualification=$false }
    if (-not $metrics.pass -or $route.ExitCode -ne 0) { throw "Sweep response diagnostic failed (deviation $($metrics.worstDeviationDb) dB; target ±$MaxDeviationDb dB)." }
} catch { $failure = $_ } finally {
    if ($route) { try { if (-not $route.HasExited) { $route.Kill(); [void]$route.WaitForExit(5000) } } finally { $route.Dispose() } }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; Passed=(-not [bool]$failure); Error=[string]$failure; Result=$result
        Qualification=$false; Scope='Relative 20 Hz-20 kHz sweep magnitude through Cable A Input -> AudioRouter product cable route -> Cable B Output; passband only, aliasing/phase not measured, not VCAB-22 or A16 qualification' }
    $zip = Join-Path $bundle "$run.zip"; Compress-Archive -LiteralPath $evidence -DestinationPath $zip; Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
}
if ($failure) { throw $failure }
Write-Host "Sweep response diagnostic passed: $($result.WorstDeviationDb) dB maximum deviation. A16 remains incomplete."
