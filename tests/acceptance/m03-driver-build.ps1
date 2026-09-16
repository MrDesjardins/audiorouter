[CmdletBinding()]
param(
    [ValidateSet('x64', 'ARM64')]
    [string] $Platform = 'x64'
)

$ErrorActionPreference = 'Stop'

# Some managed shells expose both Path and PATH in the process environment.
# PowerShell's Start-Process cannot materialize that case-insensitive duplicate;
# normalize this test process only. The process exits after the acceptance run,
# so the machine/user environment is not changed.
$existingPath = $env:Path
[Environment]::SetEnvironmentVariable('PATH', $null, 'Process')
[Environment]::SetEnvironmentVariable('Path', $existingPath, 'Process')

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$build = Join-Path $workspace 'drivers/audiorouter-virtual/build.ps1'
$manage = Join-Path $workspace 'drivers/audiorouter-virtual/manage.ps1'
$powershellExe = Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe'
$adapter = Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/adapter.cpp'
$infSource = Get-Content -LiteralPath (Join-Path $workspace 'drivers/audiorouter-virtual/Source/Main/AudioRouterVirtual.inx') -Raw
foreach ($required in @(
        'AUDIOROUTERVIRTUAL.WaveSpeaker.szPname="AudioRouter - Desktop In"',
        'AUDIOROUTERVIRTUAL.WaveMicArray1.szPname="AudioRouter - Voice Chat"',
        'SWD\AudioRouterVirtual')) {
    if (-not $infSource.Contains($required)) {
        throw "driver endpoint identity contract is missing: $required"
    }
}
$buildScript = Get-Content -LiteralPath $build -Raw
$manageScript = Get-Content -LiteralPath $manage -Raw
foreach ($required in @(
        'ParameterSetName = ''Install''',
        'ParameterSetName = ''Uninstall''',
        'Preview',
        'AllowDriverInstall',
        'Refusing to overwrite existing lifecycle state',
        'refusing unmanaged cleanup',
        '$driverRootPrefix',
        'Assert-NoReparsePath',
        'Assert-NoReparseAncestors',
        '$maxStateBytes',
        'Read-BoundedState',
        'Assert-AudioRouterInf',
        'Assert-AudioRouterPackage',
        'package is incomplete',
        'package file cannot be a reparse point',
        'packageFiles',
        'lifecycle state file cannot be a reparse point',
        'lifecycle state file exceeds',
        'FileAttributes]::ReparsePoint',
        'temporaryStatePath',
        'automatic driver rollback',
        '/add-driver',
        '/delete-driver',
        'publishedName -notmatch ''^oem\d+\.inf$''',
        'Lifecycle state INF does not match the requested INF')) {
    if (-not $manageScript.Contains($required)) {
        throw "guarded driver lifecycle control is missing: $required"
    }
}
foreach ($required in @(
        '$outputWasProvided',
        '$outputExistedBeforeBuild',
        '$platformOutputRoot',
        '$outputIsUnderTemp',
        'Preserved caller-owned build output')) {
    if (-not $buildScript.Contains($required)) {
        throw "driver build cleanup guard is missing: $required"
    }
}

$buildOutput = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $build -Platform $Platform 2>&1)
if ($LASTEXITCODE -ne 0) {
    throw "AudioRouter virtual-driver build failed with exit code $LASTEXITCODE"
}
$buildText = $buildOutput -join "`n"
$expectedPlatformRoot = [IO.Path]::Combine($workspace, 'drivers', 'audiorouter-virtual', $Platform)
$unexpectedPlatform = if ($Platform -eq 'x64') { 'ARM64' } else { 'x64' }
if ($buildText -notmatch [regex]::Escape("Driver binary: $expectedPlatformRoot") -or
    $buildText -notmatch [regex]::Escape("Driver INF: $expectedPlatformRoot") -or
    $buildText -match [regex]::Escape("Driver binary: $([IO.Path]::Combine($workspace, 'drivers', 'audiorouter-virtual', $unexpectedPlatform))") -or
    $buildText -match [regex]::Escape("Driver INF: $([IO.Path]::Combine($workspace, 'drivers', 'audiorouter-virtual', $unexpectedPlatform))")) {
    throw "driver build reported an artifact outside the requested $Platform platform root`n$buildText"
}

