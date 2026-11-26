/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minipairs.h

Abstract:

    Local audio endpoint filter definitions.
--*/

#ifndef _AUDIOROUTERVIRTUAL_MINIPAIRS_H_
#define _AUDIOROUTERVIRTUAL_MINIPAIRS_H_

#include "cablewavtable.h"
#include "cabletopotable.h"
#include <devpkey.h>

NTSTATUS CreateMiniportWaveRTAudioRouterVirtual(
    _Out_ PUNKNOWN *, _In_ REFCLSID, _In_opt_ PUNKNOWN, _In_ POOL_FLAGS,
    _In_ PUNKNOWN, _In_opt_ PVOID, _In_ PENDPOINT_MINIPAIR);
NTSTATUS CreateMiniportTopologyAudioRouterVirtual(
    _Out_ PUNKNOWN *, _In_ REFCLSID, _In_opt_ PUNKNOWN, _In_ POOL_FLAGS,
    _In_ PUNKNOWN, _In_opt_ PVOID, _In_ PENDPOINT_MINIPAIR);

static PHYSICALCONNECTIONTABLE CableRenderPhysicalConnections[] = {
    { KSPIN_TOPO_WAVEOUT_SOURCE, KSPIN_WAVE_RENDER3_SOURCE, CONNECTIONTYPE_WAVE_OUTPUT }
};
static PHYSICALCONNECTIONTABLE CableCapturePhysicalConnections[] = {
    { KSPIN_TOPO_BRIDGE, KSPIN_WAVE_BRIDGE, CONNECTIONTYPE_TOPOLOGY_OUTPUT }
};

// Registry configuration, read at StartDevice (adapter.cpp ReadBridgeConfig).
static AR_BRIDGE_CONFIG g_BridgeConfig = {
    AR_CONFIG_CABLE_COUNT_DEFAULT, AR_CONFIG_MIN_PERIOD_FRAMES_DEFAULT,
    AR_CONFIG_DEFAULT_PERIOD_FRAMES_DEFAULT, AR_CONFIG_MAX_LEASE_MS_DEFAULT };

// Low-latency support (17 §5.4): every cable wave filter's KSCATEGORY_AUDIO
// interface carries DEVPKEY_KsAudio_PacketSize_Constraints2, set before the
// interface is enabled (CAdapterCommon::InstallSubdevice). The minimum packet
// period is updated from MinPeriodFrames at StartDevice; no per-mode entries,
// no alignment or maximum-size restriction.
static KSAUDIO_PACKETSIZE_CONSTRAINTS2 g_CablePacketSizeConstraints = {
    26666,                  // 128 frames at 48 kHz; overwritten at StartDevice
    FILE_BYTE_ALIGNMENT,    // any byte alignment
    0,                      // no maximum packet size
    0,                      // no processing-mode specific constraints
    {}
};
static const AUDIOROUTERVIRTUAL_DEVPROPERTY g_CableWaveInterfaceProperties[] = {
    { &DEVPKEY_KsAudio_PacketSize_Constraints2, DEVPROP_TYPE_BINARY,
      sizeof(g_CablePacketSizeConstraints), &g_CablePacketSizeConstraints },
};
// The bus index is implicit in the stable endpoint ordinal: render/capture
// pairs occupy adjacent enum values, so endpoint / 2 is the cable bus.
#define DEFINE_CABLE_PAIR(letter, displayLetter, renderType, captureType) \
DEFINE_CABLE_TOPOLOGY_PAIR(letter) \
static ENDPOINT_MINIPAIR Cable##letter##RenderMiniports = { \
    renderType, L"TopologyCable" displayLetter L"Render", NULL, CreateMiniportTopologyAudioRouterVirtual, \
    &Cable##letter##RenderTopologyFilterDescriptor, 0, NULL, \
    L"WaveCable" displayLetter L"Render", NULL, \
    CreateMiniportWaveRTAudioRouterVirtual, &CableRenderWaveFilterDescriptor, \
    SIZEOF_ARRAY(g_CableWaveInterfaceProperties), g_CableWaveInterfaceProperties, \
    CABLE_DEVICE_MAX_CHANNELS, CableRenderPinFormats, \
    SIZEOF_ARRAY(CableRenderPinFormats), CableRenderPhysicalConnections, \
    SIZEOF_ARRAY(CableRenderPhysicalConnections), ENDPOINT_NO_FLAGS }; \
