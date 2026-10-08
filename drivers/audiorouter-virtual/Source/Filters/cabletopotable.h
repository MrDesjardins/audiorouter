/*++

Module Name: cabletopotable.h

Abstract: Minimal direct-path topology tables for virtual cables.
--*/

#ifndef _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_
#define _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_

// The INF generator keeps these per-cable categories aligned with the
// device-software-key names used by AudioEndpointBuilder.
// BEGIN GENERATED CABLE PIN CATEGORY GUIDS
static const GUID CableARenderPinCategory = {0x45faca63,0x8302,0x4bb2,{0xb1,0x2a,0x2e,0x02,0xb2,0xf0,0xf6,0x91}};
static const GUID CableACapturePinCategory = {0xa978b404,0x531f,0x4b33,{0xbf,0x37,0x4a,0xdd,0x65,0x95,0xd9,0x3f}};
static const GUID CableBRenderPinCategory = {0x7fd23486,0xd370,0x4ebd,{0xb9,0xe0,0xc2,0x0f,0x2f,0x75,0x1c,0x89}};
static const GUID CableBCapturePinCategory = {0x8fd441b5,0x651e,0x45a2,{0x95,0x6a,0xb8,0x78,0xc2,0x0d,0xa3,0x86}};
static const GUID CableCRenderPinCategory = {0x66f3b079,0x5a39,0x4f4d,{0x8a,0x32,0x5a,0x4b,0x23,0xd1,0x8d,0x4d}};
static const GUID CableCCapturePinCategory = {0xdd4080a7,0x1f79,0x4381,{0xa2,0x7c,0xea,0xff,0xf8,0x30,0x24,0xca}};
static const GUID CableDRenderPinCategory = {0x0714e52a,0x05fd,0x44a7,{0xaa,0x99,0x70,0x55,0xbf,0xc4,0x0c,0x6c}};
static const GUID CableDCapturePinCategory = {0x51279b57,0x57d7,0x4340,{0x98,0xe4,0x52,0x0d,0xa8,0x03,0xe5,0x0c}};
static const GUID CableERenderPinCategory = {0x7aad623e,0x878f,0x46f5,{0xbc,0xbd,0x79,0x0d,0x1e,0xd6,0xd3,0x74}};
static const GUID CableECapturePinCategory = {0x89cd2009,0xd9ef,0x4dd3,{0xa6,0xc9,0xca,0x85,0x9d,0xa7,0xee,0x7a}};
static const GUID CableFRenderPinCategory = {0x81cfe35b,0x23b8,0x4715,{0x89,0xb2,0x09,0x30,0x8e,0x23,0x74,0x71}};
static const GUID CableFCapturePinCategory = {0x8f29dc80,0x794d,0x4bfa,{0xa3,0x15,0xa2,0xc4,0x52,0x4b,0xda,0x72}};
static const GUID CableGRenderPinCategory = {0xb5a1784e,0x1324,0x4b3f,{0xac,0x86,0x2a,0x3a,0x4c,0xfd,0x18,0xf9}};
static const GUID CableGCapturePinCategory = {0xc45b9a3f,0xf715,0x47c1,{0x9a,0xf9,0x28,0xf8,0x35,0x63,0xc7,0x8a}};
static const GUID CableHRenderPinCategory = {0x88583ceb,0xd6b6,0x4e78,{0x86,0x72,0x53,0xe4,0xed,0x95,0xba,0x32}};
static const GUID CableHCapturePinCategory = {0x637b0783,0xf72a,0x48af,{0x8e,0x52,0x6b,0xd8,0xf4,0x72,0x77,0xb9}};
// END GENERATED CABLE PIN CATEGORY GUIDS

static KSDATARANGE CableTopologyAnalogDataRange = {
    sizeof(KSDATARANGE), 0, 0, 0,
    STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
    STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
};
static PKSDATARANGE CableTopologyDataRanges[] = { &CableTopologyAnalogDataRange };

static PCCONNECTION_DESCRIPTOR CableRenderTopologyConnections[] = {
    { PCFILTER_NODE, KSPIN_TOPO_WAVEOUT_SOURCE, PCFILTER_NODE, KSPIN_TOPO_LINEOUT_DEST }
};
static PCCONNECTION_DESCRIPTOR CableCaptureTopologyConnections[] = {
    { PCFILTER_NODE, KSPIN_TOPO_MIC_ELEMENTS, PCFILTER_NODE, KSPIN_TOPO_BRIDGE }
};

// Give every bridge pin a device-specific category. Windows otherwise maps
// all virtual render pins to the fixed "Speakers" label and capture pins to
// the generic category name, losing the cable identity. The INF registers
// each category GUID/name pair in the root device's software key.
#define DEFINE_CABLE_TOPOLOGY_PAIR(letter) \
static PCPIN_DESCRIPTOR Cable##letter##RenderTopologyPins[] = { \
    { 0, 0, 0, NULL, \
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges, \
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } }, \
    { 0, 0, 0, NULL, \
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges, \
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE, &Cable##letter##RenderPinCategory, NULL, 0 } } \
}; \
static PCFILTER_DESCRIPTOR Cable##letter##RenderTopologyFilterDescriptor = { \
    0, NULL, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(Cable##letter##RenderTopologyPins), \
    Cable##letter##RenderTopologyPins, sizeof(PCNODE_DESCRIPTOR), 0, NULL, \
    SIZEOF_ARRAY(CableRenderTopologyConnections), CableRenderTopologyConnections, 0, NULL \
}; \
static PCPIN_DESCRIPTOR Cable##letter##CaptureTopologyPins[] = { \
    { 0, 0, 0, NULL, \
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges, \
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE, &Cable##letter##CapturePinCategory, NULL, 0 } }, \
    { 0, 0, 0, NULL, \
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges, \
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } } \
}; \
static PCFILTER_DESCRIPTOR Cable##letter##CaptureTopologyFilterDescriptor = { \
    0, NULL, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(Cable##letter##CaptureTopologyPins), \
    Cable##letter##CaptureTopologyPins, sizeof(PCNODE_DESCRIPTOR), 0, NULL, \
    SIZEOF_ARRAY(CableCaptureTopologyConnections), CableCaptureTopologyConnections, 0, NULL \
};

#endif // _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_
