param(
    [switch]$AllowLiveAudio,
    [int]$ImpulseCount = 1000,
    [int]$IntervalMilliseconds = 10,
    [string]$RenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$CaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)'
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing impulse acceptance without explicit -AllowLiveAudio' }
if ($ImpulseCount -lt 10 -or $ImpulseCount -gt 2000) { throw 'ImpulseCount must be between 10 and 2000' }
if ($IntervalMilliseconds -ne 10) { throw 'The native impulse probe currently emits a fixed 10 ms interval' }

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m00-impulse-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [IO.Path]::ChangeExtension($output, '.obj')
$raw = "$output.raw"
$captureLog = "$output.capture.log"
$impulseLog = "$output.impulse.log"

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}
function Invoke-Probe([string[]]$Arguments) {
    $saved = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $result = @(& $output @Arguments 2>&1); $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    [pscustomobject]@{ Output = $result; ExitCode = $exitCode }
}

if (Test-Path -LiteralPath $object) { throw "generated object already exists: $object" }
$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output)) { throw 'impulse probe build failed' }
    $inventory = Invoke-Probe @('inventory')
    $inventoryText = $inventory.Output -join "`n"
    $renderMatch = [regex]::Match($inventoryText, ('render\[(\d+)\] name=' + [regex]::Escape($RenderFriendlyName)))
    $captureMatch = [regex]::Match($inventoryText, ('capture\[(\d+)\] name=' + [regex]::Escape($CaptureFriendlyName)))
    if (-not $renderMatch.Success -or -not $captureMatch.Success) {
        throw "requested endpoints were not both found: '$RenderFriendlyName' / '$CaptureFriendlyName'"
    }
    $renderIndex = [int]$renderMatch.Groups[1].Value
    $captureIndex = [int]$captureMatch.Groups[1].Value
    $captureDuration = $ImpulseCount * $IntervalMilliseconds + 500
    $capture = Start-Process -FilePath $output -ArgumentList @('capture-file', $captureIndex, $captureDuration, $raw) `
        -RedirectStandardOutput $captureLog -RedirectStandardError "$captureLog.err" -PassThru
    Start-Sleep -Milliseconds 150
    $impulse = Start-Process -FilePath $output -ArgumentList @('impulse', ($ImpulseCount * $IntervalMilliseconds), $renderIndex) `
        -RedirectStandardOutput $impulseLog -RedirectStandardError "$impulseLog.err" -PassThru
    $capture.WaitForExit(); $impulse.WaitForExit()
    $captureText = Get-Content -LiteralPath $captureLog -Raw
    $impulseText = Get-Content -LiteralPath $impulseLog -Raw
    $after = Get-MediaSnapshot
    if (Compare-Object $before $after) { throw 'media-device identity/state changed during impulse acceptance' }
    if ($captureText -notmatch 'capture_start=0x0' -or $captureText -notmatch 'capture_stop=0x0' -or $captureText -notmatch 'capture_reset=0x0') { throw "capture lifecycle failed`n$captureText" }
    if ($impulseText -notmatch 'render_start=0x0' -or $impulseText -notmatch 'render_stop=0x0' -or $impulseText -notmatch 'render_reset=0x0' -or $impulseText -notmatch 'render_impulse_written=1') { throw "impulse lifecycle failed`n$impulseText" }

    $formatMatch = [regex]::Match($captureText, 'rate=(\d+) channels=(\d+) bits=(\d+)')
    if (-not $formatMatch.Success -or [int]$formatMatch.Groups[3].Value -ne 32) { throw 'impulse analyzer requires 32-bit capture' }
    $rate = [int]$formatMatch.Groups[1].Value; $channels = [int]$formatMatch.Groups[2].Value
    $bytes = [IO.File]::ReadAllBytes($raw)
    $bytesPerFrame = $channels * 4
    if ($bytes.Length -eq 0 -or $bytes.Length % $bytesPerFrame -ne 0) { throw 'captured raw file has an invalid frame shape' }
    $groups = [Collections.Generic.List[int]]::new(); $lastHit = -1000000
    for ($offset = 0; $offset + $bytesPerFrame -le $bytes.Length; $offset += $bytesPerFrame) {
        $peak = 0.0
        for ($channel = 0; $channel -lt $channels; $channel++) {
            $sample = [Math]::Abs([BitConverter]::ToSingle($bytes, $offset + ($channel * 4)))
            if ($sample -gt $peak) { $peak = $sample }
        }
        $frame = [int]($offset / $bytesPerFrame)
        if ($peak -gt 0.05 -and $frame -gt ($lastHit + 8)) { $groups.Add($frame); $lastHit = $frame }
    }
    if ($groups.Count -lt [Math]::Floor($ImpulseCount * 0.9)) { throw "only $($groups.Count) impulse groups detected; expected at least $([Math]::Floor($ImpulseCount * 0.9))" }
    $intervalFrames = [int]($rate * $IntervalMilliseconds / 1000)
    $errors = [Collections.Generic.List[int]]::new()
    for ($index = 1; $index -lt $groups.Count; $index++) { $errors.Add([Math]::Abs(($groups[$index] - $groups[$index - 1]) - $intervalFrames)) }
    $sortedErrors = @($errors | Sort-Object)
    $p95Error = $sortedErrors[[Math]::Min($sortedErrors.Count - 1, [Math]::Floor($sortedErrors.Count * 0.95))]
    $captureTick = [int64]([regex]::Match($captureText, 'capture_start_tick_ms=(\d+)').Groups[1].Value)
    $renderTick = [int64]([regex]::Match($impulseText, 'render_start_tick_ms=(\d+)').Groups[1].Value)
    $launchFrames = [Math]::Round(($renderTick - $captureTick) * $rate / 1000.0)
    $onsetFrames = $groups[0] - $launchFrames
    Write-Output ("M00 native impulse analysis passed: render='{0}' capture='{1}' detected_groups={2} expected_impulses={3} p95_spacing_error_frames={4} estimated_onset_ms={5:N2}" -f `
        $RenderFriendlyName, $CaptureFriendlyName, $groups.Count, $ImpulseCount, $p95Error, ($onsetFrames * 1000.0 / $rate))
    Write-Output 'Scope: bounded signal correlation only; the estimated onset is not the required acoustic p95 latency gate without calibrated impulse timestamps and a validated physical setup.'
}
finally {
    $cleanupPaths = @($output, $temporaryObject, $raw, $captureLog, "$captureLog.err", $impulseLog, "$impulseLog.err", $object)
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        Remove-Item -LiteralPath $cleanupPaths -Force -ErrorAction SilentlyContinue
        if (-not (Test-Path -LiteralPath $object)) { break }
        Start-Sleep -Milliseconds 100
    }
}
