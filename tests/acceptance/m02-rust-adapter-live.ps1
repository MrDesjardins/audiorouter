param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500
)

$ErrorActionPreference = 'Stop'

if (-not $AllowLiveAudio) {
    throw 'Refusing live audio acceptance without explicit -AllowLiveAudio'
}
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 5000) {
    throw 'DurationMilliseconds must be between 100 and 5000'
}

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
$savedErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$output = @(cargo run --manifest-path tools\m00-wasapi-probe\Cargo.toml -- adapter-smoke $DurationMilliseconds 2>&1)
$cargoExitCode = $LASTEXITCODE
$ErrorActionPreference = $savedErrorActionPreference
if ($cargoExitCode -ne 0) {
    throw "production Rust adapter smoke failed with exit code $cargoExitCode`n$($output -join [Environment]::NewLine)"
}
$line = $output | Where-Object { $_ -match '^adapter_smoke ' } | Select-Object -Last 1
if (-not $line -or $line -notmatch 'capture_packets=(\d+)' -or $line -notmatch 'capture_frames=(\d+)' -or $line -notmatch 'render_frames=(\d+)') {
    throw "adapter smoke did not report bounded capture/render counts`n$($output -join [Environment]::NewLine)"
}
$capturePackets = [int]([regex]::Match($line, 'capture_packets=(\d+)').Groups[1].Value)
if ($capturePackets -le 0) {
    throw 'adapter smoke reported no capture packets'
}
$captureFrames = [int]([regex]::Match($line, 'capture_frames=(\d+)').Groups[1].Value)
$renderFrames = [int]([regex]::Match($line, 'render_frames=(\d+)').Groups[1].Value)
if ($captureFrames -le 0 -or $renderFrames -le 0) {
    throw "adapter smoke reported invalid frame counts: $line"
}
$after = Get-MediaSnapshot
if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
    throw 'media-device identity/state changed during live adapter acceptance'
}
Write-Output "M02 production Rust adapter live acceptance passed: $line"
Write-Output 'Scope: explicit bounded live capture plus silent render; streams stop/reset; media-device identity/state unchanged.'
