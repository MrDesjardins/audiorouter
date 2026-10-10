<# Guest speaker capture only; never starts a cable/driver test. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'

function Assert-SpeakerReady($Ready) {
    if ($Ready.mode -cne 'guest-speaker-loopback' -or
        $Ready.endpointName -cne 'Speakers (High Definition Audio Device)' -or
        [string]::IsNullOrWhiteSpace([string]$Ready.endpointId) -or
        $Ready.rate -ne 44100 -or $Ready.channels -ne 2 -or $Ready.seconds -ne 30 -or
        $Ready.qualification -ne $false -or
        -not (($Ready.bits -eq 16 -and $Ready.wavTag -eq 1) -or ($Ready.bits -eq 32 -and $Ready.wavTag -eq 3))) {
        throw 'Speaker recorder readiness contract mismatch; no reference was opened.'
    }
}

function Invoke-SpeakerRecording([string] $Probe, [string] $Evidence, [string] $Reference) {
    $capture = Join-Path $Evidence 'capture'
    New-Item -ItemType Directory -Path $capture | Out-Null
    $process = $null
    try {
        $arguments = @('speaker-record',$capture) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
        $process = Start-Process -FilePath $Probe -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $Evidence 'recorder.txt') -RedirectStandardError (Join-Path $Evidence 'recorder-stderr.txt')
        $null = $process.Handle
        $watch = [Diagnostics.Stopwatch]::StartNew()
        $readyFile = Join-Path $capture 'ready.json'
        while (-not (Test-Path -LiteralPath $readyFile)) {
            if ($process.HasExited) { throw 'Speaker recorder exited before readiness; see recorder-process.json and stderr.' }
            if ($watch.Elapsed.TotalSeconds -ge 12) { throw 'Speaker recorder readiness timed out; no reference was opened.' }
            Start-Sleep -Milliseconds 50
        }
        Assert-SpeakerReady (Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json)
        if ($process.HasExited) { throw 'Speaker recorder ended before playback; no reference was opened.' }
        Write-Host 'Recording guest Speakers for 30 seconds. The 17-second reference opens now; Listen must stay off.'
        Invoke-Item -LiteralPath $Reference
        Write-PairedTraceJson (Join-Path $Evidence 'playback-request.json') @{ Reference=$Reference; Utc=[DateTime]::UtcNow.ToString('o'); Started=$true }
        if (-not $process.WaitForExit(45000)) { throw 'Speaker recorder process watchdog expired; preserve this run.' }
        if ($process.ExitCode -ne 0) { throw 'Speaker recording failed; see recording.json and recorder-stderr.txt.' }
        if (-not (Test-Path -LiteralPath (Join-Path $capture 'speakers.wav') -PathType Leaf)) { throw 'Speaker recorder produced no WAV.' }
    } finally {
        if ($process) {
            try {
                if (-not $process.HasExited) {
                    $process.Kill()
                    [void]$process.WaitForExit(5000)
                }
                if ($process.HasExited) {
                    $hex = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$process.ExitCode),0).ToString('X8')
                    Write-PairedTraceJson (Join-Path $Evidence 'recorder-process.json') @{ ExitCode=$process.ExitCode; ExitCodeHex=('0x'+$hex); ProcessId=$process.Id; Qualification=$false }
                }
            } finally { $process.Dispose() }
        }
    }
}

$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the new bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'm03-driver-vm-support.ps1')
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 64MB) { throw 'Requires 64 MB free on guest C: for current-run evidence.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Stop here.' }
$reference = Join-Path $bundle 'reference-44100.wav'
if ((Get-FileHash -LiteralPath $reference -Algorithm SHA256).Hash -cne 'C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7') { throw 'Independent reference checksum mismatch.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Shared evidence destination is missing.' }
$run = 'speaker-loopback-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$probe = Join-Path $bundle 'm03_direct_audio.exe'
$failure = $null
try {
    $startup = Invoke-DriverVmProcess -Executable $probe -Arguments @('startup-check') -Stdout (Join-Path $evidence 'startup.txt') `
        -Stderr (Join-Path $evidence 'startup-stderr.txt') -TimeoutSeconds 5
    Write-PairedTraceJson (Join-Path $evidence 'startup-process.json') $startup
    if ($startup.Code -ne 0) { throw 'Speaker helper startup failed; no audio was started.' }
    Invoke-SpeakerRecording $probe $evidence $reference
} catch { $failure = [string]$_ }
finally {
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; CaptureCompleted=(-not [bool]$failure); Error=[string]$failure; Qualification=$false }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current speaker recording copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Capture saved for waveform review. This is not an audio-quality or driver pass. Send the output and describe any crackles.'
