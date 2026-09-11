param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500,
    [Parameter(Mandatory = $true)][string]$CaptureEndpointId,
    [Parameter(Mandatory = $true)][string]$RenderEndpointId
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing control-owned route acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) { throw 'DurationMilliseconds must be between 100 and 2000' }
if ([string]::IsNullOrWhiteSpace($CaptureEndpointId) -or [string]::IsNullOrWhiteSpace($RenderEndpointId)) { throw 'Both endpoint IDs are required' }

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
try {
    $output = @(& cargo run --quiet --manifest-path (Join-Path $workspace 'tools/m00-wasapi-probe/Cargo.toml') -- adapter-control-route $DurationMilliseconds $CaptureEndpointId $RenderEndpointId 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "control-owned route failed: $($output -join "`n")" }
    $line = $output | Where-Object { $_ -match '^adapter_control_route ' } | Select-Object -Last 1
    if (-not $line) { throw "control-owned route did not report telemetry: $($output -join "`n")" }
    $generation = [int]([regex]::Match($line, 'generation=(\d+)').Groups[1].Value)
    $packets = [int]([regex]::Match($line, 'packets=(\d+)').Groups[1].Value)
    $captured = [int]([regex]::Match($line, 'captured_frames=(\d+)').Groups[1].Value)
    $quanta = [int]([regex]::Match($line, 'processed_quanta=(\d+)').Groups[1].Value)
    $rendered = [int]([regex]::Match($line, 'rendered_frames=(\d+)').Groups[1].Value)
    $captureRate = [int]([regex]::Match($line, 'capture_rate_hz=(\d+)').Groups[1].Value)
    $renderRate = [int]([regex]::Match($line, 'render_rate_hz=(\d+)').Groups[1].Value)
    if ($line -notmatch 'route=true' -or $generation -ne 1 -or $packets -le 0 -or $captured -le 0 -or $quanta -le 0 -or $rendered -le 0 -or $captureRate -le 0 -or $renderRate -le 0) {
        throw "control-owned route reported invalid telemetry: $line"
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) { throw 'media-device identity/state changed during control-owned route acceptance' }
    Write-Output "M02 control-owned route passed: generation=$generation packets=$packets captured_frames=$captured processed_quanta=$quanta rendered_frames=$rendered capture_rate_hz=$captureRate render_rate_hz=$renderRate"
    Write-Output 'Scope: explicitly selected existing endpoints; worker was stopped/detached and defaults, volume, mute, privacy, drivers, signing, startup configuration, and endpoint registration were unchanged.'
}
finally {
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) { throw 'media-device identity/state changed during control-owned route cleanup' }
}
