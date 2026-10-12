<# Measure 997 Hz THD+N through the product cable route. Diagnostic subset of VCAB-22/23, never A16 qualification. #>
[CmdletBinding()]
param([ValidateRange(1,8)][int] $Channels = 2, [ValidateRange(5,60)][int] $Seconds = 20,
    [ValidateRange(1,120)][int] $FrequencyHz = 997, [double] $MaxThdnDb = -60)
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
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$quality = Join-Path $bundle 'tools\m03-cable-quality.exe'
$cableSession = Join-Path $bundle 'tools\m03_cable_session.exe'
foreach ($file in @($probe,$quality,$cableSession)) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Bundle tool missing: $file" } }
$run = "thdn-$FrequencyHz-hz-$Channels-ch-" + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$route = $null; $failure = $null; $result = $null
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
    $probeRun = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-tone',[string]$Seconds,$render[0].Groups[1].Value,$capture[0].Groups[1].Value,$evidence,[string]$FrequencyHz) `
        -Stdout (Join-Path $evidence 'probe.txt') -Stderr (Join-Path $evidence 'probe-stderr.txt') -TimeoutSeconds ($Seconds + 60)
    if (-not $route.WaitForExit(($Seconds + 60) * 1000)) { throw 'Product route did not finish within its watchdog.' }
    $reference = Join-Path $evidence 'cable-tone-reference.wav'; $captureWav = Join-Path $evidence 'cable-tone-capture.wav'
    if ($probeRun.Code -ne 0 -or -not (Test-Path -LiteralPath $reference) -or -not (Test-Path -LiteralPath $captureWav)) { throw 'Native probe failed or did not produce both WAV captures.' }
    $metricsPath = Join-Path $evidence 'metrics.json'
    $analysis = Invoke-DriverVmProcess -Executable $quality -Arguments @('tone',$reference,$captureWav,$metricsPath,[string]$FrequencyHz,[string]$MaxThdnDb) `
        -Stdout (Join-Path $evidence 'analysis.txt') -Stderr (Join-Path $evidence 'analysis-stderr.txt') -TimeoutSeconds 60
    if ($analysis.Code -ne 0 -or -not (Test-Path -LiteralPath $metricsPath)) { throw 'Rust THD+N analysis failed.' }
    $metrics = Get-Content -LiteralPath $metricsPath -Raw | ConvertFrom-Json
    $result = [pscustomobject]@{ Channels=$Channels; FrequencyHz=$FrequencyHz; MaxThdnDb=$MaxThdnDb; WorstThdnDb=$metrics.worstThdnDb
        AnalyzerPass=$metrics.pass; AlignmentOffsetFrames=$metrics.alignmentOffsetFrames; Frames=$metrics.frames; Metrics='metrics.json'
        RouteExit=$route.ExitCode; ProbeExit=$probeRun.Code; Qualification=$false }
    if (-not $metrics.pass -or $route.ExitCode -ne 0) { throw "THD+N diagnostic failed ($($metrics.worstThdnDb) dB; target $MaxThdnDb dB)." }
} catch { $failure = $_ } finally {
    if ($route) { try { if (-not $route.HasExited) { $route.Kill(); [void]$route.WaitForExit(5000) } } finally { $route.Dispose() } }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; Passed=(-not [bool]$failure); Error=[string]$failure; Result=$result
        Qualification=$false; Scope='997 Hz float32 THD+N of Cable A Input -> AudioRouter product cable route -> Cable B Output; diagnostic subset, not VCAB-22/23 or A16 qualification' }
    $zip = Join-Path $bundle "$run.zip"; Compress-Archive -LiteralPath $evidence -DestinationPath $zip; Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
}
if ($failure) { throw $failure }
Write-Host "THD+N diagnostic passed: $($result.WorstThdnDb) dB at $FrequencyHz Hz. A16 remains incomplete."
