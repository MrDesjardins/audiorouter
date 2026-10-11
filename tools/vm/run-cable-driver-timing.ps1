<# Guest only: one disabled baseline and one enabled diagnostic observation. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*' -or
    -not (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)) { throw 'Run only inside AR-DriverTest from C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Requires 2 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio','m00-probe','m03-timing-trace' -ErrorAction SilentlyContinue) { throw 'Another audio diagnostic is running.' }
$package = Get-Content -LiteralPath (Join-Path $bundle 'driver\package.json') -Raw | ConvertFrom-Json
if (-not $package.timingDiagnostics -or $package.signed -ne 'test') { throw 'Requires the verified diagnostic test package.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first.' }
$run = 'driver-timing-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$failure = $null
$children = @()
try {
    $powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $children += Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',
        (Join-Path $bundle 'run-cable-latency-starts.ps1'),'-Route','engine','-Starts','1','-Impulses','1000','-TraceRoute','-PeriodMode','low-latency') `
        -Stdout (Join-Path $evidence 'baseline.txt') -Stderr (Join-Path $evidence 'baseline-stderr.txt') -TimeoutSeconds 120
    if ($children[-1].Code -ne 0) { throw 'Disabled baseline failed; preserve evidence.' }
    $enabled = Join-Path $evidence 'enabled'
    New-Item -ItemType Directory -Path $enabled | Out-Null
    $children += Invoke-DriverVmProcess -Executable (Join-Path $bundle 'tools\m03-timing-trace.exe') -Arguments @($bundle,$enabled) `
        -Stdout (Join-Path $evidence 'timing-summary.txt') -Stderr (Join-Path $evidence 'timing-stderr.txt') -TimeoutSeconds 150
    if ($children[-1].Code -ne 0) { throw 'Enabled diagnostic failed or incomplete; preserve evidence.' }
} catch { $failure = $_ } finally {
    foreach ($path in @((Join-Path $evidence 'baseline.txt'),(Join-Path $evidence 'enabled\measurement.txt'))) {
        if (Test-Path -LiteralPath $path) {
            $text = Get-Content -LiteralPath $path -Raw
            $match = [regex]::Match($text, 'Current-run reports copied to Z:\\[^\r\n]+\\(starts-engine-[a-f0-9]{32}\.zip)')
            if ($match.Success) {
                $archive = Join-Path $bundle $match.Groups[1].Value
                if (Test-Path -LiteralPath $archive -PathType Leaf) { Copy-Item -LiteralPath $archive -Destination $evidence }
                elseif (-not $failure) { $failure = 'Current measurement archive missing.' }
            } elseif (-not $failure) { $failure = 'Current measurement identity missing.' }
        }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; Passed=(-not [bool]$failure); Error=[string]$failure; Children=$children; Qualification=$false }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Disabled/enabled driver timing comparison saved. Send the output.'
