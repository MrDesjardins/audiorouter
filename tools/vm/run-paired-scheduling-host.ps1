<# Host scheduling recorder only. Audio and driver calls remain guest-only. #>
[CmdletBinding()]
param([ValidateSet('HostProbe','PairProbe','Tone')][string] $Phase = 'HostProbe',
    [ValidateSet(30,300)][int] $ToneSeconds = 30)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ieq 'AR-DriverTest' -or (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM')) {
    throw 'Run the host script on the main PC, outside the VM.'
}
. (Join-Path $bundle 'paired-trace-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
$requiredFree = if ($Phase -eq 'Tone') { 16GB } else { 4GB }
if (([IO.DriveInfo]::new([IO.Path]::GetPathRoot($bundle))).AvailableFreeSpace -lt $requiredFree) { throw "Host recording requires $($requiredFree / 1GB) GB free." }
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $bundle 'vm-scheduling-trace.ps1')
$recorder = Join-Path $env:SystemRoot 'System32\wpr.exe'
$pairId = [Guid]::NewGuid().ToString('N')
$runs = Join-Path $bundle 'paired-runs'
$pairDirectory = Join-Path $runs $pairId
$hostEvidence = Join-Path $pairDirectory 'host'
New-Item -ItemType Directory -Path $hostEvidence -Force | Out-Null
$failure = $null
$pairOutcome = [pscustomobject]@{ GuestExitCode = $null }
$measureTimeout = if ($Phase -eq 'Tone') { $ToneSeconds + 240 } else { 45 }
$maxHostTraceBytes = if ($Phase -eq 'Tone') { 6GB } else { 1GB }
$minHostTraceFree = if ($Phase -eq 'Tone') { 8GB } else { 2GB }
try {
    if ($Phase -eq 'HostProbe') {
        Write-Host 'Checking host recorder for two seconds. No audio test runs.'
        Invoke-DriverVmSchedulingProbe -Recorder $recorder -Directory $hostEvidence
    } else {
        # The selected ID is advisory only; run-specific messages prevent reuse.
        # Refuse a concurrent/stale offer rather than overwriting it.
        $offer = Join-Path $runs 'offer.json'
        Write-PairedTraceJson $offer @{ Run = $pairId; Phase = $Phase; ToneSeconds = $ToneSeconds; CreatedUtc = [DateTime]::UtcNow.ToString('o') }
        Write-Host "Ready for the guest $Phase. Run the matching guest command now; waiting at most two minutes."
        $null = Wait-PairedTraceMessage (Join-Path $pairDirectory 'guest-joined.json') $pairId 120 (Join-Path $pairDirectory 'guest-failed.json')
        Get-Process -Name VirtualBoxVM,VBoxHeadless -ErrorAction SilentlyContinue |
            Select-Object Id,ProcessName,StartTime | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $hostEvidence 'virtualbox-processes.json') -Encoding UTF8
        Invoke-DriverVmSchedulingRun -Recorder $recorder -Directory $hostEvidence -Run {
            $guard = { Assert-PairedTraceBudget $hostEvidence $maxHostTraceBytes $minHostTraceFree }
            Write-PairedTraceJson (Join-Path $pairDirectory 'host-started.json') @{ Run = $pairId }
            Save-PairedTraceClockSamples $pairDirectory $pairId 'before' $hostEvidence $guard
            Write-PairedTraceJson (Join-Path $pairDirectory 'measure-go.json') @{ Run = $pairId }
            $measurement = Wait-PairedTraceMessage (Join-Path $pairDirectory 'measure-done.json') $pairId $measureTimeout (Join-Path $pairDirectory 'guest-failed.json') 10 $guard
            $pairOutcome.GuestExitCode = $measurement.ExitCode
            Save-PairedTraceClockSamples $pairDirectory $pairId 'after' $hostEvidence $guard
        }
        Write-PairedTraceJson (Join-Path $pairDirectory 'host-saved.json') @{ Run = $pairId }
    }
} catch {
    $failure = $_
    Write-PairedTraceJson (Join-Path $pairDirectory 'host-failed.json') @{ Run = $pairId; Error = [string]$_ }
} finally {
    # Delete only our own offer, even after timeout; retain all run evidence.
    if ($Phase -ne 'HostProbe') {
        $ownOffer = Read-PairedTraceJson (Join-Path $runs 'offer.json')
        if ($ownOffer -and $ownOffer.Run -ceq $pairId) { Remove-Item -LiteralPath (Join-Path $runs 'offer.json') }
    }
    Write-PairedTraceJson (Join-Path $hostEvidence 'result.json') @{ Run = $pairId; Phase = $Phase; CoordinationPassed = -not [bool]$failure;
        GuestExitCode = $pairOutcome.GuestExitCode; Passed = -not [bool]$failure -and ($Phase -ne 'Tone' -or $pairOutcome.GuestExitCode -eq 0); Error = [string]$failure }
    $zip = Join-Path $bundle ("paired-$pairId-host.zip")
    if ($Phase -eq 'Tone') {
        # Host ETL may exceed the archive cmdlet's per-file limit. Keep it
        # private in this folder; zip only small metadata and clock reports.
        $metadata = @(Get-ChildItem -LiteralPath $hostEvidence -File | Where-Object Extension -ne '.etl')
        Compress-Archive -LiteralPath $metadata.FullName -DestinationPath $zip
        if (Test-Path -LiteralPath (Join-Path $hostEvidence 'scheduling.etl')) {
            Get-FileHash -LiteralPath (Join-Path $hostEvidence 'scheduling.etl') -Algorithm SHA256
            Write-Host "Host trace retained separately: $(Join-Path $hostEvidence 'scheduling.etl')"
        }
    } else { Compress-Archive -LiteralPath $hostEvidence -DestinationPath $zip }
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Host evidence: $zip"
}
if ($failure) { throw $failure }
if ($Phase -eq 'Tone') {
    if ($pairOutcome.GuestExitCode -ne 0) { throw 'Guest tone acceptance failed; paired diagnostic evidence was preserved. Send both outputs.' }
    Write-Host 'Paired tone passed. Send both outputs for review before any longer run.'
} else { Write-Host "$Phase passed. No audio test was run." }
