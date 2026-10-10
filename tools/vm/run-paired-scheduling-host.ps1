<# Host recorder compatibility and paired two-second probe only. No audio/driver calls. #>
[CmdletBinding()]
param([ValidateSet('HostProbe','PairProbe')][string] $Phase = 'HostProbe')
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ieq 'AR-DriverTest' -or (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM')) {
    throw 'Run the host script on the main PC, outside the VM.'
}
. (Join-Path $bundle 'paired-trace-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if (([IO.DriveInfo]::new([IO.Path]::GetPathRoot($bundle))).AvailableFreeSpace -lt 4GB) { throw 'Host probe requires 4 GB free.' }
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $bundle 'vm-scheduling-trace.ps1')
$recorder = Join-Path $env:SystemRoot 'System32\wpr.exe'
$pairId = [Guid]::NewGuid().ToString('N')
$runs = Join-Path $bundle 'paired-runs'
$pairDirectory = Join-Path $runs $pairId
$hostEvidence = Join-Path $pairDirectory 'host'
New-Item -ItemType Directory -Path $hostEvidence -Force | Out-Null
$failure = $null
try {
    if ($Phase -eq 'HostProbe') {
        Write-Host 'Checking host recorder for two seconds. No audio test runs.'
        Invoke-DriverVmSchedulingProbe -Recorder $recorder -Directory $hostEvidence
    } else {
        # The selected ID is advisory only; run-specific messages prevent reuse.
        # Refuse a concurrent/stale offer rather than overwriting it.
        $offer = Join-Path $runs 'offer.json'
        Write-PairedTraceJson $offer @{ Run = $pairId; Phase = 'PairProbe'; CreatedUtc = [DateTime]::UtcNow.ToString('o') }
        Write-Host 'Ready for the guest probe. Run the guest command now; waiting at most two minutes.'
        $null = Wait-PairedTraceMessage (Join-Path $pairDirectory 'guest-joined.json') $pairId 120 (Join-Path $pairDirectory 'guest-failed.json')
        Get-Process -Name VirtualBoxVM,VBoxHeadless -ErrorAction SilentlyContinue |
            Select-Object Id,ProcessName,StartTime | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $hostEvidence 'virtualbox-processes.json') -Encoding UTF8
        Invoke-DriverVmSchedulingRun -Recorder $recorder -Directory $hostEvidence -Run {
            $guard = { Assert-PairedTraceBudget $hostEvidence }
            Write-PairedTraceJson (Join-Path $pairDirectory 'host-started.json') @{ Run = $pairId }
            Save-PairedTraceClockSamples $pairDirectory $pairId 'before' $hostEvidence $guard
            Write-PairedTraceJson (Join-Path $pairDirectory 'measure-go.json') @{ Run = $pairId }
            $null = Wait-PairedTraceMessage (Join-Path $pairDirectory 'measure-done.json') $pairId 45 (Join-Path $pairDirectory 'guest-failed.json') 10 $guard
            Save-PairedTraceClockSamples $pairDirectory $pairId 'after' $hostEvidence $guard
        }
        Write-PairedTraceJson (Join-Path $pairDirectory 'host-saved.json') @{ Run = $pairId }
    }
} catch {
    $failure = $_
    Write-PairedTraceJson (Join-Path $pairDirectory 'host-failed.json') @{ Run = $pairId; Error = [string]$_ }
} finally {
    # Delete only our own offer, even after timeout; retain all run evidence.
    if ($Phase -eq 'PairProbe') {
        $ownOffer = Read-PairedTraceJson (Join-Path $runs 'offer.json')
        if ($ownOffer -and $ownOffer.Run -ceq $pairId) { Remove-Item -LiteralPath (Join-Path $runs 'offer.json') }
    }
    Write-PairedTraceJson (Join-Path $hostEvidence 'result.json') @{ Run = $pairId; Phase = $Phase; Passed = -not [bool]$failure; Error = [string]$failure }
    $zip = Join-Path $bundle ("paired-$pairId-host.zip")
    Compress-Archive -LiteralPath $hostEvidence -DestinationPath $zip
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Host evidence: $zip"
}
if ($failure) { throw $failure }
Write-Host "$Phase passed. No audio test was run."
