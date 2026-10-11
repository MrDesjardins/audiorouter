
/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minitopo.h

Abstract:

    Declaration of topology miniport.
--*/

#ifndef _AUDIOROUTERVIRTUAL_MINTOPO_H_
#define _AUDIOROUTERVIRTUAL_MINTOPO_H_

#include "basetopo.h"

//=============================================================================
// Classes
//=============================================================================

///////////////////////////////////////////////////////////////////////////////
// CMiniportTopology
//

class CMiniportTopology :
    public CMiniportTopologyAudioRouterVirtual,
    public IMiniportTopology,
    public CUnknown
{
  private:
    eDeviceType             m_DeviceType;

    union {
        PVOID               m_DeviceContext;
    };
    LONG                    m_ChannelConfig;

public:
    DECLARE_STD_UNKNOWN();
    CMiniportTopology
    (
        _In_opt_    PUNKNOWN                UnknownOuter,
        _In_        PCFILTER_DESCRIPTOR    *FilterDesc,
        _In_        USHORT                  DeviceMaxChannels,
        _In_        eDeviceType             DeviceType,
        _In_opt_    PVOID                   DeviceContext
    )
    : CUnknown(UnknownOuter),
      CMiniportTopologyAudioRouterVirtual(FilterDesc, DeviceMaxChannels),
      m_DeviceType(DeviceType),
      m_DeviceContext(DeviceContext),
      m_ChannelConfig(KSAUDIO_SPEAKER_STEREO)
    {
    }

    // Speaker configuration set through a cable topology's DAC node
    // (KSPROPERTY_AUDIO_CHANNEL_CONFIG, Control Panel speaker setup). Stereo
    // until Windows sets another; the cable carries whatever channel count
    // each stream opens with, so this is only reported back.
    LONG ChannelConfig() const { return m_ChannelConfig; }
    void SetChannelConfig(LONG Config) { m_ChannelConfig = Config; }

    ~CMiniportTopology();

    IMP_IMiniportTopology;

    NTSTATUS PropertyHandlerJackDescription
    (
        _In_        PPCPROPERTY_REQUEST                         PropertyRequest,
        _In_        ULONG                                       cJackDescriptions,
        _In_reads_(cJackDescriptions) PKSJACK_DESCRIPTION       *JackDescriptions
    );

    NTSTATUS PropertyHandlerJackDescription2
    (
        _In_        PPCPROPERTY_REQUEST                         PropertyRequest,
        _In_        ULONG                                       cJackDescriptions,
        _In_reads_(cJackDescriptions) PKSJACK_DESCRIPTION       *JackDescriptions,
        _In_        DWORD                                       JackCapabilities
    );

    PVOID GetDeviceContext() { return m_DeviceContext;  }
};

typedef CMiniportTopology *PCMiniportTopology;

#endif // _AUDIOROUTERVIRTUAL_MINTOPO_H_
