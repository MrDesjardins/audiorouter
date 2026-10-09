/*++

Module Name: cablewavtable.h

Abstract: Shared, format-bounded WaveRT tables for AudioRouter cable pairs.
--*/

#ifndef _AUDIOROUTERVIRTUAL_CABLEWAVTABLE_H_
#define _AUDIOROUTERVIRTUAL_CABLEWAVTABLE_H_

#define CABLE_DEVICE_MAX_CHANNELS 8
#define CABLE_MAX_INPUT_STREAMS 1
#define SPEAKER_DEVICE_MAX_CHANNELS CABLE_DEVICE_MAX_CHANNELS
#define MICARRAY_DEVICE_MAX_CHANNELS CABLE_DEVICE_MAX_CHANNELS

#define AR_WFX_PCM(ch, rate, bits, valid, mask) \
    { \
        { sizeof(KSDATAFORMAT_WAVEFORMATEXTENSIBLE), 0, 0, 0, \
          STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM), \
          STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX) }, \
        { { WAVE_FORMAT_EXTENSIBLE, ch, rate, rate * ch * (bits / 8), \
            ch * (bits / 8), bits, sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) }, \
          valid, mask, STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM) } \
    }
#define AR_WFX_FLOAT(ch, rate, mask) \
    { \
        { sizeof(KSDATAFORMAT_WAVEFORMATEXTENSIBLE), 0, 0, 0, \
          STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT), \
          STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX) }, \
        { { WAVE_FORMAT_EXTENSIBLE, ch, rate, rate * ch * 4, \
            ch * 4, 32, sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX) }, \
          32, mask, STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) } \
    }
#define AR_RATE_FORMATS(ch, mask, rate) \
    AR_WFX_PCM(ch, rate, 16, 16, mask), \
    AR_WFX_PCM(ch, rate, 32, 24, mask), \
    AR_WFX_PCM(ch, rate, 32, 32, mask), \
    AR_WFX_FLOAT(ch, rate, mask)
#define AR_CHANNEL_FORMATS(ch, mask) \
    AR_RATE_FORMATS(ch, mask, 44100), \
    AR_RATE_FORMATS(ch, mask, 48000), \
    AR_RATE_FORMATS(ch, mask, 96000)

// Windows 11 uses side speakers for 7.1. The older 7POINT1 (0xFF) means
// front-of-center speakers and does not match the user-mode surround probe.
C_ASSERT(KSAUDIO_SPEAKER_7POINT1_SURROUND == 0x63F);
static KSDATAFORMAT_WAVEFORMATEXTENSIBLE CableSupportedFormats[] = {
    AR_CHANNEL_FORMATS(1, KSAUDIO_SPEAKER_MONO),
    AR_CHANNEL_FORMATS(2, KSAUDIO_SPEAKER_STEREO),
    AR_CHANNEL_FORMATS(4, KSAUDIO_SPEAKER_QUAD),
    AR_CHANNEL_FORMATS(6, KSAUDIO_SPEAKER_5POINT1),
    AR_CHANNEL_FORMATS(8, KSAUDIO_SPEAKER_7POINT1_SURROUND),
};

#undef AR_CHANNEL_FORMATS
#undef AR_RATE_FORMATS
#undef AR_WFX_FLOAT
#undef AR_WFX_PCM

// Channel count 2 starts after the 12 mono entries. Within each rate's four
// encodings, 48 kHz float is index 12 + 4 + 3 = 19.
static MODE_AND_DEFAULT_FORMAT CableSupportedModes[] = {
    { STATIC_AUDIO_SIGNALPROCESSINGMODE_DEFAULT,
      &CableSupportedFormats[19].DataFormat }
};

static PIN_DEVICE_FORMATS_AND_MODES CableRenderPinFormats[] = {
    { SystemRenderPin, CableSupportedFormats, SIZEOF_ARRAY(CableSupportedFormats),
      CableSupportedModes, SIZEOF_ARRAY(CableSupportedModes) },
    { BridgePin, NULL, 0, NULL, 0 }
};
static PIN_DEVICE_FORMATS_AND_MODES CableCapturePinFormats[] = {
    { BridgePin, NULL, 0, NULL, 0 },
    { SystemCapturePin, CableSupportedFormats, SIZEOF_ARRAY(CableSupportedFormats),
      CableSupportedModes, SIZEOF_ARRAY(CableSupportedModes) }
};

#define AR_PCM_RANGE(channels) \
    { { sizeof(KSDATARANGE_AUDIO), KSDATARANGE_ATTRIBUTES, 0, 0, \
        STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_PCM), \
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX) }, channels, 16, 32, 44100, 96000 }
#define AR_FLOAT_RANGE(channels) \
    { { sizeof(KSDATARANGE_AUDIO), KSDATARANGE_ATTRIBUTES, 0, 0, \
        STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT), \
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_WAVEFORMATEX) }, channels, 32, 32, 44100, 96000 }
static KSDATARANGE_AUDIO CablePcmDataRanges[] = {
    AR_PCM_RANGE(1), AR_PCM_RANGE(2), AR_PCM_RANGE(4), AR_PCM_RANGE(6), AR_PCM_RANGE(8)
};
static KSDATARANGE_AUDIO CableFloatDataRanges[] = {
    AR_FLOAT_RANGE(1), AR_FLOAT_RANGE(2), AR_FLOAT_RANGE(4), AR_FLOAT_RANGE(6), AR_FLOAT_RANGE(8)
};
#undef AR_PCM_RANGE
#undef AR_FLOAT_RANGE
#define AR_RANGE_WITH_ATTRIBUTES(range) \
    PKSDATARANGE(range), PKSDATARANGE(&PinDataRangeAttributeList)
