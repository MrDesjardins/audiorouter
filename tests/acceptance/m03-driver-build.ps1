$ErrorActionPreference = 'Stop'

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$build = Join-Path $workspace 'drivers/audiorouter-virtual/build.ps1'
$adapter = Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/adapter.cpp'

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $build
if ($LASTEXITCODE -ne 0) {
    throw "AudioRouter virtual-driver build failed with exit code $LASTEXITCODE"
}

$source = Get-Content -LiteralPath $adapter -Raw
$bridgeHeader = Get-Content -LiteralPath (Join-Path $workspace 'drivers/audiorouter-virtual/Source/Inc/bridgeio.h') -Raw
if (-not $bridgeHeader.Contains('Request->Reserved2 != 0')) {
    throw 'bridge request validation must reject non-zero reserved fields'
}
foreach ($required in @(
        'ReleaseLeasesOwnedByFileObject',
        'IRP_MJ_CLEANUP',
        'IRP_MJ_CLOSE',
        'DriverObject->MajorFunction[IRP_MJ_CLEANUP] = BridgeControlCreateClose',
        'OwnerFileObject == FileObject',
        'RetireBridgeResources(lease, mappedView, sectionObject')) {
    if (-not $source.Contains($required)) {
        throw "driver close cleanup is missing required ownership invariant: $required"
    }
}
$copyStart = $source.IndexOf('NTSTATUS AudioRouterCopyLeaseBlock(')
$copyEnd = $source.IndexOf('static void RetireBridgeResources(', $copyStart)
if ($copyStart -lt 0 -or $copyEnd -le $copyStart) {
    throw 'driver callback lease helper boundary is missing'
}
$copyHelper = $source.Substring($copyStart, $copyEnd - $copyStart)
foreach ($required in @(
        'ExAcquireRundownProtection',
        'InterlockedCompareExchangePointer',
        'InterlockedCompareExchange64',
        'STATUS_RETRY',
        'ExReleaseRundownProtection')) {
    if (-not $copyHelper.Contains($required)) {
        throw "callback lease helper is missing required invariant: $required"
    }
}
if ($copyHelper.Contains('KeAcquireSpinLock')) {
    throw 'callback lease helper must not acquire the lease spin lock'
}

$publishStart = $source.IndexOf('NTSTATUS AudioRouterPublishLeaseBlock(')
$publishEnd = $source.IndexOf('static void RetireBridgeResources(', $publishStart)
if ($publishStart -lt 0 -or $publishEnd -le $publishStart) {
    throw 'capture callback publisher boundary is missing'
}
$publishHelper = $source.Substring($publishStart, $publishEnd - $publishStart)
foreach ($required in @(
        'AR_BRIDGE_DIRECTION_CAPTURE_SINK',
        'InterlockedIncrement64',
        'InterlockedCompareExchange64',
        'STATUS_INTEGER_OVERFLOW',
        'ExReleaseRundownProtection')) {
    if (-not $publishHelper.Contains($required)) {
        throw "capture callback publisher is missing required invariant: $required"
    }
}
if ($publishHelper.Contains('KeAcquireSpinLock')) {
    throw 'capture callback publisher must not acquire the lease spin lock'
}

$stream = Get-Content -LiteralPath (Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/minwavertstream.cpp') -Raw
foreach ($required in @(
        'AudioRouterCopyLeaseBlockForDirection',
        'AudioRouterPublishLeaseBlockForDirection',
        'AudioRouterGetLeaseShapeForDirection',
        'm_BridgeScratch',
        'AR_BRIDGE_DIRECTION_RENDER_SOURCE',
        'AR_BRIDGE_DIRECTION_CAPTURE_SINK',
        'RtlZeroMemory')) {
    if (-not $stream.Contains($required)) {
        throw "WaveRT bridge fill path is missing required fail-closed seam: $required"
    }
}
if (-not $stream.Contains('ReadBytes(ByteDisplacement);')) {
    throw 'WaveRT render consumption must run the bridge publisher even when file diagnostics are disabled'
}

$retireStart = $source.IndexOf('static void RetireBridgeResources(')
$retireEnd = $source.IndexOf('static AR_BRIDGE_LEASE_STATE* BridgeLeaseForDirection(', $retireStart)
if ($retireStart -lt 0 -or $retireEnd -le $retireStart) {
    throw 'driver resource-retirement helper boundary is missing'
}
$retireHelper = $source.Substring($retireStart, $retireEnd - $retireStart)
$waitOffset = $retireHelper.IndexOf('ExWaitForRundownProtectionRelease')
$unmapOffset = $retireHelper.IndexOf('MmUnmapViewInSystemSpace')
if ($waitOffset -lt 0 -or $unmapOffset -lt 0 -or $waitOffset -ge $unmapOffset) {
    throw 'mapped-view retirement must wait for rundown before unmapping'
}

Write-Output 'M03 AudioRouter virtual-driver build acceptance passed'
Write-Output 'Scope: project-owned x64 WDK compile/signability/catalog qualification only; no installation, loading, signing-mode, boot-policy, service, or audio configuration action.'
