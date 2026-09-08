param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500,
    [string]$RenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$CaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)'
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing event live acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) {
    throw 'DurationMilliseconds must be between 100 and 2000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-event-{0}.exe" -f ([guid]::NewGuid()))

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

function Invoke-Probe([string[]]$Arguments) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $result = @(& $output @Arguments 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    [pscustomobject]@{ Output = $result; ExitCode = $exitCode }
}

if (Test-Path -LiteralPath $object) {
    throw "refusing event acceptance because generated object already exists: $object"
}
$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'event acceptance probe build failed'
    }
    $inventory = Invoke-Probe @('inventory')
    $text = $inventory.Output -join "`n"
    if ($inventory.ExitCode -ne 0) { throw "native endpoint inventory failed`n$text" }
    $renderMatch = [regex]::Match($text, ('render\[(\d+)\] name=' + [regex]::Escape($RenderFriendlyName)))
    $captureMatch = [regex]::Match($text, ('capture\[(\d+)\] name=' + [regex]::Escape($CaptureFriendlyName)))
    if (-not $renderMatch.Success -or -not $captureMatch.Success) {
        throw "Requested event endpoints were not found: '$RenderFriendlyName' / '$CaptureFriendlyName'"
    }
    $renderIndex = [int]$renderMatch.Groups[1].Value
    $captureIndex = [int]$captureMatch.Groups[1].Value
    $capture = Invoke-Probe @('event-capture', $captureIndex, $DurationMilliseconds)
    $render = Invoke-Probe @('event-render', $renderIndex, $DurationMilliseconds)
    $captureText = $capture.Output -join "`n"
    $renderText = $render.Output -join "`n"
    if ($capture.ExitCode -ne 0 -or $captureText -notmatch 'capture_initialize=0x0' -or
        $captureText -notmatch 'capture_set_event=0x0' -or $captureText -notmatch 'capture_start=0x0' -or
        $captureText -notmatch 'capture_stop=0x0' -or $captureText -notmatch 'capture_reset=0x0') {
        throw "event capture lifecycle failed`n$captureText"
    }
    if ($render.ExitCode -ne 0 -or $renderText -notmatch 'render_initialize=0x0' -or
        $renderText -notmatch 'render_set_event=0x0' -or $renderText -notmatch 'render_start=0x0' -or
        $renderText -notmatch 'render_stop=0x0' -or $renderText -notmatch 'render_reset=0x0') {
        throw "event render lifecycle failed`n$renderText"
    }
    $captureFrames = [regex]::Match($captureText, 'capture_frames=(\d+)')
    $renderFrames = [regex]::Match($renderText, 'render_submitted_frames=(\d+)')
    if (-not $captureFrames.Success -or [int]$captureFrames.Groups[1].Value -le 0 -or
        -not $renderFrames.Success -or [int]$renderFrames.Groups[1].Value -le 0) {
        throw "event paths reported no frames`ncapture=$captureText`nrender=$renderText"
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during event acceptance'
    }
    Write-Output ("M00 native event acceptance passed: render='{0}' capture='{1}' capture_frames={2} render_frames={3} duration_ms={4}" -f `
        $RenderFriendlyName, $CaptureFriendlyName, $captureFrames.Groups[1].Value, $renderFrames.Groups[1].Value,
        $DurationMilliseconds)
    Write-Output 'Scope: selected existing endpoints; silent render only; defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged.'
}
finally {
    Remove-Item -LiteralPath $output, $object -Force -ErrorAction SilentlyContinue
}