// KS expects the attribute-list pointer immediately after every range whose
// KSDATARANGE_ATTRIBUTES flag is set. Keep these pairs adjacent in the pointer
// table; placing one attribute list at the end makes KS walk a format range as
// an attribute list and can bugcheck during endpoint enumeration.
static PKSDATARANGE CableStreamDataRanges[] = {
    AR_RANGE_WITH_ATTRIBUTES(&CablePcmDataRanges[0]),
    AR_RANGE_WITH_ATTRIBUTES(&CablePcmDataRanges[1]),
    AR_RANGE_WITH_ATTRIBUTES(&CablePcmDataRanges[2]),
    AR_RANGE_WITH_ATTRIBUTES(&CablePcmDataRanges[3]),
    AR_RANGE_WITH_ATTRIBUTES(&CablePcmDataRanges[4]),
    AR_RANGE_WITH_ATTRIBUTES(&CableFloatDataRanges[0]),
    AR_RANGE_WITH_ATTRIBUTES(&CableFloatDataRanges[1]),
    AR_RANGE_WITH_ATTRIBUTES(&CableFloatDataRanges[2]),
    AR_RANGE_WITH_ATTRIBUTES(&CableFloatDataRanges[3]),
    AR_RANGE_WITH_ATTRIBUTES(&CableFloatDataRanges[4]),
};
#undef AR_RANGE_WITH_ATTRIBUTES
C_ASSERT(SIZEOF_ARRAY(CableStreamDataRanges) == 20);
static KSDATARANGE CableBridgeDataRange = {
    sizeof(KSDATARANGE), 0, 0, 0,
    STATICGUIDOF(KSDATAFORMAT_TYPE_AUDIO), STATICGUIDOF(KSDATAFORMAT_SUBTYPE_ANALOG),
    STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
};
static PKSDATARANGE CableBridgeDataRanges[] = { &CableBridgeDataRange };

static PCPIN_DESCRIPTOR CableRenderPins[] = {
    { CABLE_MAX_INPUT_STREAMS, CABLE_MAX_INPUT_STREAMS, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableStreamDataRanges), CableStreamDataRanges,
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_SINK, &KSCATEGORY_AUDIO, NULL, 0 } },
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableBridgeDataRanges), CableBridgeDataRanges,
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } }
};
static PCPIN_DESCRIPTOR CableCapturePins[] = {
    { 0, 0, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableBridgeDataRanges), CableBridgeDataRanges,
        KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE, &KSCATEGORY_AUDIO, NULL, 0 } },
    { CABLE_MAX_INPUT_STREAMS, CABLE_MAX_INPUT_STREAMS, 0, NULL,
      { 0, NULL, 0, NULL, SIZEOF_ARRAY(CableStreamDataRanges), CableStreamDataRanges,
        KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_SINK, &KSCATEGORY_AUDIO,
        &KSAUDFNAME_RECORDING_CONTROL, 0 } }
};
static PCCONNECTION_DESCRIPTOR CableRenderConnections[] = {
    { PCFILTER_NODE, KSPIN_WAVE_RENDER3_SINK_SYSTEM, PCFILTER_NODE, KSPIN_WAVE_RENDER3_SOURCE }
};
static PCCONNECTION_DESCRIPTOR CableCaptureConnections[] = {
    { PCFILTER_NODE, KSPIN_WAVE_BRIDGE, PCFILTER_NODE, KSPIN_WAVEIN_HOST }
};
static PCPROPERTY_ITEM CableWaveProperties[] = {
    { &KSPROPSETID_Pin, KSPROPERTY_PIN_PROPOSEDATAFORMAT,
      KSPROPERTY_TYPE_SET | KSPROPERTY_TYPE_BASICSUPPORT, PropertyHandler_WaveFilter },
    { &KSPROPSETID_Pin, KSPROPERTY_PIN_PROPOSEDATAFORMAT2,
      KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT, PropertyHandler_WaveFilter }
};
DEFINE_PCAUTOMATION_TABLE_PROP(AutomationCableWaveFilter, CableWaveProperties);

static PCFILTER_DESCRIPTOR CableRenderWaveFilterDescriptor = {
    0, &AutomationCableWaveFilter, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(CableRenderPins),
    CableRenderPins, sizeof(PCNODE_DESCRIPTOR), 0, NULL,
    SIZEOF_ARRAY(CableRenderConnections), CableRenderConnections, 0, NULL
};
static PCFILTER_DESCRIPTOR CableCaptureWaveFilterDescriptor = {
    0, &AutomationCableWaveFilter, sizeof(PCPIN_DESCRIPTOR), SIZEOF_ARRAY(CableCapturePins),
    CableCapturePins, sizeof(PCNODE_DESCRIPTOR), 0, NULL,
    SIZEOF_ARRAY(CableCaptureConnections), CableCaptureConnections, 0, NULL
};

#endif // _AUDIOROUTERVIRTUAL_CABLEWAVTABLE_H_
