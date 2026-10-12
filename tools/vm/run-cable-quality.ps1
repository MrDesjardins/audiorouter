<# Run implemented WP-09b cases in the disposable driver VM. These are
   diagnostic subsets of A16, never complete A16 qualification. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('float32-44k1','float32-48k','float32-96k','pcm16-44k1','pcm16-48k','pcm16-96k','pcm24-44k1','pcm24-48k','pcm24-96k','pcm32-44k1','pcm32-48k','pcm32-96k','sweep-44k1','sweep-48k','sweep-96k','thdn-997-48k','tone-continuity','eight-cable-continuity','isolation-A-to-B','isolation-all-pairs')][string] $Case,
    [ValidateRange(1, 8)][int] $Channels = 2,
    [ValidateRange(5, 60)][int] $Seconds = 20,
    [ValidateRange(1, 60)][int] $Minutes = 1,
    [switch] $ChannelMatrix
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

if ($ChannelMatrix) {
    if ($Case -notlike 'float32-*' -and $Case -notlike 'pcm*' -and $Case -notlike 'sweep-*') {
        throw '-ChannelMatrix is supported for float32, PCM and sweep cases.'
    }
    foreach ($matrixChannels in @(1, 2, 8)) {
        Write-Host "Running $Case with $matrixChannels channel(s)."
        & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
            -NoProfile -ExecutionPolicy Bypass -File $PSCommandPath -Case $Case -Channels $matrixChannels -Seconds $Seconds
        if ($LASTEXITCODE -ne 0) { throw "$Case failed for $matrixChannels channel(s) (exit $LASTEXITCODE)." }
    }
    Write-Host "$Case completed for the 1/2/8-channel matrix. A16 remains incomplete."
    return
}

if ($Case -like 'sweep-*') {
    $runner = Join-Path $bundle 'run-cable-sweep.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no product-route sweep runner.' }
    $rate = switch ($Case) { 'sweep-44k1' { 44100 } 'sweep-48k' { 48000 } 'sweep-96k' { 96000 } }
    Write-Host "WP-09b diagnostic subset: 20 Hz-20 kHz float32 sweep through the product route at $rate Hz, $Channels channel(s). A16 remains incomplete."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Channels $Channels -Rate $rate -Seconds $Seconds
    if ($LASTEXITCODE -ne 0) { throw "Sweep response diagnostic failed (exit $LASTEXITCODE)." }
    return
}

if ($Case -like 'float32-*' -or $Case -like 'pcm*') {
    $runner = Join-Path $bundle 'run-cable-bitexact.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no bit-exact product-route runner.' }
    $rate = switch -Regex ($Case) { '-44k1$' { 44100; break } '-96k$' { 96000; break } default { 48000 } }
    $pcmBits = if ($Case -match '^pcm(16|24|32)-') { [int]$Matches[1] } else { 0 }
    $format = if ($pcmBits -eq 0) { 'float32' } else { "PCM$pcmBits" }
    Write-Host "WP-09b diagnostic subset: $Case, $Channels channel(s), $format. Set both Cable endpoints to $rate Hz and the requested format first. This does not pass A16."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Channels $Channels -Rate $rate -PcmBits $pcmBits -Seconds $Seconds
    if ($LASTEXITCODE -ne 0) { throw "$format product-route diagnostic failed (exit $LASTEXITCODE)." }
    Write-Host 'Diagnostic case completed. A16 remains incomplete until every required rate, format, channel count, resampling and crosstalk case is implemented and run.'
    return
}
if ($Case -eq 'thdn-997-48k') {
    $runner = Join-Path $bundle 'run-cable-thdn.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no product-route THD+N runner.' }
    Write-Host "WP-09b diagnostic subset: 997 Hz float32 tone through the product route, $Channels channel(s). Set both Cable endpoints to 48 kHz first. This does not pass A16."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Channels $Channels -Seconds $Seconds
    if ($LASTEXITCODE -ne 0) { throw "THD+N diagnostic failed (exit $LASTEXITCODE)." }
    Write-Host 'Diagnostic case completed. A16 remains incomplete.'
    return
}
if ($Case -eq 'tone-continuity') {
    $runner = Join-Path $bundle 'run-cable-tone.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no product-route tone continuity runner.' }
    Write-Host "WP-09b diagnostic subset: 997 Hz and 47 Hz product-route continuity, $Minutes minute(s) each. This does not pass A16."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Minutes $Minutes
    if ($LASTEXITCODE -ne 0) { throw "Tone continuity diagnostic failed (exit $LASTEXITCODE)." }
    return
}
if ($Case -eq 'eight-cable-continuity') {
    $runner = Join-Path $bundle 'run-cable-continuity.ps1'
    if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no eight-cable continuity runner.' }
    Write-Host "WP-09b diagnostic subset: eight cables and four product routes for $Minutes minute(s). The runner restores the original cable count; this does not pass A16."
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $runner -Minutes $Minutes
    if ($LASTEXITCODE -ne 0) { throw "Eight-cable continuity diagnostic failed (exit $LASTEXITCODE)." }
    return
}
if ($Case -eq 'isolation-all-pairs') {
    $matrix = Join-Path $bundle 'run-cable-isolation-matrix.ps1'
    if (-not (Test-Path -LiteralPath $matrix -PathType Leaf)) { throw 'The bundle has no ordered isolation matrix runner.' }
    & (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -NoProfile -ExecutionPolicy Bypass -File $matrix -Seconds $Seconds
    if ($LASTEXITCODE -ne 0) { throw "Cable isolation matrix failed (exit $LASTEXITCODE)." }
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
