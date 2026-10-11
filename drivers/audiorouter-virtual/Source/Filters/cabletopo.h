/*++

Module Name: cabletopo.h

Abstract: Jack description property handlers for the virtual-cable topology
          filters (spec 17 VCAB-11).
--*/

#ifndef _AUDIOROUTERVIRTUAL_CABLETOPO_H_
#define _AUDIOROUTERVIRTUAL_CABLETOPO_H_

NTSTATUS PropertyHandler_CableRenderTopoFilter(_In_ PPCPROPERTY_REQUEST PropertyRequest);

NTSTATUS PropertyHandler_CableCaptureTopoFilter(_In_ PPCPROPERTY_REQUEST PropertyRequest);

NTSTATUS PropertyHandler_CableDacChannelConfig(_In_ PPCPROPERTY_REQUEST PropertyRequest);

#endif // _AUDIOROUTERVIRTUAL_CABLETOPO_H_