static ENDPOINT_MINIPAIR Cable##letter##CaptureMiniports = { \
    captureType, L"TopologyCable" displayLetter L"Capture", NULL, CreateMiniportTopologyAudioRouterVirtual, \
    &Cable##letter##CaptureTopologyFilterDescriptor, 0, NULL, \
    L"WaveCable" displayLetter L"Capture", NULL, \
    CreateMiniportWaveRTAudioRouterVirtual, &CableCaptureWaveFilterDescriptor, \
    SIZEOF_ARRAY(g_CableWaveInterfaceProperties), g_CableWaveInterfaceProperties, \
    CABLE_DEVICE_MAX_CHANNELS, CableCapturePinFormats, \
    SIZEOF_ARRAY(CableCapturePinFormats), CableCapturePhysicalConnections, \
    SIZEOF_ARRAY(CableCapturePhysicalConnections), ENDPOINT_NO_FLAGS };

DEFINE_CABLE_PAIR(A, L"A", eCableARender, eCableACapture)
DEFINE_CABLE_PAIR(B, L"B", eCableBRender, eCableBCapture)
DEFINE_CABLE_PAIR(C, L"C", eCableCRender, eCableCCapture)
DEFINE_CABLE_PAIR(D, L"D", eCableDRender, eCableDCapture)
DEFINE_CABLE_PAIR(E, L"E", eCableERender, eCableECapture)
DEFINE_CABLE_PAIR(F, L"F", eCableFRender, eCableFCapture)
DEFINE_CABLE_PAIR(G, L"G", eCableGRender, eCableGCapture)
DEFINE_CABLE_PAIR(H, L"H", eCableHRender, eCableHCapture)
DEFINE_CABLE_PAIR(I, L"I", eCableIRender, eCableICapture)
DEFINE_CABLE_PAIR(J, L"J", eCableJRender, eCableJCapture)
DEFINE_CABLE_PAIR(K, L"K", eCableKRender, eCableKCapture)
DEFINE_CABLE_PAIR(L, L"L", eCableLRender, eCableLCapture)
DEFINE_CABLE_PAIR(M, L"M", eCableMRender, eCableMCapture)
DEFINE_CABLE_PAIR(N, L"N", eCableNRender, eCableNCapture)
DEFINE_CABLE_PAIR(O, L"O", eCableORender, eCableOCapture)
DEFINE_CABLE_PAIR(P, L"P", eCablePRender, eCablePCapture)
#undef DEFINE_CABLE_PAIR

static PENDPOINT_MINIPAIR g_RenderEndpoints[] = {
    &CableARenderMiniports, &CableBRenderMiniports, &CableCRenderMiniports,
    &CableDRenderMiniports, &CableERenderMiniports, &CableFRenderMiniports,
    &CableGRenderMiniports, &CableHRenderMiniports, &CableIRenderMiniports,
    &CableJRenderMiniports, &CableKRenderMiniports, &CableLRenderMiniports,
    &CableMRenderMiniports, &CableNRenderMiniports, &CableORenderMiniports,
    &CablePRenderMiniports,
};
static PENDPOINT_MINIPAIR g_CaptureEndpoints[] = {
    &CableACaptureMiniports, &CableBCaptureMiniports, &CableCCaptureMiniports,
    &CableDCaptureMiniports, &CableECaptureMiniports, &CableFCaptureMiniports,
    &CableGCaptureMiniports, &CableHCaptureMiniports, &CableICaptureMiniports,
    &CableJCaptureMiniports, &CableKCaptureMiniports, &CableLCaptureMiniports,
    &CableMCaptureMiniports, &CableNCaptureMiniports, &CableOCaptureMiniports,
    &CablePCaptureMiniports,
};
// One render and one capture endpoint per bus, in bus order (endpoint / 2).
C_ASSERT(ARRAYSIZE(g_RenderEndpoints) == AR_BRIDGE_MAX_CABLES);
C_ASSERT(ARRAYSIZE(g_CaptureEndpoints) == AR_BRIDGE_MAX_CABLES);
C_ASSERT(eMaxDeviceType == 2 * AR_BRIDGE_MAX_CABLES);
static ULONG g_EnabledCableCount = 2;
#define g_cRenderEndpoints (SIZEOF_ARRAY(g_RenderEndpoints))
#define g_cCaptureEndpoints (SIZEOF_ARRAY(g_CaptureEndpoints))
#define g_MaxMiniports ((g_cRenderEndpoints + g_cCaptureEndpoints) * 2)

#endif // _AUDIOROUTERVIRTUAL_MINIPAIRS_H_
