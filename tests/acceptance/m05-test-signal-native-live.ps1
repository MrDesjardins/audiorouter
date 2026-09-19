[CmdletBinding()]
param(
    [switch]$AllowLiveAudio,
    [Parameter(Mandatory = $true)][string]$CaptureEndpointId,
    [Parameter(Mandatory = $true)][string]$RenderEndpointId
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) {
    throw 'Refusing Test Signal native acceptance without explicit -AllowLiveAudio'
}
if ([string]::IsNullOrWhiteSpace($CaptureEndpointId) -or
    [string]::IsNullOrWhiteSpace($RenderEndpointId)) {
    throw 'Both exact capture and render endpoint IDs are required'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$savedEnvironment = @{}
foreach ($name in @(
        'AUDIOROUTER_ALLOW_LIVE_AUDIO',
        'AUDIOROUTER_CAPTURE_ENDPOINT_ID',
        'AUDIOROUTER_RENDER_ENDPOINT_ID')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    $env:AUDIOROUTER_ALLOW_LIVE_AUDIO = '1'
    $env:AUDIOROUTER_CAPTURE_ENDPOINT_ID = $CaptureEndpointId
    $env:AUDIOROUTER_RENDER_ENDPOINT_ID = $RenderEndpointId
    Push-Location $workspace
    try {
        & cargo test --locked -p audiorouter-control `
            guarded_live_test_signal_reaches_destination_meter -- --ignored --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Test Signal native acceptance failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
    Write-Output 'M05 native Test Signal and destination-meter acceptance passed'
    Write-Output 'Scope: exact existing endpoints and temporary streams; plan/commit, start, bounded meter observation, stop, and cleanup only.'
}
finally {
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
}
