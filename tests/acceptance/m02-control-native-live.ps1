param(
    [switch]$AllowLiveAudio,
    [string]$CaptureEndpointId = '',
    [string]$RenderEndpointId = '',
    [string]$OutputFanoutEndpointIds = ''
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) {
    throw 'Refusing control-owned native acceptance without explicit -AllowLiveAudio'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw "missing CLI binary: $cli" }

$savedEnvironment = @{}
foreach ($name in @('AUDIOROUTER_ALLOW_LIVE_AUDIO', 'AUDIOROUTER_CAPTURE_ENDPOINT_ID', 'AUDIOROUTER_RENDER_ENDPOINT_ID', 'AUDIOROUTER_OUTPUT_FANOUT_ENDPOINT_IDS')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    if ([string]::IsNullOrWhiteSpace($CaptureEndpointId) -or
        [string]::IsNullOrWhiteSpace($RenderEndpointId) -or
        [string]::IsNullOrWhiteSpace($OutputFanoutEndpointIds)) {
        $inventoryText = @(& $cli devices list --json 2>&1)
        if ($LASTEXITCODE -ne 0) { throw "endpoint inventory failed: $($inventoryText -join "`n")" }
        $inventory = ($inventoryText -join "`n") | ConvertFrom-Json
        if ([string]::IsNullOrWhiteSpace($CaptureEndpointId)) {
            $CaptureEndpointId = @($inventory | Where-Object { $_.state -eq 'active' -and $_.direction -eq 'capture' -and $_.name -eq 'CABLE Output (VB-Audio Virtual Cable)' } | Select-Object -First 1 -ExpandProperty id)
        }
        if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) {
            $RenderEndpointId = @($inventory | Where-Object { $_.state -eq 'active' -and $_.direction -eq 'render' -and $_.name -eq 'CABLE Input (VB-Audio Virtual Cable)' } | Select-Object -First 1 -ExpandProperty id)
        }
    }
    if ([string]::IsNullOrWhiteSpace($CaptureEndpointId) -or [string]::IsNullOrWhiteSpace($RenderEndpointId)) { throw 'both exact active endpoint IDs are required (or the default CABLE pair must be present)' }
    if ([string]::IsNullOrWhiteSpace($OutputFanoutEndpointIds)) {
        $fanout = @($inventory | Where-Object {
            $_.state -eq 'active' -and
            $_.direction -eq 'render' -and
            $_.name -eq 'CABLE In 16ch (VB-Audio Virtual Cable)' -and
            $_.id -ne $RenderEndpointId
        } | Select-Object -First 1 -ExpandProperty id)
        if ($fanout.Count -gt 0) { $OutputFanoutEndpointIds = $fanout[0] }
    }
    $env:AUDIOROUTER_ALLOW_LIVE_AUDIO = '1'
    $env:AUDIOROUTER_CAPTURE_ENDPOINT_ID = $CaptureEndpointId
    $env:AUDIOROUTER_RENDER_ENDPOINT_ID = $RenderEndpointId
    $env:AUDIOROUTER_OUTPUT_FANOUT_ENDPOINT_IDS = $OutputFanoutEndpointIds
    & cargo test -p audiorouter-control --offline guarded_live_native_endpoint_session_lifecycle -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'control-owned native lifecycle acceptance failed' }
    Write-Output 'M02 control-owned native lifecycle acceptance passed'
    Write-Output 'Scope: exact existing endpoints, optional VB-Cable output fan-out, 500 ms graph delivery, and same-process start/stop; no persistent audio configuration.'
}
finally {
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process') }
}
