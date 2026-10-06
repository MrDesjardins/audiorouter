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

// The bus index is implicit in the stable endpoint ordinal: render/capture
// pairs occupy adjacent enum values, so endpoint / 2 is the cable bus.
#define DEFINE_CABLE_PAIR(letter, displayLetter, renderType, captureType) \
static ENDPOINT_MINIPAIR Cable##letter##RenderMiniports = { \
    renderType, L"TopologyCable" displayLetter L"Render", NULL, CreateMiniportTopologyAudioRouterVirtual, \
    &CableRenderTopologyFilterDescriptor, 0, NULL, L"WaveCable" displayLetter L"Render", NULL, \
    CreateMiniportWaveRTAudioRouterVirtual, &CableRenderWaveFilterDescriptor, \
    0, NULL, CABLE_DEVICE_MAX_CHANNELS, CableRenderPinFormats, \
    SIZEOF_ARRAY(CableRenderPinFormats), CableRenderPhysicalConnections, \
    SIZEOF_ARRAY(CableRenderPhysicalConnections), ENDPOINT_NO_FLAGS }; \
static ENDPOINT_MINIPAIR Cable##letter##CaptureMiniports = { \
    captureType, L"TopologyCable" displayLetter L"Capture", NULL, CreateMiniportTopologyAudioRouterVirtual, \
    &CableCaptureTopologyFilterDescriptor, 0, NULL, L"WaveCable" displayLetter L"Capture", NULL, \
    CreateMiniportWaveRTAudioRouterVirtual, &CableCaptureWaveFilterDescriptor, \
    0, NULL, CABLE_DEVICE_MAX_CHANNELS, CableCapturePinFormats, \
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
#undef DEFINE_CABLE_PAIR

static PENDPOINT_MINIPAIR g_RenderEndpoints[] = {
    &CableARenderMiniports, &CableBRenderMiniports, &CableCRenderMiniports,
    &CableDRenderMiniports, &CableERenderMiniports, &CableFRenderMiniports,
    &CableGRenderMiniports, &CableHRenderMiniports,
};
static PENDPOINT_MINIPAIR g_CaptureEndpoints[] = {
    &CableACaptureMiniports, &CableBCaptureMiniports, &CableCCaptureMiniports,
    &CableDCaptureMiniports, &CableECaptureMiniports, &CableFCaptureMiniports,
    &CableGCaptureMiniports, &CableHCaptureMiniports,
};
static ULONG g_EnabledCableCount = 2;
#define g_cRenderEndpoints (SIZEOF_ARRAY(g_RenderEndpoints))
#define g_cCaptureEndpoints (SIZEOF_ARRAY(g_CaptureEndpoints))
#define g_MaxMiniports ((g_cRenderEndpoints + g_cCaptureEndpoints) * 2)

#endif // _AUDIOROUTERVIRTUAL_MINIPAIRS_H_
