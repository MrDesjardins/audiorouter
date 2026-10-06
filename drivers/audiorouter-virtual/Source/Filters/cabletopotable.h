/*++

Module Name: cabletopotable.h

Abstract: Minimal direct-path topology tables for virtual cables.
--*/

#ifndef _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_
#define _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_

static KSDATARANGE CableTopologyAnalogDataRange = {
    sizeof(KSDATARANGE), 0, 0, 0,
    STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
    STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
};
static PKSDATARANGE CableTopologyDataRanges[] = { &CableTopologyAnalogDataRange };

// Render: system-facing speaker pin -> bridge pin. No volume/mute nodes or
// jack-detection properties are inserted into the samples.
static PCPIN_DESCRIPTOR CableRenderTopologyPins[] = {
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges,
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } },
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges,
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE, &KSNODETYPE_SPEAKER, NULL, 0 } }
};
static PCCONNECTION_DESCRIPTOR CableRenderTopologyConnections[] = {
    { PCFILTER_NODE, KSPIN_TOPO_WAVEOUT_SOURCE, PCFILTER_NODE, KSPIN_TOPO_LINEOUT_DEST }
};
static PCFILTER_DESCRIPTOR CableRenderTopologyFilterDescriptor = {
    0, NULL, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(CableRenderTopologyPins),
    CableRenderTopologyPins, sizeof(PCNODE_DESCRIPTOR), 0, NULL,
    SIZEOF_ARRAY(CableRenderTopologyConnections), CableRenderTopologyConnections, 0, NULL
};

// Capture: generic line-in pin -> bridge pin. It does not advertise or answer
// microphone-array geometry, sensitivity, or microphone jack properties.
static PCPIN_DESCRIPTOR CableCaptureTopologyPins[] = {
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges,
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE, &KSNODETYPE_LINE_CONNECTOR, NULL, 0 } },
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableTopologyDataRanges), CableTopologyDataRanges,
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } }
};
static PCCONNECTION_DESCRIPTOR CableCaptureTopologyConnections[] = {
    { PCFILTER_NODE, KSPIN_TOPO_MIC_ELEMENTS, PCFILTER_NODE, KSPIN_TOPO_BRIDGE }
};
static PCFILTER_DESCRIPTOR CableCaptureTopologyFilterDescriptor = {
    0, NULL, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(CableCaptureTopologyPins),
    CableCaptureTopologyPins, sizeof(PCNODE_DESCRIPTOR), 0, NULL,
    SIZEOF_ARRAY(CableCaptureTopologyConnections), CableCaptureTopologyConnections, 0, NULL
};

#endif // _AUDIOROUTERVIRTUAL_CABLETOPOTABLE_H_