# Exercise the lifecycle guard against the generated .inf but no state record.
# This must reach the precise missing-state refusal after walking the file's
# parent chain, and must never invoke pnputil or delete a package.
$guardState = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-driver-guard-' + [guid]::NewGuid().ToString('N') + '.json')
$trackedState = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-driver-tracked-' + [guid]::NewGuid().ToString('N') + '.json')
$guardInf = Join-Path $expectedPlatformRoot 'Release/package/AudioRouterVirtual.inf'
$foreignInf = Join-Path $expectedPlatformRoot 'Release/package/audiorouter-foreign-test.inf'
$guardStdout = [IO.Path]::GetTempFileName()
$guardStderr = [IO.Path]::GetTempFileName()
$consentStdout = [IO.Path]::GetTempFileName()
$consentStderr = [IO.Path]::GetTempFileName()
if (-not (Test-Path -LiteralPath $guardInf -PathType Leaf)) {
    throw "generated driver INF is missing for lifecycle regression: $guardInf"
}
try {
    'Class=MEDIA' | Set-Content -LiteralPath $foreignInf -Encoding ASCII -NoNewline
    $identityProcess = Start-Process -FilePath $powershellExe -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage,
        '-Install', '-Preview', '-Inf', $foreignInf, '-State', $guardState
    ) -RedirectStandardOutput $guardStdout -RedirectStandardError $guardStderr -PassThru
    $identityProcess.WaitForExit(); $identityProcess.Refresh()
    $identityOutput = ((Get-Content -LiteralPath $guardStdout -Raw -ErrorAction SilentlyContinue) +
        (Get-Content -LiteralPath $guardStderr -Raw -ErrorAction SilentlyContinue))
    if ($identityProcess.ExitCode -eq 0 -or $identityOutput -notmatch 'not an AudioRouter package') {
        throw "driver lifecycle accepted an unrelated INF: $identityOutput"
    }

    $previewOutput = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $manage `
        -Install -Preview -Inf $guardInf -State $guardState)
    if ($LASTEXITCODE -ne 0) { throw "driver lifecycle preview failed with exit code $LASTEXITCODE" }
    $preview = ($previewOutput -join "`n") | ConvertFrom-Json
    if ($preview.mutates -ne $false -or $preview.action -ne 'install' -or
        $preview.ready -ne $true -or $preview.statePresent -ne $false -or
         $preview.packageFiles.'AudioRouterVirtual.sys' -eq $null -or
         $preview.packageFiles.'AudioRouterVirtual.cat' -eq $null -or
         $preview.consentProvided -ne $false -or $preview.command[0] -ne '/add-driver' -or
        (Test-Path -LiteralPath $guardState)) {
        throw "driver lifecycle preview was not read-only or did not describe install: $($previewOutput -join ' ')"
    }

    $consentProcess = Start-Process -FilePath $powershellExe -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage,
        '-Install', '-Inf', $guardInf, '-State', $guardState
    ) -RedirectStandardOutput $consentStdout -RedirectStandardError $consentStderr -PassThru
    $consentProcess.WaitForExit(); $consentProcess.Refresh()
    $consentOutput = ((Get-Content -LiteralPath $consentStdout -Raw -ErrorAction SilentlyContinue) +
        (Get-Content -LiteralPath $consentStderr -Raw -ErrorAction SilentlyContinue))
    if ($consentProcess.ExitCode -eq 0 -or $consentOutput -notmatch 'require .*AllowDriverInstall') {
        throw "driver install did not fail closed without consent: $consentOutput"
    }

    $uninstallPreviewOutput = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $manage `
        -Uninstall -Preview -Inf $guardInf -State $guardState)
    if ($LASTEXITCODE -ne 0) { throw "driver lifecycle uninstall preview failed with exit code $LASTEXITCODE" }
    $uninstallPreview = ($uninstallPreviewOutput -join "`n") | ConvertFrom-Json
    if ($uninstallPreview.mutates -ne $false -or $uninstallPreview.action -ne 'uninstall' -or
        $uninstallPreview.ready -ne $false -or $uninstallPreview.statePresent -ne $false -or
        $uninstallPreview.blocker -notmatch 'no lifecycle state') {
        throw "driver lifecycle preview did not describe the missing uninstall state: $($uninstallPreviewOutput -join ' ')"
    }

    @{ schemaVersion = 1; inf = $guardInf; publishedName = 'oem123.inf' } |
        ConvertTo-Json | Set-Content -LiteralPath $trackedState -Encoding UTF8 -NoNewline
    $trackedPreviewOutput = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $manage `
        -Uninstall -Preview -Inf $guardInf -State $trackedState)
    if ($LASTEXITCODE -ne 0) { throw "tracked driver lifecycle preview failed with exit code $LASTEXITCODE" }
    $trackedPreview = ($trackedPreviewOutput -join "`n") | ConvertFrom-Json
    if ($trackedPreview.mutates -ne $false -or $trackedPreview.ready -ne $true -or
        $trackedPreview.publishedName -ne 'oem123.inf' -or
        $trackedPreview.command[0] -ne '/delete-driver' -or $trackedPreview.command[1] -ne 'oem123.inf') {
        throw "tracked driver lifecycle preview did not describe uninstall: $($trackedPreviewOutput -join ' ')"
    }

    $guardProcess = Start-Process -FilePath $powershellExe -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage,
        '-Uninstall', '-AllowDriverInstall', '-Inf', $guardInf, '-State', $guardState
    ) -RedirectStandardOutput $guardStdout -RedirectStandardError $guardStderr -PassThru
    $guardProcess.WaitForExit(); $guardProcess.Refresh()
    $guardOutput = ((Get-Content -LiteralPath $guardStdout -Raw -ErrorAction SilentlyContinue) +
        (Get-Content -LiteralPath $guardStderr -Raw -ErrorAction SilentlyContinue))
    if ($guardProcess.ExitCode -eq 0 -or $guardOutput -notmatch 'No lifecycle state exists') {
        throw "driver lifecycle guard did not refuse an untracked package: $guardOutput"
    }
} finally {
    foreach ($guardPath in @($foreignInf, $guardState, $trackedState, $guardStdout, $guardStderr, $consentStdout, $consentStderr)) {
        if (Test-Path -LiteralPath $guardPath) { Remove-Item -LiteralPath $guardPath -Force }
    }
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
        'IoCreateDeviceSecure',
        'AUDIOROUTER_BRIDGE_DEVICE_SDDL',
        'IoCreateSymbolicLink',
        'DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = BridgeControlDeviceControl',
        'DriverObject->MajorFunction[IRP_MJ_CLEANUP] = BridgeControlCreateClose',
        'DriverObject->MajorFunction[IRP_MJ_CLOSE] = BridgeControlCreateClose',
    'IOCTL_AUDIOROUTER_BRIDGE_OPEN &&',
    'IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT',
    'request->SectionHandle == 0',
    'request->SectionHandle != 0',
    'BridgeRequestsHaveSameLeaseIdentity',
    'SectionHandle and MappingBytes',
    'Left->Generation == Right->Generation',
    'RtlCompareMemory(Left->BusId, Right->BusId')) {
    if (-not $source.Contains($required)) {
        throw "driver IOCTL handle-role validation is missing: $required"
    }
}
foreach ($required in @(
        'AudioRouterCopyLeaseBlockForDirection',
        'AudioRouterPublishLeaseBlockForDirection',
        'AudioRouterGetLeaseShapeForDirection')) {
    if (-not $source.Contains($required)) {
        throw "driver bridge direction export is missing: $required"
    }
}
$publishRequestStart = $source.IndexOf('static void PublishBridgeRequest(')
$publishRequestEnd = $source.IndexOf('// This helper is intentionally independent', $publishRequestStart)
if ($publishRequestStart -lt 0 -or $publishRequestEnd -le $publishRequestStart) {
    throw 'callback request publication helper boundary is missing'
}
$publishRequest = $source.Substring($publishRequestStart, $publishRequestEnd - $publishRequestStart)
$generationClear = $publishRequest.IndexOf('Request.Generation), 0)')
$requestCopy = $publishRequest.IndexOf('Lease->Request.ProtocolMajor = Request->ProtocolMajor;')
$generationPublish = $publishRequest.LastIndexOf('Request->Generation))')
if ($generationClear -lt 0 -or $requestCopy -le $generationClear -or
    $generationPublish -le $requestCopy) {
    throw 'callback request publication must clear generation, publish shape, then publish generation'
}
foreach ($required in @(
        'StoreBridgeUshort(&Lease->Request.Channels',
        '&Lease->Request.FramesPerQuantum',
        '&Lease->Request.Direction')) {
    if (-not $publishRequest.Contains($required)) {
        throw "callback request publication is missing atomic shape store: $required"
    }
}
if (-not $source.Contains('ClearBridgeRequest(')) {
    throw 'callback request teardown helper is missing'
}
foreach ($required in @(
        'ReleaseLeasesOwnedByFileObject',
        'IRP_MJ_CLEANUP',
        'IRP_MJ_CLOSE',
        'DriverObject->MajorFunction[IRP_MJ_CLEANUP] = BridgeControlCreateClose',
    'OwnerFileObject == FileObject',
    'lease->Active || lease->Retiring',
    'lease->OwnerFileObject = stack->FileObject',
    'lease->RundownStarted == oldRundownStarted',
    'Do not resurrect a lease for a closed handle',
    'RetireBridgeResources(lease, mappedView, sectionObject')) {
    if (-not $source.Contains($required)) {
        throw "driver close cleanup is missing required ownership invariant: $required"
    }
}
if ($source.Contains('if (lease->Retiring)')) {
    throw 'bridge retirement cleanup must use captured state after rundown, not an unlocked lease read'
}
$deleteStart = $source.IndexOf('void DeleteBridgeControlDevice()')
$deleteEnd = $source.IndexOf('#pragma code_seg("PAGE")', $deleteStart)
if ($deleteStart -lt 0 -or $deleteEnd -le $deleteStart) {
    throw 'bridge unload cleanup boundary is missing'
}
$deleteSource = $source.Substring($deleteStart, $deleteEnd - $deleteStart)
$retireIndex = $deleteSource.IndexOf('RetireBridgeResources(')
$clearIndex = $deleteSource.IndexOf('ClearBridgeRequest(&g_BridgeLeases[index])')
if ($retireIndex -lt 0 -or $clearIndex -le $retireIndex) {
    throw 'bridge unload must clear request identity after rundown retirement'
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
if (-not $readBytesSource.Contains('m_pDmaBuffer == NULL || m_ulDmaBufferSize == 0')) {
    throw 'ReadBytes callback must fail closed before DMA-buffer modulo arithmetic'
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
        'USHORT framesPerQuantum',
        'LoadBridgeUshort(&Lease->Request.FramesPerQuantum)',
        'USHORT channels',
        'LoadBridgeUshort(&Lease->Request.Channels)',
        'framesPerQuantum != Frames',
        'channels != Channels',
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
        'RefreshBridgePublishShape();',
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
$updatePositionStart = $stream.IndexOf('VOID CMiniportWaveRTStream::UpdatePosition')
$writeBytesStart = $stream.IndexOf('VOID CMiniportWaveRTStream::WriteBytes', $updatePositionStart)
if ($updatePositionStart -lt 0 -or $writeBytesStart -le $updatePositionStart) {
    throw 'WaveRT position-update callback boundary is missing'
}
$updatePositionSource = $stream.Substring($updatePositionStart, $writeBytesStart - $updatePositionStart)
foreach ($required in @('WriteBytes(ByteDisplacement);', 'ReadBytes(ByteDisplacement);')) {
    if (-not $updatePositionSource.Contains($required)) {
        throw "WaveRT position-update callback does not reach the bridge path: $required"
    }
}
$timerStart = $stream.LastIndexOf('TimerNotifyRT')
if ($timerStart -lt 0) {
    throw 'WaveRT timer callback boundary is missing'
}
$timerSource = $stream.Substring($timerStart)
if (-not $timerSource.Contains('_this->UpdatePosition(qpc);')) {
    throw 'WaveRT timer callback does not reach the position-update bridge path'
}
$readBytesStart = $stream.IndexOf('VOID CMiniportWaveRTStream::ReadBytes')
$readBytesEnd = $stream.IndexOf('VOID CMiniportWaveRTStream::RefreshBridgePublishShape', $readBytesStart)
if ($readBytesStart -lt 0 -or $readBytesEnd -le $readBytesStart) {
    throw 'WaveRT capture callback boundary is missing'
}
$readBytesSource = $stream.Substring($readBytesStart, $readBytesEnd - $readBytesStart)
if (-not $readBytesSource.Contains('RefreshBridgePublishShape();')) {
    throw 'WaveRT render callback must refresh the capture-sink bridge shape before publication'
}
$writeBytesSource = $stream.Substring($writeBytesStart, $readBytesStart - $writeBytesStart)
if (-not $writeBytesSource.Contains('AudioRouterCopyLeaseBlockForDirection')) {
    throw 'WaveRT render callback must consume only through the bounded bridge helper'
}
if (-not $writeBytesSource.Contains('AR_BRIDGE_DIRECTION_RENDER_SOURCE')) {
    throw 'WaveRT render callback must consume the render-source lease direction'
}
if (-not $writeBytesSource.Contains('header.Channels != bridgeChannels')) {
    throw 'WaveRT render callback must reject a bridge channel-shape mismatch'
}
$callbackBoundaries = @(
    $stream.Substring($stream.IndexOf('VOID CMiniportWaveRTStream::WriteBytes'), $stream.IndexOf('VOID CMiniportWaveRTStream::ReadBytes') - $stream.IndexOf('VOID CMiniportWaveRTStream::WriteBytes')),
    $readBytesSource
)
foreach ($callback in $callbackBoundaries) {
    foreach ($forbidden in @('KeWaitFor', 'KeDelayExecutionThread', 'ExAllocatePool', 'IoQueueWorkItem')) {
        if ($callback.Contains($forbidden)) {
            throw "WaveRT bridge callback contains a forbidden realtime operation: $forbidden"
        }
    }
    $loggingLine = $callback -split "`r?`n" | Where-Object {
        $trimmed = $_.TrimStart()
        $trimmed.StartsWith('DPF_') -or $trimmed.StartsWith('DbgPrint')
    } | Select-Object -First 1
    if ($null -ne $loggingLine) {
        throw 'WaveRT bridge callback contains a forbidden realtime logging call'
    }
}
if (-not $readBytesSource.Contains('AudioRouterPublishLeaseBlockForDirection')) {
    throw 'WaveRT capture callback must publish only through the bounded bridge helper'
}
if (-not $stream.Substring($stream.IndexOf('VOID CMiniportWaveRTStream::WriteBytes')).Contains('RtlZeroMemory')) {
    throw 'WaveRT render callback must retain a fail-closed silence path'
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
if (-not $stream.Contains('ByteDisplacement > MAXULONGLONG - m_ullPresentationPosition')) {
    throw 'WaveRT position callback must reject presentation-position overflow'
}
if (-not $stream.Contains('ByteDisplacement > MAXULONGLONG - m_ullLinearPosition')) {
    throw 'WaveRT position callback must reject linear-position overflow'
}
foreach ($required in @(
        'm_ullPerformanceCounterFrequency.QuadPart <= 0',
        'static_cast<ULONGLONG>(packetCounter) >',
        'MAXULONGLONG / packetSize',
        'hnsElapsedTimeCarryForward >',
        'MAXULONGLONG - ullLinearPosition',
        'advancedLinearPosition < linearPositionOfAvailablePacket',
        'deltaLinearPosition > MAXULONGLONG / 10000000',
        'deltaTimeInHns > ullDmaTimeStamp',
        'timeOfAvailablePacketInHns >')) {
    if (-not $stream.Contains($required)) {
        throw "WaveRT packet timestamp arithmetic guard is missing: $required"
    }
}
foreach ($required in @(
        'm_llPacketCounter == MAXLONGLONG',
        'before the counter can wrap')) {
    if (-not $stream.Contains($required)) {
        throw "WaveRT packet counter overflow guard is missing: $required"
    }
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
if (-not $stream.Contains('m_pMiniport == NULL')) {
    throw 'WaveRT write-position updates must reject a missing miniport owner'
}
if (-not $stream.Contains('pAdapterComm == NULL')) {
    throw 'WaveRT write-position updates must reject a missing adapter owner'
}
if (-not $stream.Contains('ullPresentationPosition > MAXULONGLONG / sampleRate')) {
    throw 'WaveRT presentation position must reject multiplication overflow'
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
Write-Output "Scope: project-owned $Platform WDK compile/signability/catalog qualification only; no installation, loading, signing-mode, boot-policy, service, or audio configuration action."
