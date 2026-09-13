$ErrorActionPreference = 'Stop'

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$build = Join-Path $workspace 'drivers/audiorouter-virtual/build.ps1'
$adapter = Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/adapter.cpp'
$infSource = Get-Content -LiteralPath (Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/AudioRouterVirtual.inx') -Raw
foreach ($required in @(
        'AUDIOROUTERVIRTUAL.WaveSpeaker.szPname="AudioRouter - Desktop In"',
        'AUDIOROUTERVIRTUAL.WaveMicArray1.szPname="AudioRouter - Voice Chat"')) {
    if (-not $infSource.Contains($required)) {
        throw "driver endpoint identity contract is missing: $required"
    }
}
$buildScript = Get-Content -LiteralPath $build -Raw
foreach ($required in @(
        '$outputWasProvided',
        '$outputExistedBeforeBuild',
        '$outputIsUnderTemp',
        'Preserved caller-owned build output')) {
    if (-not $buildScript.Contains($required)) {
        throw "driver build cleanup guard is missing: $required"
    }
}

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
        "Request->BusId[index] == L'\0'",
        "Request->BusId[index] != L'\0'")) {
    if (-not $bridgeHeader.Contains($required)) {
        throw "bridge UTF-16 identity validation is missing: $required"
    }
}
foreach ($required in @(
        'if (Irp == NULL)',
        'NTSTATUS BridgeControlCreateClose',
        'if (stack == NULL)',
        'stack == NULL || stack->FileObject == NULL',
        'IOCTL_AUDIOROUTER_BRIDGE_OPEN &&',
        'request->SectionHandle == 0',
        'code != IOCTL_AUDIOROUTER_BRIDGE_OPEN',
        'request->SectionHandle != 0')) {
    if (-not $source.Contains($required)) {
        throw "driver IOCTL handle-role validation is missing: $required"
    }
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
        'MinimumSequence',
        'AudioRouterCopyBridgeBlock',
        'generation != 0',
        'STATUS_RETRY',
        'ExReleaseRundownProtection')) {
    if (-not $copyHelper.Contains($required)) {
        throw "callback lease helper is missing required invariant: $required"
    }
}
if ($copyHelper.Contains('KeAcquireSpinLock')) {
    throw 'callback lease helper must not acquire the lease spin lock'
}
$streamSource = Get-Content -LiteralPath (Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/minwavertstream.cpp') -Raw
$readBytesStart = $streamSource.IndexOf('VOID CMiniportWaveRTStream::ReadBytes')
$readBytesEnd = $streamSource.IndexOf('#pragma code_seg("PAGE")', $readBytesStart)
if ($readBytesStart -lt 0 -or $readBytesEnd -le $readBytesStart) {
    throw 'ReadBytes callback boundary is missing'
}
$readBytesSource = $streamSource.Substring($readBytesStart, $readBytesEnd - $readBytesStart)
if ($readBytesSource.Contains('m_SaveData.WriteData')) {
    throw 'ReadBytes callback must not perform diagnostic file output'
}
foreach ($required in @(
        'AudioRouterGetLeaseShapeForDirection(',
        'm_BridgeScratchFrames = 0;',
        'm_BridgeScratchFrameOffset = 0;',
        'm_BridgeScratchFrames > m_BridgePublishFrames',
        'stale frame state',
        'AdvanceDmaOffset(',
        'nextWritePosition')) {
    if (-not $streamSource.Contains($required)) {
        throw "WaveRT bridge scratch-shape guard is missing: $required"
    }
}

$publishStart = $source.IndexOf('NTSTATUS AudioRouterPublishLeaseBlock(')
$publishEnd = $source.IndexOf('static void RetireBridgeResources(', $publishStart)
if ($publishStart -lt 0 -or $publishEnd -le $publishStart) {
    throw 'capture callback publisher boundary is missing'
}
$publishHelper = $source.Substring($publishStart, $publishEnd - $publishStart)
foreach ($required in @(
        'AR_BRIDGE_DIRECTION_CAPTURE_SINK',
        'Request.FramesPerQuantum != Frames',
        'Request.Channels != Channels',
        'InterlockedIncrement64',
        'InterlockedCompareExchange64',
        'generation == 0',
        'nextSequence == MAXULONGLONG',
        'STATUS_DATA_ERROR',
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
        'm_BridgeReadSequence',
        'header.Sequence',
        'header.Frames',
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
if (-not $stream.Contains('IID_IMiniportWaveRTOutputStream) && (!this->m_bCapture)')) {
    throw 'WaveRT capture streams must not advertise the render-stream interface'
}
if (-not $stream.Contains('m_pDmaBuffer == NULL || m_ulDmaBufferSize == 0 || m_ulDmaMovementRate == 0')) {
    throw 'WaveRT position callback must fail closed before DMA buffer arithmetic'
}
if (-not $stream.Contains('m_ullPerformanceCounterFrequency.QuadPart == 0')) {
    throw 'WaveRT position callback must reject an uninitialized performance-counter frequency'
}
if (-not $stream.Contains('static_cast<ULONGLONG>(ilQPC.QuadPart) < m_ullDmaTimeStamp')) {
    throw 'WaveRT position callback must reject a backwards performance-counter sample'
}
if (-not $stream.Contains('ULONGLONG byteNumerator = static_cast<ULONGLONG>(m_ulDmaMovementRate)')) {
    throw 'WaveRT DMA displacement arithmetic must widen before multiplication'
}
if (-not $stream.Contains('byteDisplacementWide > MAXULONG')) {
    throw 'WaveRT DMA displacement must fail closed when it exceeds ULONG capacity'
}
if (-not $stream.Contains('static_cast<ULONGLONG>(qpc.QuadPart) < _this->m_ullLastDPCTimeStamp')) {
    throw 'WaveRT timer callback must reject a backwards QPC sample before conversion'
}
if (-not $stream.Contains('ULONGLONG intervalHns = static_cast<ULONGLONG>(_this->m_ulNotificationIntervalMs) * 10000')) {
    throw 'WaveRT timer notification arithmetic must widen before interval multiplication'
}
if (-not $stream.Contains('if (_this->m_pMiniport == NULL)')) {
    throw 'WaveRT timer callback must guard its miniport owner'
}
if (-not $stream.Contains('PacketNumber == NULL || Flags == NULL')) {
    throw 'WaveRT packet query must validate output pointers'
}
if (-not $stream.Contains('if (Position_ == NULL)')) {
    throw 'WaveRT position query must validate its output pointer'
}
if (-not $stream.Contains('m_pDmaBuffer == NULL')) {
    throw 'WaveRT DMA allocation must reject a failed mapping'
}
if (-not $stream.Contains('m_pPortStream == NULL')) {
    throw 'WaveRT DMA allocation must reject a missing PortCls stream owner'
}
if (-not $stream.Contains('RequestedSize_ > (MAXULONG / 4)')) {
    throw 'WaveRT notification allocation must bound diagnostic-size multiplication'
}
if (-not $stream.Contains('static_cast<ULONGLONG>(RequestedSize_) * 1000')) {
    throw 'WaveRT notification timing arithmetic must widen before multiplication'
}
if (-not $stream.Contains('m_pDmaBuffer != NULL && m_pPortStream != NULL')) {
    throw 'WaveRT buffer teardown must not dereference a missing PortCls stream owner'
}
if (-not $stream.Contains('if (NotificationEvent_ == NULL)')) {
    throw 'WaveRT notification registration must reject a null event'
}
if (-not $stream.Contains('if (Latency_ == NULL)')) {
    throw 'WaveRT hardware-latency query must handle a null output pointer'
}
if (-not $stream.Contains('if (drmRights == NULL || m_pMiniport == NULL)')) {
    throw 'WaveRT content-id handling must validate DRM rights and miniport ownership'
}
if (-not $stream.Contains('m_pNotificationTimer = NULL;')) {
    throw 'WaveRT constructor must initialize the notification timer owner before allocation'
}
if (-not $stream.Contains('m_ulNotificationIntervalMs > 0 && m_pNotificationTimer == NULL')) {
    throw 'WaveRT RUN transition must fail closed without its notification timer owner'
}
if (-not $stream.Contains('if (!NT_SUCCESS(ntStatus))')) {
    throw 'WaveRT state transitions must not publish a failed state change'
}
if (-not $stream.Contains('m_pPortStream->FreePagesFromMdl(pBufferMdl)')) {
    throw 'WaveRT DMA mapping failure must release allocated pages'
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
