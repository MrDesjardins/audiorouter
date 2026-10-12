<# Run implemented WP-09b cases in the disposable driver VM. These are
   diagnostic subsets of A16, never complete A16 qualification. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('float32-44k1','float32-48k','float32-96k','isolation-A-to-B')][string] $Case,
    [ValidateRange(1, 8)][int] $Channels = 2,
    [ValidateRange(5, 60)][int] $Seconds = 20
)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') {
    throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.'
}
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle

if ($Case -like 'float32-*') {
    $runner = Join-Path $bundle 'run-cable-bitexact.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no bit-exact product-route runner.' }
    $rate = switch ($Case) { 'float32-44k1' { 44100 } 'float32-48k' { 48000 } 'float32-96k' { 96000 } }
    Write-Host "WP-09b diagnostic subset: $Case, $Channels channel(s), float32. Set both Cable endpoints to $rate Hz first. This does not pass A16."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Channels $Channels -Rate $rate -Seconds $Seconds
    if ($LASTEXITCODE -ne 0) { throw "Float32 bit-exact diagnostic failed (exit $LASTEXITCODE)." }
    Write-Host 'Diagnostic case completed. A16 remains incomplete until every required rate, format, channel count, resampling and crosstalk case is implemented and run.'
    return
}

if (Get-Process -Name 'm00-probe','m03_bridge_tone','m03_cable_session' -ErrorAction SilentlyContinue) {
    throw 'Another cable diagnostic is running.'
}
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the bundle to C:\ar from Z: before collecting evidence.' }
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$quality = Join-Path $bundle 'tools\m03-cable-quality.exe'
if (-not (Test-Path -LiteralPath $probe -PathType Leaf) -or -not (Test-Path -LiteralPath $quality -PathType Leaf)) {
    throw 'The bundle is missing the native probe or Rust analyzer.'
}
$run = 'quality-isolation-ab-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$failure = $null
$result = $null
try {
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') `
        -Stdout (Join-Path $evidence 'inventory.txt') -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $text = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $render = [regex]::Matches($text, '(?m)^render\[(\d+)\] name=AudioRouter Cable A Input \(AudioRouter Virtual Cable\) id=')
    $capture = [regex]::Matches($text, '(?m)^capture\[(\d+)\] name=AudioRouter Cable B Output \(AudioRouter Virtual Cable\) id=')
    if ($inventory.Code -ne 0 -or $render.Count -ne 1 -or $capture.Count -ne 1) { throw 'Expected exactly one Cable A Input and Cable B Output; no audio test started.' }
    $probeRun = Invoke-DriverVmProcess -Executable $probe `
        -Arguments @('cable-isolation',[string]$Seconds,$render[0].Groups[1].Value,$capture[0].Groups[1].Value,'4294967295',$evidence) `
        -Stdout (Join-Path $evidence 'probe.txt') -Stderr (Join-Path $evidence 'probe-stderr.txt') -TimeoutSeconds ($Seconds + 60)
    if ($probeRun.Code -ne 0) { throw "Native cable isolation probe failed (exit $($probeRun.Code)); see probe output." }
    $captureWav = Join-Path $evidence 'cable-isolation-capture.wav'
    if (-not (Test-Path -LiteralPath $captureWav -PathType Leaf)) { throw 'The probe did not save the captured cable samples.' }
    $metricsPath = Join-Path $evidence 'metrics.json'
    $analysis = Invoke-DriverVmProcess -Executable $quality -Arguments @('silence',$captureWav,$metricsPath,'-140') `
        -Stdout (Join-Path $evidence 'analysis.txt') -Stderr (Join-Path $evidence 'analysis-stderr.txt') -TimeoutSeconds 60
    if ($analysis.Code -ne 0 -or -not (Test-Path -LiteralPath $metricsPath -PathType Leaf)) { throw 'Rust isolation analyzer failed.' }
    $metrics = Get-Content -LiteralPath $metricsPath -Raw | ConvertFrom-Json
    if (-not $metrics.pass) { throw "VCAB-26 diagnostic exceeded -140 dBFS (observed $($metrics.peakDbfs) dBFS)." }
    $result = [pscustomobject]@{ Case = $Case; Seconds = $Seconds; Requirement = $metrics.requirement
        PeakDbfs = $metrics.peakDbfs; Pass = $metrics.pass; Metrics = 'metrics.json'; Qualification = $false }
} catch { $failure = $_ } finally {
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure)
        Error = [string]$failure; Result = $result; Qualification = $false
        Scope = 'Cable A Input to Cable B Output isolation diagnostic; single pair, float32 shared mix format; not complete VCAB-26 or A16 qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
}
if ($failure) { throw $failure }
Write-Host "Isolation diagnostic passed at $($result.PeakDbfs) dBFS. A16 remains incomplete."
