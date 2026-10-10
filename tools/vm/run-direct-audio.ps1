<# One bounded, automatic guest diagnostic on the already installed driver. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Requires 2 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$run = 'direct-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
$capture = Join-Path $evidence 'capture'
New-Item -ItemType Directory -Path $capture | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$probe = Join-Path $bundle 'tools\m03_direct_audio.exe'
$process = $null
$toneEvidence = $null
$failure = $null
$codes = @{}
try {
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    Write-Host 'Automatic 30-second test: Cable A generated source and Cable B direct recording. No manual playback needed.'
    $arguments = @('record',$capture) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
    $process = Start-Process -FilePath $probe -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $evidence 'probe.txt') -RedirectStandardError (Join-Path $evidence 'probe-stderr.txt')
    $null = $process.Handle
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not (Test-Path -LiteralPath (Join-Path $capture 'ready.json'))) {
        if ($process.HasExited) { throw 'Direct recorder failed to start; see probe-stderr.txt.' }
        if ($watch.Elapsed.TotalSeconds -ge 12) { throw 'Direct recorder readiness timed out.' }
        Start-Sleep -Milliseconds 50
    }
    $ready = Read-PairedTraceJson (Join-Path $capture 'ready.json')
    if ($ready.rate -ne 48000 -or $ready.channels -ne 2 -or $ready.bits -ne 32 -or $ready.format -cne 'IEEE_FLOAT') { throw 'Probe format contract mismatch.' }
    Start-Sleep -Milliseconds 500
    if ($process.HasExited) { throw 'Direct recorder ended before native tone started.' }
    $before = @(Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter '*-tone' | Select-Object -ExpandProperty FullName)
    try {
        $tone = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','tone','-ToneSeconds','30') `
            -Stdout (Join-Path $evidence 'tone.txt') -Stderr (Join-Path $evidence 'tone-stderr.txt') -TimeoutSeconds 90
        $codes.Tone = $tone.Code
        $tone | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'tone-process.json') -Encoding UTF8
    } finally {
        # Stop only the recorder/source owned by this invocation.
        New-Item -ItemType File -Path (Join-Path $capture 'stop') | Out-Null
        $newTone = @(Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter '*-tone' | Where-Object { $_.FullName -notin $before })
        if ($newTone.Count -eq 1) { $toneEvidence = $newTone[0].FullName }
    }
    if (-not $process.WaitForExit(10000)) { throw 'Direct recorder did not close within its watchdog.' }
    $codes.Recorder = $process.ExitCode
    if ($codes.Recorder -ne 0) { throw 'Direct recorder failed; preserve probe-stderr.txt and recording.json.' }
    if (-not $toneEvidence) { throw 'Cannot uniquely identify the new Cable A recording.' }
    foreach ($item in @(
        @{ Kind = 'a'; Wav = (Join-Path $toneEvidence 'render-source.wav') },
        @{ Kind = 'b'; Wav = (Join-Path $capture 'cable-b-output.wav') }
    )) {
        $kind = $item.Kind
        $analysis = Invoke-DriverVmProcess -Executable $probe -Arguments @('analyze',$item.Wav,$kind,(Join-Path $evidence "metrics-$kind.json")) `
            -Stdout (Join-Path $evidence "analysis-$kind.txt") -Stderr (Join-Path $evidence "analysis-$kind-stderr.txt") -TimeoutSeconds 20
        $codes["Signal$kind"] = $analysis.Code
        Write-Host "Cable $($kind.ToUpper()) signal analysis: exit $($analysis.Code) (0 = diagnostic passed)."
    }
    $metrics = Get-Content -LiteralPath (Join-Path $evidence 'metrics-b.json') -Raw | ConvertFrom-Json
    $recording = Get-Content -LiteralPath (Join-Path $capture 'recording.json') -Raw | ConvertFrom-Json
    $badPackets = @($recording.packets | Where-Object {
        $_.fileFrame -lt $metrics.fitEndFrame -and ($_.fileFrame + $_.frames) -gt $metrics.fitStartFrame -and ($_.flags -band 5) -ne 0
    })
    if ($badPackets.Count -gt 0) { throw "Cable B has $($badPackets.Count) in-signal discontinuity/timestamp-error packets." }
    if (@($codes.Values | Where-Object { $_ -ne 0 }).Count -gt 0) { throw 'Native counters or recorded signal failed. Send this output; do not repeat or extend the test.' }
} catch { $failure = $_ } finally {
    if ($process) {
        try {
            if (-not $process.HasExited) {
                if (-not (Test-Path -LiteralPath (Join-Path $capture 'stop'))) { New-Item -ItemType File -Path (Join-Path $capture 'stop') | Out-Null }
                if (-not $process.WaitForExit(10000)) { $process.Kill(); [void]$process.WaitForExit(5000) }
            }
        } finally { $process.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run=$run; Passed=(-not [bool]$failure); Codes=$codes; Error=[string]$failure; Qualification=$false }
    $paths = @($evidence)
    if ($toneEvidence) { $paths += $toneEvidence }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $paths -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run recordings and reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Direct signal diagnostic passed. Send the output for waveform review before a longer test.'
