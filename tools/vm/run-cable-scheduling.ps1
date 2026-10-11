<# Guest-only focused scheduler trace. Never installs a driver or changes settings. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $bundle 'vm-scheduling-trace.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Requires 2 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first.' }
$run = 'cable-scheduling-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$recorder = Join-Path $env:SystemRoot 'System32\wpr.exe'
$failure = $null
$child = $null
$script:cableSchedulingChild = $null
try {
    Invoke-DriverVmSchedulingRun -Recorder $recorder -Directory (Join-Path $evidence 'trace') -ProfilePath (Join-Path $bundle 'AudioRouterScheduling.wprp') -Run {
        # Redirect the bounded child: console backpressure cannot extend audio.
        $script:cableSchedulingChild = Invoke-DriverVmProcess -Executable $powershell -Arguments @(
            '-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'run-cable-latency-starts.ps1'),
            '-Route','engine','-Starts','1','-Impulses','1000','-TraceRoute','-PeriodMode','low-latency') `
            -Stdout (Join-Path $evidence 'measurement.txt') -Stderr (Join-Path $evidence 'measurement-stderr.txt') -TimeoutSeconds 120
        if ($script:cableSchedulingChild.Code -ne 0) { throw 'Focused measurement failed; retain measurement and scheduler evidence.' }
    }
} catch { $failure = $_ } finally {
    $child = $script:cableSchedulingChild
    if (Test-Path -LiteralPath (Join-Path $evidence 'measurement.txt')) {
        $text = Get-Content -LiteralPath (Join-Path $evidence 'measurement.txt') -Raw
        $match = [regex]::Match($text, 'Current-run reports copied to Z:\\[^\r\n]+\\(starts-engine-[a-f0-9]{32}\.zip)')
        if ($match.Success) {
            $inner = Join-Path $bundle $match.Groups[1].Value
            if (Test-Path -LiteralPath $inner -PathType Leaf) { Copy-Item -LiteralPath $inner -Destination $evidence }
            elseif (-not $failure) { $failure = 'Measurement archive is missing.' }
        } elseif (-not $failure) { $failure = 'Measurement archive identity is missing.' }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; Passed=(-not [bool]$failure); Error=[string]$failure; Child=$child; Qualification=$false }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Focused cable scheduling trace saved. Send the output.'
