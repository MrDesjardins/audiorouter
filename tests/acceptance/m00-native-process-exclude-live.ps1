param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing process exclusion acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) {
    throw 'DurationMilliseconds must be between 100 and 2000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-process-exclude-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [System.IO.Path]::ChangeExtension($output, '.obj')

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

if (Test-Path -LiteralPath $object) {
    throw "refusing process exclusion acceptance because generated object already exists: $object"
}
$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'process exclusion acceptance probe build failed'
    }
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $result = @(& $output process-attribution-exclude $DurationMilliseconds 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    $text = $result -join "`n"
    if ($exitCode -ne 0 -or $text -notmatch 'attribution_mode=exclude' -or
        $text -notmatch 'process_activate_result=0x0' -or
        $text -notmatch 'process_capture_start=0x0' -or
        $text -notmatch 'process_capture_stop=0x0' -or
        $text -notmatch 'process_capture_reset=0x0' -or
        $text -notmatch 'attribution_child_exit=0') {
        throw "controlled process exclusion failed`n$text"
    }
    $frames = [regex]::Match($text, 'process_capture_frames=(\d+)')
    if (-not $frames.Success -or [int]$frames.Groups[1].Value -le 0) {
        throw "controlled process exclusion reported no capture frames`n$text"
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during process exclusion acceptance'
    }
    Write-Output ("M00 native process exclusion passed: capture_frames={0} duration_ms={1}" -f `
        $frames.Groups[1].Value, $DurationMilliseconds)
    Write-Output 'Scope: disposable child excluded from the selected process-loopback tree; this validates API mode/lifecycle, not a full cross-process isolation threshold.'
}
finally {
    Remove-Item -LiteralPath $output, $object, $temporaryObject -Force -ErrorAction SilentlyContinue
}
