<# Measure ordered Cable Input -> different Cable Output isolation pairs.
   Expands to eight cables only for this guarded VM session and restores the
   original cable count in finally. This is not full A16 qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 60)][int] $Seconds = 5)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') {
    throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.'
}
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if (Get-Process -Name 'm00-probe','m03_bridge_tone','m03_cable_session' -ErrorAction SilentlyContinue) {
    throw 'Another cable diagnostic is running.'
}
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the bundle to C:\ar from Z: before collecting evidence.' }
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$quality = Join-Path $bundle 'tools\m03-cable-quality.exe'
$helper = Join-Path $bundle 'tools\audiorouter-driver-helper.exe'
foreach ($path in @($probe,$quality,$helper)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "The bundle is missing $([IO.Path]::GetFileName($path))." }
}
$run = 'quality-isolation-matrix-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$letters = @('A','B','C','D','E','F','G','H')
$results = @()
$failure = $null
$originalCount = $null
$countChanged = $false
$restoreError = $null

function Invoke-Helper([int] $Count, [string] $Name) {
    $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
    return Invoke-DriverVmProcess -Executable $helper `
        -Arguments @('set-cables','--count',[string]$Count,'--result',(Join-Path $evidence "$Name.json")) `
        -Stdout (Join-Path $evidence "$Name.txt") -Stderr (Join-Path $evidence "$Name-stderr.txt") -TimeoutSeconds 240
}
function Get-Inventory([string] $Name) {
    $result = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') `
        -Stdout (Join-Path $evidence "$Name.txt") -Stderr (Join-Path $evidence "$Name-stderr.txt") -TimeoutSeconds 20
    if ($result.Code -ne 0) { throw "Endpoint inventory failed ($($result.Code))." }
    return Get-Content -LiteralPath (Join-Path $evidence "$Name.txt") -Raw
}
function Get-Index([string] $Text, [string] $Direction, [string] $Letter) {
    $name = if ($Direction -eq 'render') { "AudioRouter Cable $Letter Input (AudioRouter Virtual Cable)" } else { "AudioRouter Cable $Letter Output (AudioRouter Virtual Cable)" }
    $matches = [regex]::Matches($Text, "(?m)^$Direction\[(\d+)\] name=" + [regex]::Escape($name) + ' id=')
    if ($matches.Count -ne 1) { return $null }
    return $matches[0].Groups[1].Value
}
function Get-CableCount([string] $Text) {
    $count = 0
    foreach ($letter in $letters) {
        if ($null -ne (Get-Index $Text 'render' $letter) -and $null -ne (Get-Index $Text 'capture' $letter)) { $count++ }
    }
    return $count
}

try {
    $before = Get-Inventory 'inventory-before'
    $originalCount = Get-CableCount $before
    if ($originalCount -lt 2 -or $originalCount -gt 8) { throw "Expected between two and eight complete cables, found $originalCount." }
    if ($originalCount -ne 8) {
        $set = Invoke-Helper 8 'set-cables-8'
        if ($set.Code -ne 0) { throw "Could not enable eight cables (exit $($set.Code))." }
        $countChanged = $true
        Start-Sleep -Seconds 3
    }
    $inventory = Get-Inventory 'inventory-eight'
    if ((Get-CableCount $inventory) -ne 8) { throw 'Eight complete cables were not enumerated after set-cables.' }
    foreach ($source in $letters) {
        $renderIndex = Get-Index $inventory 'render' $source
        foreach ($sink in $letters) {
            if ($source -eq $sink) { continue }
            $captureIndex = Get-Index $inventory 'capture' $sink
            if ($null -eq $renderIndex -or $null -eq $captureIndex) { throw "Missing endpoint for Cable $source to Cable $sink." }
            $case = "cable-$source-to-$sink"
            $caseDir = Join-Path $evidence $case
            New-Item -ItemType Directory -Path $caseDir | Out-Null
            $probeRun = Invoke-DriverVmProcess -Executable $probe `
                -Arguments @('cable-isolation',[string]$Seconds,$renderIndex,$captureIndex,'4294967295',$caseDir) `
                -Stdout (Join-Path $caseDir 'probe.txt') -Stderr (Join-Path $caseDir 'probe-stderr.txt') -TimeoutSeconds ($Seconds + 60)
            if ($probeRun.Code -ne 0) { throw "$case native probe failed (exit $($probeRun.Code))." }
            $captureWav = Join-Path $caseDir 'cable-isolation-capture.wav'
            $metricsPath = Join-Path $caseDir 'metrics.json'
            $analysis = Invoke-DriverVmProcess -Executable $quality -Arguments @('silence',$captureWav,$metricsPath,'-140') `
                -Stdout (Join-Path $caseDir 'analysis.txt') -Stderr (Join-Path $caseDir 'analysis-stderr.txt') -TimeoutSeconds 60
            if ($analysis.Code -ne 0 -or -not (Test-Path -LiteralPath $metricsPath -PathType Leaf)) { throw "$case analyzer failed." }
            $metrics = Get-Content -LiteralPath $metricsPath -Raw | ConvertFrom-Json
            $entry = [pscustomobject]@{ Source = $source; Sink = $sink; PeakDbfs = $metrics.peakDbfs
                Pass = [bool]$metrics.pass; Metrics = "$case\metrics.json" }
            $results += $entry
            Write-Host ("{0}: {1} at {2:N2} dBFS" -f $case, $(if ($entry.Pass) { 'PASS' } else { 'FAIL' }), $entry.PeakDbfs)
            if (-not $entry.Pass) { throw "$case exceeded the -140 dBFS threshold." }
        }
    }
} catch { $failure = $_ } finally {
    if ($countChanged -and $null -ne $originalCount) {
        try {
            $restore = Invoke-Helper $originalCount 'restore-cable-count'
            if ($restore.Code -ne 0) { throw "Restoring $originalCount cables failed (exit $($restore.Code))." }
            Start-Sleep -Seconds 3
        } catch { $restoreError = $_ }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure -and -not [bool]$restoreError)
        Error = [string]$failure; RestoreError = [string]$restoreError; OriginalCableCount = $originalCount
        Results = $results; Qualification = $false
        Scope = '56 ordered float32 cable isolation pairs at the available shared mix rate; diagnostic subset, not complete A16/VCAB-26 qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
}
if ($restoreError) { throw $restoreError }
if ($failure) { throw $failure }
Write-Host "Completed $($results.Count) ordered cable pairs. Original cable count $originalCount restored. A16 remains incomplete."
