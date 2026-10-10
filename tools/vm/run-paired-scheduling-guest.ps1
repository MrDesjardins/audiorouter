<# Guest-only paired recorder probe or bounded tone on the already installed driver. #>
[CmdletBinding()]
param([ValidateSet('PairProbe','Tone')][string] $Phase = 'PairProbe',
    [ValidateSet(30,300)][int] $ToneSeconds = 30)
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
if (-not $offer -or $offer.Phase -cne $Phase -or $offer.Run -cnotmatch '^[a-f0-9]{32}$' -or
    ($Phase -eq 'Tone' -and $offer.ToneSeconds -ne $ToneSeconds)) { throw 'No matching active host offer. Start the host command with the same phase/duration first.' }
$run = $offer.Run
$directory = Join-Path $runs $run
$guestEvidence = Join-Path 'C:\ar\evidence' ("paired-$run-guest")
New-Item -ItemType Directory -Path $guestEvidence | Out-Null
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $bundle 'vm-scheduling-trace.ps1')
$failure = $null
$toneExit = $null
$toneEvidence = $null
try {
    if ($Phase -eq 'Tone') {
        $status = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
            -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
            -Stdout (Join-Path $guestEvidence 'status-stdout.txt') -Stderr (Join-Path $guestEvidence 'status-stderr.txt') -TimeoutSeconds 60
        $status | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $guestEvidence 'status-process.json') -Encoding UTF8
        if ($status.Code -ne 0) { throw 'Guest status failed before the paired recording. Preserve the evidence and send output.' }
        Write-Host "Keep Cable A Input loop and Cable B Output listener running for $ToneSeconds seconds."
    }
    Write-PairedTraceJson (Join-Path $directory 'guest-joined.json') @{ Run = $run }
    $null = Wait-PairedTraceMessage (Join-Path $directory 'host-started.json') $run 45 (Join-Path $directory 'host-failed.json')
    Reply-PairedTraceClockSamples $directory $run 'before'
    $null = Wait-PairedTraceMessage (Join-Path $directory 'measure-go.json') $run 20 (Join-Path $directory 'host-failed.json')
    if ($Phase -eq 'Tone') {
        $beforeTone = @(Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter '*-tone' | Select-Object -ExpandProperty FullName)
        $toneResult = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
            -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','tone','-ToneSeconds',[string]$ToneSeconds,'-TraceScheduling') `
            -Stdout (Join-Path $guestEvidence 'tone-stdout.txt') -Stderr (Join-Path $guestEvidence 'tone-stderr.txt') -TimeoutSeconds ($ToneSeconds + 210)
        $toneExit = $toneResult.Code
        $toneResult | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $guestEvidence 'tone-process.json') -Encoding UTF8
        $newTone = @(Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter '*-tone' | Where-Object { $_.FullName -notin $beforeTone })
        if ($newTone.Count -eq 1) { $toneEvidence = $newTone[0].FullName }
        else { throw 'Cannot uniquely identify new tone evidence; preserve C:\ar\evidence.' }
        Get-Content -LiteralPath (Join-Path $guestEvidence 'tone-stdout.txt') -Tail 15 | Write-Host
    } else { Invoke-DriverVmSchedulingProbe -Recorder (Join-Path $env:SystemRoot 'System32\wpr.exe') -Directory $guestEvidence }
    Write-PairedTraceJson (Join-Path $directory 'measure-done.json') @{ Run = $run; ExitCode = $(if ($Phase -eq 'Tone') { $toneExit } else { 0 }) }
    Reply-PairedTraceClockSamples $directory $run 'after'
    $null = Wait-PairedTraceMessage (Join-Path $directory 'host-saved.json') $run 180 (Join-Path $directory 'host-failed.json')
} catch {
    $failure = $_
    Write-PairedTraceJson (Join-Path $directory 'guest-failed.json') @{ Run = $run; Error = [string]$_ }
} finally {
    Write-PairedTraceJson (Join-Path $guestEvidence 'result.json') @{ Run = $run; Phase = $Phase; CoordinationPassed = -not [bool]$failure;
        ToneExitCode = $toneExit; Passed = -not [bool]$failure -and ($Phase -ne 'Tone' -or $toneExit -eq 0); Error = [string]$failure }
    $zip = Join-Path $bundle ("paired-$run-guest.zip")
    $archivePaths = @($guestEvidence)
    if ($toneEvidence) { $archivePaths += $toneEvidence }
    Compress-Archive -LiteralPath $archivePaths -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peerBundle
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Guest evidence copied to shared folder: $(Split-Path -Leaf $zip)"
}
if ($failure) { throw $failure }
if ($Phase -eq 'Tone') {
    if ($toneExit -ne 0) { throw 'Tone failed; current-run paired evidence was copied. Send both outputs.' }
    Write-Host 'Paired tone passed. Send both outputs for review before any longer run.'
} else { Write-Host 'PairProbe passed. No audio test was run. Send both outputs for review.' }
