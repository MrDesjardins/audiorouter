param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing process attribution acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) {
    throw 'DurationMilliseconds must be between 100 and 2000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-process-{0}.exe" -f ([guid]::NewGuid()))

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

if (Test-Path -LiteralPath $object) {
    throw "refusing process acceptance because generated object already exists: $object"
}
$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'process attribution acceptance probe build failed'
    }
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $result = @(& $output process-attribution $DurationMilliseconds 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    $text = $result -join "`n"
    if ($exitCode -ne 0 -or $text -notmatch 'process_activate_result=0x0' -or
        $text -notmatch 'process_capture_start=0x0' -or $text -notmatch 'process_capture_stop=0x0' -or
        $text -notmatch 'process_capture_reset=0x0' -or $text -notmatch 'attribution_child_exit=0') {
        throw "controlled process attribution failed`n$text"
    }
    $frames = [regex]::Match($text, 'process_capture_frames=(\d+)')
    $payload = [regex]::Match($text, 'process_capture_nonzero_bytes=(\d+)')
    if (-not $frames.Success -or [int]$frames.Groups[1].Value -le 0 -or
        -not $payload.Success -or [int64]$payload.Groups[1].Value -le 0) {
        throw "controlled process attribution reported no data`n$text"
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during process attribution acceptance'
    }
    Write-Output ("M00 native process attribution passed: capture_frames={0} nonzero_bytes={1} duration_ms={2}" -f `
        $frames.Groups[1].Value, $payload.Groups[1].Value, $DurationMilliseconds)
    Write-Output 'Scope: disposable child and selected process tree only; no persistent audio configuration changed.'
}
finally {
    Remove-Item -LiteralPath $output, $object -Force -ErrorAction SilentlyContinue
}
