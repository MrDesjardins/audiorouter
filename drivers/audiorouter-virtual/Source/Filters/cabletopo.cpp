/*++

Module Name: cabletopo.cpp

Abstract: Jack descriptions of the virtual-cable topology filters.

  Windows takes an endpoint's physical speaker layout from the jack
  description of its bridge pin (ChannelMapping). Without one it treats the
  cable as stereo and offers only 1- and 2-channel formats in Sound settings,
  although the wave pins accept 1, 2, 4, 6 and 8 channels (VCAB-11). Each
  cable reports one jack with the 7.1 surround mask (0x63F, side speakers),
  the widest layout its data ranges carry. The default format stays the wave
  pin's 48 kHz float stereo mode entry (cablewavtable.h).
--*/

#pragma warning (disable : 4127)

#include "definitions.h"
#include "endpoints.h"
#include "mintopo.h"
#include "cabletopo.h"

static KSJACK_DESCRIPTION CableJackDescription =
{
    KSAUDIO_SPEAKER_7POINT1_SURROUND,   // widest layout the wave pins accept
    0,                                  // no colour: not a physical jack
    eConnTypeUnknown,
    eGeoLocNotApplicable,
    eGenLocPrimaryBox,
    ePortConnIntegratedDevice,
    TRUE                                // always connected
};

// Render topology: pin 0 is the wave-facing input, pin 1 the bridge
// (analog connector) the endpoint represents. Capture topology: pin 0 is the
// bridge (cable category), pin 1 the wave-facing output.
static PKSJACK_DESCRIPTION CableRenderJackDescriptions[] = { NULL, &CableJackDescription };
static PKSJACK_DESCRIPTION CableCaptureJackDescriptions[] = { &CableJackDescription, NULL };

#pragma code_seg("PAGE")
static NTSTATUS CableJackProperty(
    _In_ PPCPROPERTY_REQUEST PropertyRequest,
    _In_ ULONG Count,
    _In_reads_(Count) PKSJACK_DESCRIPTION* Descriptions)
{
    PAGED_CODE();
    ASSERT(PropertyRequest);
    PCMiniportTopology miniport = (PCMiniportTopology)PropertyRequest->MajorTarget;
    if (!IsEqualGUIDAligned(*PropertyRequest->PropertyItem->Set, KSPROPSETID_Jack))
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }
    if (PropertyRequest->PropertyItem->Id == KSPROPERTY_JACK_DESCRIPTION)
    {
        return miniport->PropertyHandlerJackDescription(PropertyRequest, Count, Descriptions);
    }
    if (PropertyRequest->PropertyItem->Id == KSPROPERTY_JACK_DESCRIPTION2)
    {
        return miniport->PropertyHandlerJackDescription2(PropertyRequest, Count, Descriptions, 0);
    }
    return STATUS_INVALID_DEVICE_REQUEST;
}

#pragma code_seg("PAGE")
NTSTATUS PropertyHandler_CableRenderTopoFilter(_In_ PPCPROPERTY_REQUEST PropertyRequest)
{
    PAGED_CODE();
    return CableJackProperty(PropertyRequest, ARRAYSIZE(CableRenderJackDescriptions),
                             CableRenderJackDescriptions);
}

#pragma code_seg("PAGE")
NTSTATUS PropertyHandler_CableCaptureTopoFilter(_In_ PPCPROPERTY_REQUEST PropertyRequest)
{
    PAGED_CODE();
    return CableJackProperty(PropertyRequest, ARRAYSIZE(CableCaptureJackDescriptions),
                             CableCaptureJackDescriptions);
}

#pragma code_seg()
