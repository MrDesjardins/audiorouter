param(
    [switch]$AllowLiveAudio,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][uint64]$CreationTime100ns,
    [ValidateSet('include', 'exclude')][string]$Mode = 'include',
    [string]$ApplicationPath = '',
    [string]$RenderEndpointId = ''
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing application-capture acceptance without explicit -AllowLiveAudio' }
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw "missing CLI binary: $cli" }

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$names = @(
    'AUDIOROUTER_ALLOW_LIVE_AUDIO',
    'AUDIOROUTER_APPLICATION_PROCESS_ID',
    'AUDIOROUTER_APPLICATION_EXECUTABLE',
    'AUDIOROUTER_APPLICATION_CREATION_TIME_100NS',
    'AUDIOROUTER_APPLICATION_MODE',
    'AUDIOROUTER_APPLICATION_PATH',
    'AUDIOROUTER_RENDER_ENDPOINT_ID'
)
$savedEnvironment = @{}
foreach ($name in $names) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$before = Get-MediaSnapshot
try {
    if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) {
        $inventoryText = @(& $cli devices list --json 2>&1)
        if ($LASTEXITCODE -ne 0) { throw "endpoint inventory failed: $($inventoryText -join "`n")" }
        $inventory = ($inventoryText -join "`n") | ConvertFrom-Json
        $RenderEndpointId = @($inventory |
            Where-Object { $_.state -eq 'active' -and $_.direction -eq 'render' -and $_.name -eq 'CABLE Input (VB-Audio Virtual Cable)' } |
            Select-Object -First 1 -ExpandProperty id)
    }
    if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) { throw 'an exact active render endpoint ID is required (or the CABLE Input endpoint must be present)' }

    $env:AUDIOROUTER_ALLOW_LIVE_AUDIO = '1'
    $env:AUDIOROUTER_APPLICATION_PROCESS_ID = $ProcessId.ToString()
    $env:AUDIOROUTER_APPLICATION_EXECUTABLE = $Executable
    $env:AUDIOROUTER_APPLICATION_CREATION_TIME_100NS = $CreationTime100ns.ToString()
    $env:AUDIOROUTER_APPLICATION_MODE = $Mode
    if ([string]::IsNullOrWhiteSpace($ApplicationPath)) {
        Remove-Item Env:AUDIOROUTER_APPLICATION_PATH -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_APPLICATION_PATH = $ApplicationPath
    }
    $env:AUDIOROUTER_RENDER_ENDPOINT_ID = $RenderEndpointId

    & cargo test -p audiorouter-control --offline guarded_live_native_application_worker_lifecycle -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'control-owned application-capture lifecycle acceptance failed' }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during application-capture acceptance'
    }
    Write-Output 'M02 control-owned application-capture lifecycle acceptance passed'
    Write-Output ("Scope: exact process identity, mode={0}, two bounded start/pump/stop cycles, same-process worker restart, and unchanged media-device state." -f $Mode)
}
finally {
    foreach ($name in $names) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during application-capture cleanup'
    }
}
