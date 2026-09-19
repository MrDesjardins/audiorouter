param(
    [switch]$AllowLiveAudio,
    [string]$CaptureEndpointIds = '',
    [string]$RenderEndpointIds = ''
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) {
    throw 'Refusing multi-input native acceptance without explicit -AllowLiveAudio'
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Refusing multi-input native acceptance without an elevated administrator process; rerun from an elevated shell'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw "missing CLI binary: $cli" }

$savedEnvironment = @{}
foreach ($name in @(
    'AUDIOROUTER_ALLOW_LIVE_AUDIO',
    'AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS',
    'AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS'
)) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

try {
    $inventoryText = @(& $cli devices list --json 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "endpoint inventory failed: $($inventoryText -join "`n")" }
    $inventory = ($inventoryText -join "`n") | ConvertFrom-Json
    $captures = @()
    $renders = @()
    if ([string]::IsNullOrWhiteSpace($CaptureEndpointIds)) {
        $captures = @($inventory | Where-Object {
            $_.state -eq 'active' -and $_.direction -eq 'capture' -and
            $_.format.channels -eq 2 -and $_.format.sampleRateHz -eq 48000 -and
            $_.format.bitsPerSample -eq 32
        } | Sort-Object @{ Expression = { if ($_.name -match '^CABLE Output \(') { 0 } elseif ($_.name -notmatch 'CABLE') { 1 } else { 2 } } }, name |
            Select-Object -First 2 -ExpandProperty id)
        $CaptureEndpointIds = $captures -join '|'
    }
    if ([string]::IsNullOrWhiteSpace($RenderEndpointIds)) {
        $renders = @($inventory | Where-Object {
            $_.state -eq 'active' -and $_.direction -eq 'render' -and
            $_.format.channels -eq 2 -and $_.format.sampleRateHz -eq 48000 -and
            $_.format.bitsPerSample -eq 32
        } | Sort-Object @{ Expression = { if ($_.name -match '^CABLE Input \(') { 0 } elseif ($_.name -notmatch 'CABLE') { 1 } else { 2 } } }, name |
            Select-Object -First 2 -ExpandProperty id)
        $RenderEndpointIds = $renders -join '|'
    }
    if (($CaptureEndpointIds -split '\|' | Where-Object { $_ }).Count -lt 2) {
        throw 'at least two exact active stereo 48 kHz capture endpoints are required'
    }
    if (($RenderEndpointIds -split '\|' | Where-Object { $_ }).Count -lt 2) {
        throw 'at least two exact active stereo 48 kHz render endpoints are required'
    }
    $selectedCaptureNames = @($inventory | Where-Object { $_.id -in ($CaptureEndpointIds -split '\|') } | Select-Object -ExpandProperty name)
    $selectedRenderNames = @($inventory | Where-Object { $_.id -in ($RenderEndpointIds -split '\|') } | Select-Object -ExpandProperty name)
    Write-Output "Selected capture endpoints: $($selectedCaptureNames -join ' | ')"
    Write-Output "Selected render endpoints: $($selectedRenderNames -join ' | ')"
    $env:AUDIOROUTER_ALLOW_LIVE_AUDIO = '1'
    $env:AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS = $CaptureEndpointIds
    $env:AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS = $RenderEndpointIds
    & cargo test -p audiorouter-control guarded_live_native_multi_input_many_output_lifecycle --offline -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'multi-input native lifecycle acceptance failed' }
    Write-Output 'M02 multi-input native lifecycle acceptance passed'
    Write-Output 'Scope: two or more exact active capture/render endpoints, 500 ms bounded multi-input pump, and same-process cleanup; no persistent audio configuration.'
}
finally {
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
}
