<# Guest-only two-second paired recorder/clock probe. No audio/driver calls. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run the guest script from the copied bundle under C:\ar inside AR-DriverTest.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Guest probe requires 2 GB free on C:.' }
$peerBundle = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
$runs = Join-Path $peerBundle 'paired-runs'
$offer = Read-PairedTraceJson (Join-Path $runs 'offer.json')
if (-not $offer -or $offer.Phase -cne 'PairProbe' -or $offer.Run -cnotmatch '^[a-f0-9]{32}$') { throw 'No active host PairProbe offer. Start the host command first.' }
$run = $offer.Run
$directory = Join-Path $runs $run
$guestEvidence = Join-Path 'C:\ar\evidence' ("paired-$run-guest")
New-Item -ItemType Directory -Path $guestEvidence | Out-Null
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $bundle 'vm-scheduling-trace.ps1')
$failure = $null
try {
    Write-PairedTraceJson (Join-Path $directory 'guest-joined.json') @{ Run = $run }
    $null = Wait-PairedTraceMessage (Join-Path $directory 'host-started.json') $run 45 (Join-Path $directory 'host-failed.json')
    Reply-PairedTraceClockSamples $directory $run 'before'
    $null = Wait-PairedTraceMessage (Join-Path $directory 'measure-go.json') $run 20 (Join-Path $directory 'host-failed.json')
    Invoke-DriverVmSchedulingProbe -Recorder (Join-Path $env:SystemRoot 'System32\wpr.exe') -Directory $guestEvidence
    Write-PairedTraceJson (Join-Path $directory 'measure-done.json') @{ Run = $run }
    Reply-PairedTraceClockSamples $directory $run 'after'
    $null = Wait-PairedTraceMessage (Join-Path $directory 'host-saved.json') $run 180 (Join-Path $directory 'host-failed.json')
} catch {
    $failure = $_
    Write-PairedTraceJson (Join-Path $directory 'guest-failed.json') @{ Run = $run; Error = [string]$_ }
} finally {
    Write-PairedTraceJson (Join-Path $guestEvidence 'result.json') @{ Run = $run; Passed = -not [bool]$failure; Error = [string]$failure }
    $zip = Join-Path $bundle ("paired-$run-guest.zip")
    Compress-Archive -LiteralPath $guestEvidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peerBundle
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Guest evidence copied to shared folder: $(Split-Path -Leaf $zip)"
}
if ($failure) { throw $failure }
Write-Host 'PairProbe passed. No audio test was run. Send both outputs for review.'
