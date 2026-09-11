/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    endpoints.h

Abstract:

    Node and Pin numbers and other common definitions for simple audio sample.
--*/

#ifndef _AUDIOROUTERVIRTUAL_ENDPOINTS_H_
#define _AUDIOROUTERVIRTUAL_ENDPOINTS_H_

// Name Guid
// {D3F9A6C1-4B27-4E8D-9A10-62B5C7D8E903}
#define STATIC_NAME_AUDIOROUTER_VIRTUAL\
    0xd3f9a6c1, 0x4b27, 0x4e8d, 0x9a, 0x10, 0x62, 0xb5, 0xc7, 0xd8, 0xe9, 0x03
DEFINE_GUIDSTRUCT("D3F9A6C1-4B27-4E8D-9A10-62B5C7D8E903", NAME_AUDIOROUTER_VIRTUAL);
#define NAME_AUDIOROUTER_VIRTUAL DEFINE_GUIDNAMED(NAME_AUDIOROUTER_VIRTUAL)

//----------------------------------------------------
// New defines for the render endpoints.
//----------------------------------------------------

// Default pin instances.
#define MAX_INPUT_SYSTEM_STREAMS        1

// Wave pins - no mix, no offload
enum
{
    KSPIN_WAVE_RENDER3_SINK_SYSTEM = 0,
    KSPIN_WAVE_RENDER3_SOURCE
};

// Wave pins - offloading is NOT supported.
enum
{
    KSPIN_WAVE_RENDER2_SINK_SYSTEM = 0,
    KSPIN_WAVE_RENDER2_SINK_LOOPBACK,
    KSPIN_WAVE_RENDER2_SOURCE
};

// Wave Topology nodes - offloading is NOT supported.
enum
{
    KSNODE_WAVE_SUM = 0,
    KSNODE_WAVE_VOLUME,
    KSNODE_WAVE_MUTE,
    KSNODE_WAVE_PEAKMETER
};

// Topology pins.
enum
{
    KSPIN_TOPO_WAVEOUT_SOURCE = 0,
    KSPIN_TOPO_LINEOUT_DEST,
};

// Topology nodes.
enum
{
    KSNODE_TOPO_WAVEOUT_VOLUME = 0,
    KSNODE_TOPO_WAVEOUT_MUTE,
    KSNODE_TOPO_WAVEOUT_PEAKMETER
};

//----------------------------------------------------
// New defines for the capture endpoints.
//----------------------------------------------------

// Default pin instances.
#define MAX_INPUT_STREAMS           1       // Number of capture streams.

// Wave pins
enum
{
    KSPIN_WAVE_BRIDGE = 0,
    KSPIN_WAVEIN_HOST,
};

// Wave Topology nodes.
enum
{
    KSNODE_WAVE_ADC = 0
};

// Wave Topology nodes.
enum
{
    KSNODE_WAVE_DAC = 0
};

// topology pins.
enum
{
    KSPIN_TOPO_MIC_ELEMENTS,
    KSPIN_TOPO_BRIDGE
};

// topology nodes.
enum
{
    KSNODE_TOPO_VOLUME,
    KSNODE_TOPO_MUTE,
    KSNODE_TOPO_PEAKMETER
};

// data format attribute range definitions.
static
KSATTRIBUTE PinDataRangeSignalProcessingModeAttribute =
{
    sizeof(KSATTRIBUTE),
    0,
    STATICGUIDOF(KSATTRIBUTEID_AUDIOSIGNALPROCESSING_MODE),
};

static
PKSATTRIBUTE PinDataRangeAttributes[] =
{
    &PinDataRangeSignalProcessingModeAttribute,
};

static
KSATTRIBUTE_LIST PinDataRangeAttributeList =
{
    ARRAYSIZE(PinDataRangeAttributes),
    PinDataRangeAttributes,
};

#endif // _AUDIOROUTERVIRTUAL_ENDPOINTS_H_
