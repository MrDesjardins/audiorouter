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

// Exactly the channel masks of CableSupportedFormats (cablewavtable.h): the
// wave pin's IsFormatSupported matches masks exactly, so accepting any other
// layout here would let Windows store a speaker setup the pin then refuses.
static bool IsCableSpeakerConfig(LONG Config)
{
    switch (Config)
    {
    case KSAUDIO_SPEAKER_MONO:
    case KSAUDIO_SPEAKER_STEREO:
    case KSAUDIO_SPEAKER_QUAD:
    case KSAUDIO_SPEAKER_5POINT1:
    case KSAUDIO_SPEAKER_7POINT1_SURROUND:
        return true;
    default:
        return false;
    }
}

// KSPROPERTY_AUDIO_CHANNEL_CONFIG on the render topology's DAC node: the
// documented way Windows' speaker setup (Control Panel, Configure) reads and
// sets an endpoint's speaker configuration. Get reports the current layout
// (stereo by default); Set accepts any layout listed above.
#pragma code_seg("PAGE")
NTSTATUS PropertyHandler_CableDacChannelConfig(_In_ PPCPROPERTY_REQUEST PropertyRequest)
{
    PAGED_CODE();
    ASSERT(PropertyRequest);
    if (!IsEqualGUIDAligned(*PropertyRequest->PropertyItem->Set, KSPROPSETID_Audio) ||
        PropertyRequest->PropertyItem->Id != KSPROPERTY_AUDIO_CHANNEL_CONFIG)
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }
    if (PropertyRequest->Verb & KSPROPERTY_TYPE_BASICSUPPORT)
    {
        return PropertyHandler_BasicSupport(
            PropertyRequest,
            KSPROPERTY_TYPE_BASICSUPPORT | KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_SET,
            VT_ILLEGAL);
    }
    if (PropertyRequest->ValueSize == 0)
    {
        if (PropertyRequest->Verb & KSPROPERTY_TYPE_SET)
        {
            return STATUS_INVALID_PARAMETER;
        }
        PropertyRequest->ValueSize = sizeof(KSAUDIO_CHANNEL_CONFIG);
        return STATUS_BUFFER_OVERFLOW;
    }
    if (PropertyRequest->ValueSize < sizeof(KSAUDIO_CHANNEL_CONFIG) || PropertyRequest->Value == NULL)
    {
        return STATUS_BUFFER_TOO_SMALL;
    }
    PCMiniportTopology miniport = (PCMiniportTopology)PropertyRequest->MajorTarget;
    PKSAUDIO_CHANNEL_CONFIG config = (PKSAUDIO_CHANNEL_CONFIG)PropertyRequest->Value;
    if (PropertyRequest->Verb & KSPROPERTY_TYPE_GET)
    {
        config->ActiveSpeakerPositions = miniport->ChannelConfig();
        PropertyRequest->ValueSize = sizeof(KSAUDIO_CHANNEL_CONFIG);
        return STATUS_SUCCESS;
    }
    if (PropertyRequest->Verb & KSPROPERTY_TYPE_SET)
    {
        if (!IsCableSpeakerConfig(config->ActiveSpeakerPositions))
        {
            return STATUS_NOT_SUPPORTED;
        }
        miniport->SetChannelConfig(config->ActiveSpeakerPositions);
        return STATUS_SUCCESS;
    }
    return STATUS_INVALID_DEVICE_REQUEST;
}

#pragma code_seg()
