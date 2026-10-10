#include "definitions.h"
#include <limits.h>
#include <ks.h>
#include "endpoints.h"
#include "minwavert.h"
#include "minwavertstream.h"
#include "sampleconv.h"
#define MINWAVERTSTREAM_POOLTAG 'SRWM'

#pragma warning (disable : 4127)

// Convert negotiated endpoint samples to/from the double-precision bridge.
// PCM24 is left-aligned in its 32-bit container; PCM32 remains exact because
// every signed 32-bit integer and its 2^-31 scale are exactly representable.
static __forceinline DOUBLE ReadBridgeSample(
    _In_reads_bytes_(sizeof(LONG)) const UCHAR* Source,
    _In_ const WAVEFORMATEXTENSIBLE* Format)
{
    if (Source == NULL || Format == NULL) {
        return 0.0;
    }
    if (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) &&
        Format->Format.wBitsPerSample == 32) {
        FLOAT value = 0.0F;
        RtlCopyMemory(&value, Source, sizeof(value));
        return AudioRouterFloat32ToDouble(value);
    }
    if (!IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM)) {
        return 0.0;
    }
    if (Format->Format.wBitsPerSample == 16 && Format->Samples.wValidBitsPerSample == 16) {
        SHORT value = 0;
        RtlCopyMemory(&value, Source, sizeof(value));
        return AudioRouterPcm16ToDouble(value);
    }
    if (Format->Format.wBitsPerSample == 32 &&
        Format->Samples.wValidBitsPerSample == 24) {
        LONG value = 0;
        RtlCopyMemory(&value, Source, sizeof(value));
        return AudioRouterPcm24In32ToDouble(value);
    }
    if (Format->Format.wBitsPerSample == 32 &&
        Format->Samples.wValidBitsPerSample == 32) {
        LONG value = 0;
        RtlCopyMemory(&value, Source, sizeof(value));
        return AudioRouterPcm32ToDouble(value);
    }
    return 0.0;
}

static __forceinline void WriteBridgeSample(
    _Out_writes_bytes_(sizeof(LONG)) UCHAR* Destination,
    _In_ const WAVEFORMATEXTENSIBLE* Format,
    _In_ DOUBLE Sample)
{
    if (Destination == NULL || Format == NULL) {
        return;
    }
    if (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) &&
        Format->Format.wBitsPerSample == 32) {
        FLOAT value = AudioRouterDoubleToFloat32(Sample);
        RtlCopyMemory(Destination, &value, sizeof(value));
    } else if (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM) &&
               Format->Format.wBitsPerSample == 16 &&
               Format->Samples.wValidBitsPerSample == 16) {
        SHORT value = AudioRouterDoubleToPcm16(Sample);
        RtlCopyMemory(Destination, &value, sizeof(value));
    } else if (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM) &&
               Format->Format.wBitsPerSample == 32 &&
               Format->Samples.wValidBitsPerSample == 24) {
        LONG value = AudioRouterDoubleToPcm24In32(Sample);
        RtlCopyMemory(Destination, &value, sizeof(value));
    } else if (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM) &&
               Format->Format.wBitsPerSample == 32 &&
               Format->Samples.wValidBitsPerSample == 32) {
        LONG value = AudioRouterDoubleToPcm32(Sample);
        RtlCopyMemory(Destination, &value, sizeof(value));
    }
}

static __forceinline BOOLEAN IsBridgePcmFormat(
    _In_opt_ const WAVEFORMATEXTENSIBLE* Format)
{
    return Format != NULL &&
        (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM) ||
         IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT)) &&
        Format->Format.nChannels != 0 &&
        Format->Format.nChannels <= AR_BRIDGE_MAX_CHANNELS &&
        ((IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) &&
          Format->Format.wBitsPerSample == 32) ||
         (IsEqualGUIDAligned(Format->SubFormat, KSDATAFORMAT_SUBTYPE_PCM) &&
          ((Format->Format.wBitsPerSample == 16 && Format->Samples.wValidBitsPerSample == 16) ||
           (Format->Format.wBitsPerSample == 32 &&
            (Format->Samples.wValidBitsPerSample == 24 || Format->Samples.wValidBitsPerSample == 32))))) &&
        Format->Format.nBlockAlign ==
            Format->Format.nChannels * (Format->Format.wBitsPerSample / 8);
}

//=============================================================================
// CMiniportWaveRTStream
//=============================================================================

//=============================================================================
#pragma code_seg("PAGE")
CMiniportWaveRTStream::~CMiniportWaveRTStream
(
    void
)
/*++

Routine Description:

  Destructor for wavertstream

Arguments:

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();
    // Timer callbacks own references to the format, DMA and miniport below.
    // Join them before releasing any of those dependencies, including a tail
    // drain still pending after playback has paused or reached EoS.
    if (m_pNotificationTimer)
    {
        ExDeleteTimer(m_pNotificationTimer, TRUE, TRUE, NULL);
        m_pNotificationTimer = NULL;
    }
    if (NULL != m_pMiniport)
    {

        if (m_bUnregisterStream)
        {
            m_pMiniport->StreamClosed(m_ulPin, this);
            m_bUnregisterStream = FALSE;
        }

        m_pMiniport->Release();
        m_pMiniport = NULL;
    }

    if (m_pDpc)
    {
        ExFreePoolWithTag( m_pDpc, MINWAVERTSTREAM_POOLTAG );
        m_pDpc = NULL;
    }

    if (m_pTimer)
    {
        ExFreePoolWithTag( m_pTimer, MINWAVERTSTREAM_POOLTAG );
        m_pTimer = NULL;
    }

    if (m_pbMuted)
    {
        ExFreePoolWithTag( m_pbMuted, MINWAVERTSTREAM_POOLTAG );
        m_pbMuted = NULL;
    }

    if (m_plVolumeLevel)
    {
        ExFreePoolWithTag( m_plVolumeLevel, MINWAVERTSTREAM_POOLTAG );
        m_plVolumeLevel = NULL;
    }

    if (m_plPeakMeter)
    {
        ExFreePoolWithTag( m_plPeakMeter, MINWAVERTSTREAM_POOLTAG );
        m_plPeakMeter = NULL;
    }

    if (m_pWfExt)
    {
        ExFreePoolWithTag( m_pWfExt, MINWAVERTSTREAM_POOLTAG );
        m_pWfExt = NULL;
    }

    // Since we just cancelled the notification timer, wait for all queued
    // DPCs to complete before we free the notification DPC.
    //
    KeFlushQueuedDpcs();

    DPF_ENTER(("[CMiniportWaveRTStream::~CMiniportWaveRTStream]"));
} // ~CMiniportWaveRTStream

//=============================================================================
#pragma code_seg("PAGE")

NTSTATUS
CMiniportWaveRTStream::Init
(
    _In_ PCMiniportWaveRT           Miniport_,
    _In_ PPORTWAVERTSTREAM          PortStream_,
    _In_ ULONG                      Pin_,
    _In_ BOOLEAN                    Capture_,
    _In_ PKSDATAFORMAT              DataFormat_,
    _In_ GUID                       SignalProcessingMode
)
/*++

Routine Description:

  Initializes the stream object.

Arguments:

  Miniport_ -

  Pin_ -

  DataFormat -

  SignalProcessingMode - The driver uses the signalProcessingMode to configure
    driver and/or hardware specific signal processing to be applied to this new
    stream.

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();

    PWAVEFORMATEX pWfEx = NULL;
    NTSTATUS ntStatus = STATUS_SUCCESS;

    m_pMiniport = NULL;
    m_ulPin = 0;
    m_bUnregisterStream = FALSE;
    m_bCapture = FALSE;
    m_ulDmaBufferSize = 0;
    m_pDmaBuffer = NULL;
    m_ulNotificationsPerBuffer = 0;
    m_KsState = KSSTATE_STOP;
    m_pTimer = NULL;
    m_pNotificationTimer = NULL;
    m_pDpc = NULL;
    m_llPacketCounter = 0;
    m_ullPlayPosition = 0;
    m_ullWritePosition = 0;
    m_ullDmaTimeStamp = 0;
    m_hnsElapsedTimeCarryForward = 0;
    m_llLastNotifiedPacketCounter = 0;
    m_ulDmaMovementRate = 0;
    m_byteDisplacementCarryForward = 0;
    m_bLfxEnabled = FALSE;
    m_pbMuted = NULL;
    m_plVolumeLevel = NULL;
    m_plPeakMeter = NULL;
    m_pWfExt = NULL;
    m_ullLinearPosition = 0;
    m_ullPresentationPosition = 0;
    m_ulContentId = 0;
    m_ulCurrentWritePosition = 0;
    m_ulLastOsReadPacket = ULONG_MAX;
    m_ulLastOsWritePacket = ULONG_MAX;
    m_RenderCommits.Reset();
    m_IsCurrentWritePositionUpdated = 0;
    m_SignalProcessingMode = SignalProcessingMode;
    m_bEoSReceived = FALSE;
    m_bLastBufferRendered = FALSE;
    m_BridgeScratchFrames = 0;
    m_BridgeScratchFrameOffset = 0;
    m_CaptureQueue.Reset();
    m_BridgeGeneration = 0;
    m_BridgeReadGeneration = 0;
    m_BridgePublishFrames = 0;
    m_BridgePublishChannels = 0;
    m_RenderQueue.Reset(0, 0, 0);
    m_bEosCompletionNotified = FALSE;

    m_pPortStream = PortStream_;
    InitializeListHead(&m_NotificationList);
    m_hnsNotificationInterval = 0;

    // Initialize the spinlock to synchronize position updates
    KeInitializeSpinLock(&m_PositionSpinLock);

    m_pNotificationTimer = ExAllocateTimer(
         TimerNotifyRT,
         this,
         EX_TIMER_HIGH_RESOLUTION
    );
    if (!m_pNotificationTimer)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    pWfEx = GetWaveFormatEx(DataFormat_);
    if (NULL == pWfEx)
    {
        return STATUS_UNSUCCESSFUL;
    }

    m_pMiniport = reinterpret_cast<CMiniportWaveRT*>(Miniport_);
    if (m_pMiniport == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }
    m_pMiniport->AddRef();
    if (!NT_SUCCESS(ntStatus))
    {
        return ntStatus;
    }
    m_ulPin = Pin_;
    m_bCapture = Capture_;
    m_ulDmaMovementRate = pWfEx->nAvgBytesPerSec;

    m_pDpc = (PRKDPC)ExAllocatePool2(POOL_FLAG_NON_PAGED, sizeof(KDPC), MINWAVERTSTREAM_POOLTAG);
    if (!m_pDpc)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_pWfExt = (PWAVEFORMATEXTENSIBLE)ExAllocatePool2(POOL_FLAG_NON_PAGED, sizeof(WAVEFORMATEX) + pWfEx->cbSize, MINWAVERTSTREAM_POOLTAG);
    if (m_pWfExt == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    RtlCopyMemory(m_pWfExt, pWfEx, sizeof(WAVEFORMATEX) + pWfEx->cbSize);

    m_pbMuted = (PBOOL)ExAllocatePool2(POOL_FLAG_NON_PAGED, m_pWfExt->Format.nChannels * sizeof(BOOL), MINWAVERTSTREAM_POOLTAG);
    if (m_pbMuted == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_plVolumeLevel = (PLONG)ExAllocatePool2(POOL_FLAG_NON_PAGED, m_pWfExt->Format.nChannels * sizeof(LONG), MINWAVERTSTREAM_POOLTAG);
    if (m_plVolumeLevel == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_plPeakMeter = (PLONG)ExAllocatePool2(POOL_FLAG_NON_PAGED, m_pWfExt->Format.nChannels * sizeof(LONG), MINWAVERTSTREAM_POOLTAG);
    if (m_plPeakMeter == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    // Register this stream.
    //
    ntStatus = m_pMiniport->StreamCreated(m_ulPin, this);
    if (NT_SUCCESS(ntStatus))
    {
        m_bUnregisterStream = TRUE;
    }

    return ntStatus;
} // Init

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportWaveRTStream::NonDelegatingQueryInterface
(
    _In_ REFIID  Interface,
    _COM_Outptr_ PVOID * Object
)
/*++

Routine Description:

  QueryInterface

Arguments:

  Interface - GUID

  Object - interface pointer to be returned

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();

    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface, IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(PMINIPORTWAVERTSTREAM(this)));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTStream))
    {
        *Object = PVOID(PMINIPORTWAVERTSTREAM(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTStreamNotification))
    {
        *Object = PVOID(PMINIPORTWAVERTSTREAMNOTIFICATION(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTInputStream) && (this->m_bCapture))
    {
        // This interface is supported only on capture streams
        *Object = PVOID(PMINIPORTWAVERTINPUTSTREAM(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IMiniportWaveRTOutputStream) && (!this->m_bCapture))
    {
        // This interface is supported only on host render streams
        *Object = PVOID(PMINIPORTWAVERTOUTPUTSTREAM(this));
    }
    else if (IsEqualGUIDAligned(Interface, IID_IDrmAudioStream))
    {
        *Object = (PVOID)(IDrmAudioStream*)this;
    }
    else
    {
        *Object = NULL;
    }

    if (*Object)
    {
        PUNKNOWN(*Object)->AddRef();
        return STATUS_SUCCESS;
    }

    return STATUS_INVALID_PARAMETER;
} // NonDelegatingQueryInterface

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::AllocateBufferWithNotification
(
    _In_    ULONG               NotificationCount_,
    _In_    ULONG               RequestedSize_,
    _Out_   PMDL                *AudioBufferMdl_,
    _Out_   ULONG               *ActualSize_,
    _Out_   ULONG               *OffsetFromFirstPage_,
    _Out_   MEMORY_CACHING_TYPE *CacheType_
)
{
    PAGED_CODE();

    if (AudioBufferMdl_ == NULL || ActualSize_ == NULL ||
        OffsetFromFirstPage_ == NULL || CacheType_ == NULL ||
        m_pPortStream == NULL || m_pWfExt == NULL ||
        m_pWfExt->Format.nBlockAlign == 0 ||
        m_ulDmaMovementRate == 0 ||
        (0 == RequestedSize_) || (RequestedSize_ < m_pWfExt->Format.nBlockAlign))
    {
        return STATUS_UNSUCCESSFUL;
    }

    if (NotificationCount_ != 1 && NotificationCount_ != 2)
    {
        return STATUS_INVALID_PARAMETER;
    }

    // Every notification packet must contain whole frames, including when
    // the requested cyclic buffer size needs rounding down.
    const ULONG packetAlignment = m_pWfExt->Format.nBlockAlign * NotificationCount_;
    RequestedSize_ -= RequestedSize_ % packetAlignment;
    if (RequestedSize_ == 0) { return STATUS_INVALID_PARAMETER; }
    m_RenderCommits.Reset();

    PHYSICAL_ADDRESS highAddress;
    highAddress.HighPart = 0;
    highAddress.LowPart = MAXULONG;

    PMDL pBufferMdl = m_pPortStream->AllocatePagesForMdl (highAddress, RequestedSize_);

    if (NULL == pBufferMdl)
    {
        return STATUS_UNSUCCESSFUL;
    }

    // From MSDN:
    // "Since the Windows audio stack does not support a mechanism to express memory access
    //  alignment requirements for buffers, audio drivers must select a caching type for mapped
    //  memory buffers that does not impose platform-specific alignment requirements. In other
    //  words, the caching type used by the audio driver for mapped memory buffers, must not make
    //  assumptions about the memory alignment requirements for any specific platform.
    //
    //  This method maps the physical memory pages in the MDL into kernel-mode virtual memory.
    //  Typically, the miniport driver calls this method if it requires software access to the
    //  scatter-gather list for an audio buffer. In this case, the storage for the scatter-gather
    //  list must have been allocated by the IPortWaveRTStream::AllocatePagesForMdl or
    //  IPortWaveRTStream::AllocateContiguousPagesForMdl method.
    //
    //  A WaveRT miniport driver should not require software access to the audio buffer itself."
    //
    m_pDmaBuffer = (BYTE*)m_pPortStream->MapAllocatedPages(pBufferMdl, MmCached);
    if (m_pDmaBuffer == NULL)
    {
        m_pPortStream->FreePagesFromMdl(pBufferMdl);
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    m_ulNotificationsPerBuffer = NotificationCount_;
    m_ulDmaBufferSize = RequestedSize_;
    ULONGLONG notificationIntervalHns =
        (static_cast<ULONGLONG>(RequestedSize_) * 10000000) /
        m_ulDmaMovementRate / NotificationCount_;
    if (notificationIntervalHns == 0)
    {
        m_pPortStream->UnmapAllocatedPages(m_pDmaBuffer, pBufferMdl);
        m_pDmaBuffer = NULL;
        m_pPortStream->FreePagesFromMdl(pBufferMdl);
        return STATUS_INVALID_PARAMETER;
    }
    m_hnsNotificationInterval = notificationIntervalHns;

    *AudioBufferMdl_ = pBufferMdl;
    *ActualSize_ = RequestedSize_;
    *OffsetFromFirstPage_ = 0;
    *CacheType_ = MmCached;

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::FreeBufferWithNotification
(
    _In_        PMDL    Mdl_,
    _In_        ULONG   Size_
)
{
    UNREFERENCED_PARAMETER(Size_);

    PAGED_CODE();

    // PortCls calls buffer release after stopping the pin. Join any tail
    // callback before unmapping DMA, just as destruction joins before freeing
    // the stream's format/miniport dependencies.
    if (m_pNotificationTimer != NULL) {
        ExCancelTimer(m_pNotificationTimer, NULL);
        KeFlushQueuedDpcs();
    }

    if (Mdl_ != NULL)
    {
        if (m_pDmaBuffer != NULL && m_pPortStream != NULL)
        {
            m_pPortStream->UnmapAllocatedPages(m_pDmaBuffer, Mdl_);
            m_pDmaBuffer = NULL;
        }

        if (m_pPortStream != NULL)
        {
            m_pPortStream->FreePagesFromMdl(Mdl_);
        }
    }

    m_ulDmaBufferSize = 0;
    m_ulNotificationsPerBuffer = 0;
    m_RenderCommits.Reset();

    return;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::RegisterNotificationEvent
(
    _In_ PKEVENT NotificationEvent_
)
{
    UNREFERENCED_PARAMETER(NotificationEvent_);

    PAGED_CODE();

    if (NotificationEvent_ == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    NotificationListEntry *nleNew = (NotificationListEntry*)ExAllocatePool2(
        POOL_FLAG_NON_PAGED,
        sizeof(NotificationListEntry),
        MINWAVERTSTREAM_POOLTAG);
    if (NULL == nleNew)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    nleNew->NotificationEvent = NotificationEvent_;

    // Fail if the notification event already exists in our list.
    if (!IsListEmpty(&m_NotificationList))
    {
        PLIST_ENTRY leCurrent = m_NotificationList.Flink;
        while (leCurrent != &m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            if (nleCurrent->NotificationEvent == NotificationEvent_)
            {
                ExFreePoolWithTag( nleNew, MINWAVERTSTREAM_POOLTAG );
                return STATUS_UNSUCCESSFUL;
            }

            leCurrent = leCurrent->Flink;
        }
    }

    InsertTailList(&m_NotificationList, &(nleNew->ListEntry));

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::UnregisterNotificationEvent
(
    _In_ PKEVENT NotificationEvent_
)
{
    UNREFERENCED_PARAMETER(NotificationEvent_);

    PAGED_CODE();

    if (!IsListEmpty(&m_NotificationList))
    {
        PLIST_ENTRY leCurrent = m_NotificationList.Flink;
        while (leCurrent != &m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            if (nleCurrent->NotificationEvent == NotificationEvent_)
            {
                RemoveEntryList( leCurrent );
                ExFreePoolWithTag( nleCurrent, MINWAVERTSTREAM_POOLTAG );
                return STATUS_SUCCESS;
            }

            leCurrent = leCurrent->Flink;
        }
    }

    return STATUS_NOT_FOUND;
}


//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::GetClockRegister
(
    _Out_ PKSRTAUDIO_HWREGISTER Register_
)
{
    UNREFERENCED_PARAMETER(Register_);

    PAGED_CODE();

    return STATUS_NOT_IMPLEMENTED;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::GetPositionRegister
(
    _Out_ PKSRTAUDIO_HWREGISTER Register_
)
{
    UNREFERENCED_PARAMETER(Register_);

    PAGED_CODE();

    return STATUS_NOT_IMPLEMENTED;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::GetHWLatency
(
    _Out_ PKSRTAUDIO_HWLATENCY  Latency_
)
{
    PAGED_CODE();

    if (Latency_ == NULL)
    {
        return;
    }

    Latency_->ChipsetDelay = 0;
    Latency_->CodecDelay = 0;
    Latency_->FifoSize = 0;
}

//=============================================================================
#pragma code_seg("PAGE")
VOID CMiniportWaveRTStream::FreeAudioBuffer
(
_In_opt_    PMDL        Mdl_,
_In_        ULONG       Size_
)
{
    UNREFERENCED_PARAMETER(Size_);

    PAGED_CODE();

    if (m_pNotificationTimer != NULL) {
        ExCancelTimer(m_pNotificationTimer, NULL);
        KeFlushQueuedDpcs();
    }

    if (Mdl_ != NULL)
    {
        if (m_pDmaBuffer != NULL && m_pPortStream != NULL)
        {
            m_pPortStream->UnmapAllocatedPages(m_pDmaBuffer, Mdl_);
            m_pDmaBuffer = NULL;
        }

        if (m_pPortStream != NULL)
        {
            m_pPortStream->FreePagesFromMdl(Mdl_);
        }
    }

    m_ulDmaBufferSize = 0;
    m_ulNotificationsPerBuffer = 0;
    m_RenderCommits.Reset();
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::AllocateAudioBuffer
(
_In_    ULONG                   RequestedSize_,
_Out_   PMDL                   *AudioBufferMdl_,
_Out_   ULONG                  *ActualSize_,
_Out_   ULONG                  *OffsetFromFirstPage_,
_Out_   MEMORY_CACHING_TYPE    *CacheType_
)
{
    PAGED_CODE();

    if (AudioBufferMdl_ == NULL || ActualSize_ == NULL ||
        OffsetFromFirstPage_ == NULL || CacheType_ == NULL ||
        m_pPortStream == NULL || m_pWfExt == NULL ||
        m_pWfExt->Format.nBlockAlign == 0 ||
        (0 == RequestedSize_) || (RequestedSize_ < m_pWfExt->Format.nBlockAlign))
    {
        return STATUS_UNSUCCESSFUL;
    }

    RequestedSize_ -= RequestedSize_ % (m_pWfExt->Format.nBlockAlign);

    PHYSICAL_ADDRESS highAddress;
    highAddress.HighPart = 0;
    highAddress.LowPart = MAXULONG;

    PMDL pBufferMdl = m_pPortStream->AllocatePagesForMdl(highAddress, RequestedSize_);

    if (NULL == pBufferMdl)
    {
        return STATUS_UNSUCCESSFUL;
    }

    // From MSDN:
    // "Since the Windows audio stack does not support a mechanism to express memory access
    //  alignment requirements for buffers, audio drivers must select a caching type for mapped
    //  memory buffers that does not impose platform-specific alignment requirements. In other
    //  words, the caching type used by the audio driver for mapped memory buffers, must not make
    //  assumptions about the memory alignment requirements for any specific platform.
    //
    //  This method maps the physical memory pages in the MDL into kernel-mode virtual memory.
    //  Typically, the miniport driver calls this method if it requires software access to the
    //  scatter-gather list for an audio buffer. In this case, the storage for the scatter-gather
    //  list must have been allocated by the IPortWaveRTStream::AllocatePagesForMdl or
    //  IPortWaveRTStream::AllocateContiguousPagesForMdl method.
    //
    //  A WaveRT miniport driver should not require software access to the audio buffer itself."
    //
    m_pDmaBuffer = (BYTE*)m_pPortStream->MapAllocatedPages(pBufferMdl, MmCached);
    if (m_pDmaBuffer == NULL)
    {
        m_pPortStream->FreePagesFromMdl(pBufferMdl);
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    m_ulDmaBufferSize = RequestedSize_;
    m_ulNotificationsPerBuffer = 0;
    m_RenderCommits.Reset();

    *AudioBufferMdl_ = pBufferMdl;
    *ActualSize_ = RequestedSize_;
    *OffsetFromFirstPage_ = 0;
    *CacheType_ = MmCached;

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::GetPosition
(
    _Out_   KSAUDIO_POSITION    *Position_
)
{
    if (Position_ == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    NTSTATUS ntStatus;

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    if (m_KsState == KSSTATE_RUN)
    {
        //
        // Get the current time and update position.
        //
        LARGE_INTEGER ilQPC = KeQueryPerformanceCounter(NULL);
        UpdatePosition(ilQPC);
    }

    Position_->PlayOffset = m_ullPlayPosition;
    Position_->WriteOffset = m_ullWritePosition;

    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    ntStatus = STATUS_SUCCESS;

    return ntStatus;
}

//=============================================================================
// CMiniportWaveRTStream::GetReadPacket
//
//  Returns information about the next packet for the OS to read.
//
// Return value
//
//  Returns STATUS_DEVICE_NOT_READY if no new packets are available.
//
// IRQL - PASSIVE_LEVEL
//
// Remarks
//  Although called at passive level, this routine is non-paged code because
//  it is called in the streaming path where page faults should be avoided.
//
// ISSUE-2014/10/4 Will this work correctly across pause/play?
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetReadPacket
(
    _Out_ ULONG* PacketNumber,
    _Out_ DWORD* Flags,
    _Out_ ULONG64* PerformanceCounterValue,
    _Out_ BOOL* MoreData
)
{
    if (PacketNumber == NULL || Flags == NULL ||
        PerformanceCounterValue == NULL || MoreData == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    ULONG availablePacketNumber;
    ULONG droppedPackets;

    // The call must be from event driven mode
    if (m_ulNotificationsPerBuffer == 0 ||
        m_ulDmaBufferSize == 0 || m_ulDmaMovementRate == 0 ||
        m_ullPerformanceCounterFrequency.QuadPart <= 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    *Flags = 0;

    if (m_KsState < KSSTATE_PAUSE)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    if (m_KsState == KSSTATE_RUN) {
        UpdatePosition(KeQueryPerformanceCounter(NULL));
    }
    LONGLONG packetCounter = m_llPacketCounter;
    ULONGLONG ullLinearPosition = m_ullLinearPosition;
    ULONGLONG hnsElapsedTimeCarryForward = m_hnsElapsedTimeCarryForward;
    ULONGLONG ullDmaTimeStamp = m_ullDmaTimeStamp;

    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    if (packetCounter == 0) { return STATUS_DEVICE_NOT_READY; }

    // The 0-based number of the last completed packet
    // FUTURE-2014/10/27 Update to allow different numbers of packets per WaveRT buffer
    // Startup is handled above. The low-word subtraction remains well-defined
    // at the packet-number wrap boundary without signed counter arithmetic.
    availablePacketNumber = LODWORD(packetCounter) - 1;

    // If no new packets are available...
    if (availablePacketNumber == m_ulLastOsReadPacket)
    {
        return STATUS_DEVICE_NOT_READY;
    }

    // If more than one packet has transferred since the last packet read by
    // the OS, then those were dropped. That is, a glitch occurred.
    droppedPackets = availablePacketNumber - m_ulLastOsReadPacket - 1;
    if (droppedPackets > 0)
    {
        // Trace a glitch
    }

    // Return next packet number to be read
    *PacketNumber = availablePacketNumber;

    // Compute and return timestamp corresponding to the first sample of the available packet. In a real hardware
    // driver, the timestamp would be computed in a driver and hardware specific manner. In this sample
    // driver, it is extrapolated from the sample driver's internal simulated position correlation
    // [m_ullLinearPosition @ m_ullDmaTimeStamp] and the sample's internal 64-bit packet counter, subtracting
    // 1 from the packet counter to compute the time at the start of that last completed packet.
    ULONG packetSize = m_ulDmaBufferSize / m_ulNotificationsPerBuffer;
    if (packetSize == 0 || packetCounter <= 0 ||
        static_cast<ULONGLONG>(packetCounter) >
            MAXULONGLONG / packetSize)
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    ULONGLONG linearPositionOfAvailablePacket =
        AudioRouterCompletedPacketStart(static_cast<ULONGLONG>(packetCounter), packetSize);
    // Need to divide by (1000 * 10000 because m_ulDmaMovementRate is average bytes per sec
    if (hnsElapsedTimeCarryForward >
        MAXULONGLONG / m_ulDmaMovementRate)
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    ULONGLONG carryForwardBytes = (hnsElapsedTimeCarryForward * m_ulDmaMovementRate) / 10000000;
    if (carryForwardBytes > MAXULONGLONG - ullLinearPosition)
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    ULONGLONG advancedLinearPosition = ullLinearPosition + carryForwardBytes;
    if (advancedLinearPosition < linearPositionOfAvailablePacket)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }
    ULONGLONG deltaLinearPosition =
        advancedLinearPosition - linearPositionOfAvailablePacket;
    if (deltaLinearPosition > MAXULONGLONG / 10000000)
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    ULONGLONG deltaTimeInHns = deltaLinearPosition * 10000000 / m_ulDmaMovementRate;
    if (deltaTimeInHns > ullDmaTimeStamp)
    {
        return STATUS_INVALID_DEVICE_STATE;
    }
    ULONGLONG timeOfAvailablePacketInHns = ullDmaTimeStamp - deltaTimeInHns;
    ULONGLONG timeOfAvailablePacketInQpc = 0;
    if (!AudioRouterHnsToQpc(timeOfAvailablePacketInHns,
        static_cast<ULONGLONG>(m_ullPerformanceCounterFrequency.QuadPart),
        &timeOfAvailablePacketInQpc))
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    *PerformanceCounterValue = timeOfAvailablePacketInQpc;

    // No flags are defined yet
    *Flags = 0;

    // This sample does not internally buffer data so there is never more data
    // than revealed by the results from this routine.
    *MoreData = FALSE;

    // Update the last packet read by the OS
    m_ulLastOsReadPacket = availablePacketNumber;

    return STATUS_SUCCESS;
}

#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::SetWritePacket
(
    _In_ ULONG      PacketNumber,
    _In_ DWORD      Flags,
    _In_ ULONG      EosPacketLength
)
{
    UNREFERENCED_PARAMETER(EosPacketLength);
    NTSTATUS ntStatus;

    if (m_ulDmaBufferSize == 0 || m_ulDmaMovementRate == 0)
    {
        return STATUS_DEVICE_NOT_READY;
    }

    // The call must be from event driven mode
    if (m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    // The OS wrote this packet into its physical slot before this call. Record
    // that provenance before progress consumes any byte, so bytes the OS has
    // already written (including a late write of the packet now transferring)
    // play, and the slot's previous lap can never replay. The return code
    // below is the documented admission result and does not change provenance.
    const bool endOfStream = (Flags & KSSTREAM_HEADER_OPTIONSF_ENDOFSTREAM) != 0;
    m_RenderCommits.RecordWrite(PacketNumber, static_cast<ULONGLONG>(m_llPacketCounter),
                                m_ulNotificationsPerBuffer,
                                !endOfStream && !m_bEoSReceived);
    if (m_KsState == KSSTATE_RUN) {
        UpdatePosition(KeQueryPerformanceCounter(NULL));
    }
    const bool running = m_KsState == KSSTATE_RUN;
    const ULONGLONG currentPacket = static_cast<ULONGLONG>(m_llPacketCounter);
    const LONG delta = AudioRouterRenderCommitDelta(PacketNumber, currentPacket, running);
    AR_BRIDGE_STREAM_ACTIVITY activity = {};
    if (m_bEoSReceived) {
        ntStatus = STATUS_INVALID_DEVICE_STATE;
    } else if (delta < 0) {
        // Already transferred or transferring; its unconsumed bytes may play.
        ntStatus = STATUS_DATA_LATE_ERROR;
        activity.PacketsLate = 1;
    } else if (delta > 0) {
        ntStatus = STATUS_DATA_OVERRUN;
        activity.PacketsOverrun = 1;
    } else if (endOfStream) {
        // EOS support remains a separate lifecycle task.
        ntStatus = STATUS_INVALID_PARAMETER;
    } else {
        const ULONG packetSize = m_ulDmaBufferSize / m_ulNotificationsPerBuffer;
        const ULONG packetIndex = PacketNumber % m_ulNotificationsPerBuffer;
        ntStatus = SetCurrentWritePositionInternal(packetIndex * packetSize);
        if (NT_SUCCESS(ntStatus)) {
            m_ulLastOsWritePacket = PacketNumber;
            activity.PacketsAccepted = 1;
        }
    }
    if (!m_bCapture && m_BridgePublishFrames != 0) {
        RecordBridgeActivity(AR_BRIDGE_DIRECTION_RENDER_SOURCE, &activity);
    }
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);
    return ntStatus;
}

//=============================================================================
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetOutputStreamPresentationPosition
(
    _Out_ KSAUDIO_PRESENTATION_POSITION *pPresentationPosition
)
{
    if (pPresentationPosition == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    // The call must be from event driven mode
    if(m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    return GetPresentationPosition(pPresentationPosition);
}

//=============================================================================
#pragma code_seg()
_IRQL_requires_max_(PASSIVE_LEVEL)
NTSTATUS CMiniportWaveRTStream::GetPacketCount
(
    _Out_ ULONG *pPacketCount
)
{
    if (pPacketCount == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    // The call must be from event driven mode
    if(m_ulNotificationsPerBuffer == 0)
    {
        return STATUS_NOT_SUPPORTED;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);

    if (m_KsState == KSSTATE_RUN)
    {
        // Get the current time and update simulated position.
        LARGE_INTEGER ilQPC = KeQueryPerformanceCounter(NULL);
        UpdatePosition(ilQPC);
    }

    *pPacketCount = LODWORD(m_llPacketCounter);
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    return STATUS_SUCCESS;
}

//linear and presentation positions
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::GetPositions(
    _Out_opt_  ULONGLONG* _pullLinearBufferPosition,
    _Out_opt_  ULONGLONG* _pullPresentationPosition,
    _Out_opt_  LARGE_INTEGER* _pliQPCTime
)
{
    DPF_ENTER(("[CMiniportWaveRTStream::GetPositions]"));

    NTSTATUS        ntStatus;
    LARGE_INTEGER   ilQPC;
    KIRQL           oldIrql;

    // Update *_pullLinearBufferPosition with the the number of bytes fetched from waveRT ever since a stream got set into RUN
    // state.
    // Once the stream is set to STOP state, any further read on this call would return zero.

    //
    // Get the current time and update position.
    //
    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    ilQPC = KeQueryPerformanceCounter(NULL);
    if (m_KsState == KSSTATE_RUN)
    {
        UpdatePosition(ilQPC);
    }
    if (_pullLinearBufferPosition)
    {
        *_pullLinearBufferPosition = m_ullLinearPosition;
    }
    if (_pullPresentationPosition)
    {
        *_pullPresentationPosition = m_ullPresentationPosition;
    }
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);
    if (_pliQPCTime)
    {
        *_pliQPCTime = ilQPC;
    }

    ntStatus = STATUS_SUCCESS;

    return ntStatus;
}

NTSTATUS CMiniportWaveRTStream::GetPresentationPosition(_Out_  KSAUDIO_PRESENTATION_POSITION* _pPresentationPosition)
{
    if (_pPresentationPosition == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }
    if (m_pWfExt == NULL || m_pWfExt->Format.nAvgBytesPerSec == 0)
    {
        return STATUS_DEVICE_NOT_READY;
    }
    LARGE_INTEGER timeStamp;

    DPF_ENTER(("[CMiniportWaveRTStream::GetPresentationPosition]"));

    ULONGLONG ullLinearPosition = { 0 };
    ULONGLONG ullPresentationPosition = { 0 };
    NTSTATUS status = STATUS_SUCCESS;

    status = GetPositions(&ullLinearPosition, &ullPresentationPosition, &timeStamp);
    if (!NT_SUCCESS(status))
    {
        return status;
    }

    ULONGLONG sampleRate = m_pWfExt->Format.nSamplesPerSec;
    if (sampleRate != 0 && ullPresentationPosition > MAXULONGLONG / sampleRate)
    {
        return STATUS_INTEGER_OVERFLOW;
    }
    ULONGLONG positionNumerator = ullPresentationPosition * sampleRate;
    _pPresentationPosition->u64PositionInBlocks =
        positionNumerator / m_pWfExt->Format.nAvgBytesPerSec;
    _pPresentationPosition->u64QPCPosition = (UINT64)timeStamp.QuadPart;

    return STATUS_SUCCESS;
}

#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::SetCurrentWritePositionInternal(_In_  ULONG _ulCurrentWritePosition)
{
    DPF_ENTER(("[CMiniportWaveRTStream::SetCurrentWritePositionInternal]"));

    ASSERT(m_bEoSReceived == FALSE);

    if (m_bEoSReceived)
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }

    if (_ulCurrentWritePosition > m_ulDmaBufferSize)
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }

    if (m_pMiniport == NULL)
    {
        return STATUS_DEVICE_NOT_READY;
    }

    PADAPTERCOMMON pAdapterComm = m_pMiniport->GetAdapterCommObj();
    if (pAdapterComm == NULL)
    {
        return STATUS_DEVICE_NOT_READY;
    }

    //Event type: eMINIPORT_SET_WAVERT_BUFFER_WRITE_POSITION
    //Parameter 1: Current linear buffer position
    //Parameter 2: Previous WaveRtBufferWritePosition that the driver received
    //Parameter 3: Target WaveRtBufferWritePosition received from portcls
    //Parameter 4: 0
    pAdapterComm->WriteEtwEvent(eMINIPORT_SET_WAVERT_BUFFER_WRITE_POSITION,
        m_ullLinearPosition, // replace with the correct "Current linear buffer position"
        m_ulCurrentWritePosition,
        _ulCurrentWritePosition, // this is new write position
        0); // always zero

//
// Check for eMINIPORT_GLITCH_REPORT - Same WaveRT buffer write during event driven mode.
//
    if (m_hnsNotificationInterval > 0)
    {
        if (m_ulCurrentWritePosition == _ulCurrentWritePosition)
        {
            //Event type: eMINIPORT_GLITCH_REPORT
            //Parameter 1: Current linear buffer position
            //Parameter 2: Previous WaveRtBufferWritePosition that the driver received
            //Parameter 3: Major glitch code: 3: Received same WaveRT buffer twice in a row during event driven mode
            //Parameter 4: Minor code for the glitch cause
            pAdapterComm->WriteEtwEvent(eMINIPORT_GLITCH_REPORT,
                m_ullLinearPosition, // replace with the correct "Current linear buffer position"
                m_ulCurrentWritePosition,
                3, // received same WaveRT buffer twice in a row during event driven mode
                _ulCurrentWritePosition);
        }
    }

    m_ulCurrentWritePosition = _ulCurrentWritePosition;
    InterlockedExchange(&m_IsCurrentWritePositionUpdated, 1);

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg()
NTSTATUS CMiniportWaveRTStream::SetState
(
    _In_    KSSTATE State_
)
{
    NTSTATUS        ntStatus        = STATUS_SUCCESS;
    KIRQL oldIrql;

    // Spew an event for a pin state change request from portcls
    //Event type: eMINIPORT_PIN_STATE
    switch (State_)
    {
        case KSSTATE_STOP:
            if (m_KsState == KSSTATE_ACQUIRE)
            {
                // Acquire stream resources
            }
            KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
            m_KsState = KSSTATE_STOP;
            // Reset DMA
            // Discard queued audio but retain the last acknowledgement so
            // the same shared block cannot replay after STOP/RUN.
            m_CaptureQueue.Clear();
            m_RenderQueue.Clear();
            m_BridgeScratchFrames = 0;
            m_BridgeScratchFrameOffset = 0;
            m_llPacketCounter = 0;
            m_llLastNotifiedPacketCounter = 0;
            m_hnsElapsedTimeCarryForward = 0;
            m_byteDisplacementCarryForward = 0;
            m_ullPlayPosition = 0;
            m_ullWritePosition = 0;
            m_ullLinearPosition = 0;
            m_ullPresentationPosition = 0;

            // Reset OS read/write positions
            m_ulLastOsReadPacket = ULONG_MAX;
            m_ulCurrentWritePosition = 0;
            m_ulLastOsWritePacket = ULONG_MAX;
            m_RenderCommits.Reset();
            m_bEoSReceived = FALSE;
            m_bLastBufferRendered = FALSE;
            m_bEosCompletionNotified = FALSE;

            KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

            break;

        case KSSTATE_ACQUIRE:
            if (m_KsState == KSSTATE_STOP)
            {
                // Acquire stream resources
            }
            break;

        case KSSTATE_PAUSE:

            if (m_KsState > KSSTATE_PAUSE)
            {
                //
                // Run -> Pause
                //

                // Pause DMA
                if (m_hnsNotificationInterval > 0 && m_pNotificationTimer != NULL)
                {
                    ExCancelTimer(m_pNotificationTimer, NULL);
                    KeFlushQueuedDpcs();

                    // GetPositions below finalizes DMA. Packet cadence follows
                    // its retained position through PAUSE, not a second clock.
                }
            }
            // This call updates the linear buffer and presentation positions.
            GetPositions(NULL, NULL, NULL);
            break;

        case KSSTATE_RUN:
            if (m_hnsNotificationInterval > 0 && m_pNotificationTimer == NULL)
            {
                ntStatus = STATUS_INSUFFICIENT_RESOURCES;
                break;
            }
            // Start DMA
            LARGE_INTEGER ullPerfCounterTemp;
            ullPerfCounterTemp = KeQueryPerformanceCounter(&m_ullPerformanceCounterFrequency);
            m_ullDmaTimeStamp = KSCONVERT_PERFORMANCE_TIME(m_ullPerformanceCounterFrequency.QuadPart, ullPerfCounterTemp);

            break;
    }

    if (!NT_SUCCESS(ntStatus))
    {
        return ntStatus;
    }

    KeAcquireSpinLock(&m_PositionSpinLock, &oldIrql);
    m_KsState = State_;
    if (m_pNotificationTimer != NULL && m_hnsNotificationInterval > 0) {
        if (State_ == KSSTATE_RUN ||
            (State_ == KSSTATE_PAUSE && !m_bCapture && m_RenderQueue.Count != 0)) {
            // Service transport every millisecond. A paused stream only drains
            // completed blocks; it does not advance DMA or send notifications.
            ExSetTimer(m_pNotificationTimer, -HNSTIME_PER_MILLISECOND,
                       HNSTIME_PER_MILLISECOND, NULL);
        } else {
            ExCancelTimer(m_pNotificationTimer, NULL);
        }
    }
    KeReleaseSpinLock(&m_PositionSpinLock, oldIrql);

    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS CMiniportWaveRTStream::SetFormat
(
    _In_    KSDATAFORMAT    *DataFormat_
)
{
    UNREFERENCED_PARAMETER(DataFormat_);

    PAGED_CODE();

    return STATUS_NOT_SUPPORTED;
}

#pragma code_seg()

static ULONG AdvanceDmaOffset(
    _In_ ULONGLONG Position,
    _In_ ULONG Displacement,
    _In_ ULONG BufferSize)
{
    ULONG offset = static_cast<ULONG>(Position % BufferSize);
    ULONG delta = Displacement % BufferSize;
    return offset >= BufferSize - delta
        ? offset - (BufferSize - delta)
        : offset + delta;
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::UpdatePosition
(
    _In_ LARGE_INTEGER ilQPC
)
{
    // The notification timer can outlive a client buffer during teardown.
    // Do not perform position arithmetic or call either bridge direction until
    // the DMA buffer and byte rate are valid; in particular, this prevents a
    // zero-sized modulo on an early or late callback.
    if (m_pDmaBuffer == NULL || m_ulDmaBufferSize == 0 || m_ulDmaMovementRate == 0 ||
        m_pWfExt == NULL || m_pWfExt->Format.nBlockAlign == 0 ||
        m_ullPerformanceCounterFrequency.QuadPart == 0 ||
        ilQPC.QuadPart < 0)
    {
        return;
    }

    // Convert ticks to 100ns units.
    LONGLONG  hnsCurrentTime = KSCONVERT_PERFORMANCE_TIME(m_ullPerformanceCounterFrequency.QuadPart, ilQPC);

    // Calculate the time elapsed since the last call to GetPosition() or since the
    // DMA engine started.  Note that the division by 10000 to convert to milliseconds
    // may cause us to lose some of the time, so we will carry the remainder forward
    // to the next GetPosition() call.
    //
    if (hnsCurrentTime < 0 || static_cast<ULONGLONG>(hnsCurrentTime) < m_ullDmaTimeStamp)
    {
        return;
    }
    ULONGLONG elapsedHns = static_cast<ULONGLONG>(hnsCurrentTime) - m_ullDmaTimeStamp;
    if (elapsedHns > MAXULONGLONG - m_hnsElapsedTimeCarryForward)
    {
        m_ullDmaTimeStamp = static_cast<ULONGLONG>(hnsCurrentTime);
        m_hnsElapsedTimeCarryForward = 0;
        m_byteDisplacementCarryForward = 0;
        return;
    }
    elapsedHns += m_hnsElapsedTimeCarryForward;
    ULONGLONG elapsedMilliseconds = elapsedHns / 10000;
    if (elapsedMilliseconds > MAXULONG)
    {
        m_ullDmaTimeStamp = static_cast<ULONGLONG>(hnsCurrentTime);
        m_hnsElapsedTimeCarryForward = elapsedHns % 10000;
        m_byteDisplacementCarryForward = 0;
        return;
    }
    ULONG TimeElapsedInMS = static_cast<ULONG>(elapsedMilliseconds);

    // Carry forward the remainder of this division so we don't fall behind with our position too much.
    //
    m_hnsElapsedTimeCarryForward = elapsedHns % 10000;

    // Calculate how many bytes in the DMA buffer would have been processed in the elapsed
    // time.  Note that the division by 1000 to convert to milliseconds may cause us to
    // lose part of a sample frame, so carry that fraction to the next call.
    //
    // need to divide by 1000 because m_ulDmaMovementRate is average bytes per sec.

    ULONGLONG byteNumerator = static_cast<ULONGLONG>(m_ulDmaMovementRate) *
        static_cast<ULONGLONG>(TimeElapsedInMS);
    if (byteNumerator > MAXULONGLONG - m_byteDisplacementCarryForward)
    {
        m_ullDmaTimeStamp = static_cast<ULONGLONG>(hnsCurrentTime);
        m_byteDisplacementCarryForward = 0;
        return;
    }
    byteNumerator += m_byteDisplacementCarryForward;
    const ULONG frameDenominator = 1000UL * m_pWfExt->Format.nBlockAlign;
    ULONGLONG byteDisplacementWide = AudioRouterFrameAlignedByteCount(
        byteNumerator, m_pWfExt->Format.nBlockAlign);
    if (byteDisplacementWide > MAXULONG)
    {
        m_ullDmaTimeStamp = static_cast<ULONGLONG>(hnsCurrentTime);
        m_byteDisplacementCarryForward = static_cast<ULONG>(byteNumerator % frameDenominator);
        return;
    }
    ULONG ByteDisplacement = static_cast<ULONG>(byteDisplacementWide);
    m_byteDisplacementCarryForward = static_cast<ULONG>(byteNumerator % frameDenominator);

    // These counters are monotonic and are consumed by PortCls position
    // queries.  Do not let a long-lived stream wrap either counter and expose
    // a position that moves backwards.  Update the time anchor so a later
    // callback can recover from the invalid sample without repeating the same
    // overflowing displacement forever.
    if (ByteDisplacement > MAXULONGLONG - m_ullPresentationPosition ||
        ByteDisplacement > MAXULONGLONG - m_ullLinearPosition)
    {
        m_ullDmaTimeStamp = static_cast<ULONGLONG>(hnsCurrentTime);
        m_hnsElapsedTimeCarryForward = 0;
        m_byteDisplacementCarryForward = 0;
        return;
    }

    // Increment presentation position even after last buffer is rendered.
    m_ullPresentationPosition += ByteDisplacement;

    if (m_bCapture)
    {
        // Write sine wave to buffer.
        WriteBytes(ByteDisplacement);
    }
    else
    {
        ULONG nextWritePosition = AdvanceDmaOffset(
            m_ullWritePosition, ByteDisplacement, m_ulDmaBufferSize);

        if (m_bEoSReceived)
        {
            // since EoS flag is set, we'll need to make sure not to read data beyond EOS position.
            // If driver's current position is less than EoS position, then make sure not to read data beyond EoS.
            if (m_ullWritePosition <= m_ulCurrentWritePosition)
            {
                ByteDisplacement = min(ByteDisplacement, m_ulCurrentWritePosition - (ULONG)m_ullWritePosition);
            }
            // If our current position is ahead of EoS position and we'll wrap around after new position then adjust
            // new position if it crosses EoS.
            else if (nextWritePosition < m_ullWritePosition)
            {
                if (nextWritePosition > m_ulCurrentWritePosition)
                {
                    ByteDisplacement = ByteDisplacement -
                        (nextWritePosition - m_ulCurrentWritePosition);
                }
            }
        }

        nextWritePosition = AdvanceDmaOffset(
            m_ullWritePosition, ByteDisplacement, m_ulDmaBufferSize);

        // If the last packet was rendered(read in the sample driver's case), send out an etw event.
        if (m_bEoSReceived && !m_bLastBufferRendered
            && nextWritePosition == m_ulCurrentWritePosition)
        {
            m_bLastBufferRendered = TRUE;
        }

        // Read render DMA for the bridge; callbacks perform no file or logging I/O.
        ReadBytes(ByteDisplacement);
    }

    // Increment the DMA position by the number of bytes displaced since the last
    // call to UpdatePosition() and ensure we properly wrap at buffer length.
    //
    m_ullPlayPosition = m_ullWritePosition = AdvanceDmaOffset(
        m_ullWritePosition, ByteDisplacement, m_ulDmaBufferSize);

    // m_ullDmaTimeStamp is updated in both GetPostion and GetLinearPosition calls
    // so m_ullLinearPosition needs to be updated accordingly here
    //
    m_ullLinearPosition += ByteDisplacement;
    const ULONGLONG completedPackets = AudioRouterCompletedPackets(
        m_ullLinearPosition, m_ulDmaBufferSize, m_ulNotificationsPerBuffer);
    // Saturate before the counter can wrap into a negative value. Queries
    // and notifications use the same actual completed DMA packet count.
    m_llPacketCounter = static_cast<LONGLONG>(min(
        completedPackets, static_cast<ULONGLONG>(MAXLONGLONG)));

    // Update the DMA time stamp for the next call to GetPosition()
    //
    m_ullDmaTimeStamp = hnsCurrentTime;
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::WriteBytes
(
    _In_ ULONG ByteDisplacement
)
/*++

Routine Description:

This function writes capture DMA from the bounded bridge queue.

Arguments:

ByteDisplacement - # of bytes to process.

--*/
{
    if (m_pDmaBuffer == NULL || m_ulDmaBufferSize == 0) {
        return;
    }
    RefreshBridgePublishShape();
    USHORT readFrames = 0;
    USHORT readChannels = 0;
    ULONG readSampleRate = 0;
    ULONGLONG readGeneration = 0;
    const NTSTATUS readShapeStatus = AudioRouterGetLeaseShapeForDirection(
        static_cast<USHORT>(m_pMiniport->GetCableBusIndex()),
        AR_BRIDGE_DIRECTION_CAPTURE_SINK, &readFrames, &readChannels,
        &readSampleRate,
        &readGeneration);
    if (NT_SUCCESS(readShapeStatus) && readFrames != 0 && readChannels != 0 &&
        AudioRouterStreamGenerationChanged(m_BridgeReadGeneration, readGeneration)) {
        // Reset valid length/sequence only. The block copy overwrites every
        // sample consumed before output; never clear the full DPC scratch array.
        m_BridgeScratchFrames = 0;
        m_BridgeScratchFrameOffset = 0;
        m_CaptureQueue.Reset();
        m_BridgeReadGeneration = readGeneration;
    } else if (!NT_SUCCESS(readShapeStatus) || readFrames == 0 || readChannels == 0) {
        if (m_BridgeReadGeneration != 0) {
            m_BridgeScratchFrames = 0;
            m_BridgeScratchFrameOffset = 0;
            m_CaptureQueue.Reset();
            m_BridgeReadGeneration = 0;
        }
    }
    ULONG bufferOffset = m_ullLinearPosition % m_ulDmaBufferSize;

    const BOOLEAN bridgeFormat = IsBridgePcmFormat(m_pWfExt);
    // The capture endpoint is the virtual sink for processed audio. Its
    // advertised integer/float format is converted at the callback boundary;
    // unavailable or incoherent bridge blocks are rendered as silence.
    const ULONG bridgeChannels = bridgeFormat ? m_pWfExt->Format.nChannels : 0;
    const ULONG deviceFrameBytes = bridgeFormat
        ? m_pWfExt->Format.nBlockAlign : 0;
    const ULONG deviceBytesPerSample = bridgeFormat
        ? m_pWfExt->Format.wBitsPerSample / 8 : 0;
    const BOOLEAN captureLeaseActive = NT_SUCCESS(readShapeStatus) &&
        readFrames != 0 && readChannels != 0;
    // A lease at another rate or channel count must never play: wrong-speed
    // audio is worse than silence (17 §5.2). Count it and output silence.
    const BOOLEAN captureLeaseUsable = captureLeaseActive && bridgeFormat &&
        readSampleRate == m_pWfExt->Format.nSamplesPerSec &&
        readChannels == bridgeChannels;
    if (!captureLeaseUsable) {
        m_CaptureQueue.Clear();
    }
    AR_BRIDGE_STREAM_ACTIVITY activity = {};
    if (captureLeaseActive && !captureLeaseUsable) {
        activity.FormatMismatches = 1;
    }
    const AudioRouterDmaWindow window = AudioRouterSurvivingDmaWindow(
        m_ullLinearPosition, ByteDisplacement, m_ulDmaBufferSize);
    bufferOffset = window.Offset;
    ByteDisplacement = window.Bytes;
    if (captureLeaseUsable && deviceFrameBytes != 0) {
        // Older laps no longer exist. Count their missed audio without
        // repeatedly overwriting DMA or consuming queued good blocks.
        activity.UnderrunFrames += window.SkippedBytes / deviceFrameBytes;
    }

    DOUBLE* captureBlocks[2] = { m_BridgeScratch, m_BridgePrefetch };
    bool captureSnapshotValid = true;
    // Copy into free private storage before consuming the current block.
    // A failed/noncoherent publication never invalidates queued good audio.
    // At most two copies per call; no wait for the user-mode publisher.
    auto prefetch = [&]() {
        if (!captureSnapshotValid) { return; }
        m_CaptureQueue.Prefetch([&](ULONG slot, ULONGLONG sequence) {
            AR_BRIDGE_BLOCK_HEADER header = {};
            ULONG nonFinite = 0;
            NTSTATUS status = AudioRouterCopyLeaseBlockForDirection(
                static_cast<USHORT>(m_pMiniport->GetCableBusIndex()),
                AR_BRIDGE_DIRECTION_CAPTURE_SINK,
                sequence,
                captureBlocks[slot],
                ARRAYSIZE(m_BridgeScratch), &header, &nonFinite);
            activity.NonFiniteSamples += nonFinite;
            if (!NT_SUCCESS(status)) { return false; }
            if (header.Generation != readGeneration) {
                // Lease turnover raced the shape snapshot. Discard both
                // generations and renegotiate on the next callback.
                m_CaptureQueue.Reset();
                captureSnapshotValid = false;
                return false;
            }
            if (header.Channels != bridgeChannels) {
                activity.FormatMismatches += 1;
                m_CaptureQueue.Clear();
                captureSnapshotValid = false;
                return false;
            }
            activity.SequenceGaps += AudioRouterSequenceGap(
                m_CaptureQueue.Sequence, header.Sequence);
            return m_CaptureQueue.Commit(header.Frames, header.Sequence);
        });
    };
    if (captureLeaseUsable && ByteDisplacement != 0) { prefetch(); }

    // The surviving window is at most one lap (two segments across wrap).
    while (ByteDisplacement > 0)
    {
        ULONG runWrite = min(ByteDisplacement, m_ulDmaBufferSize - bufferOffset);

        if (!captureLeaseUsable || bridgeChannels > AR_BRIDGE_MAX_CHANNELS ||
            deviceFrameBytes == 0 || runWrite < deviceFrameBytes) {
            RtlZeroMemory(m_pDmaBuffer + bufferOffset, runWrite);
        } else {
            ULONG frames = runWrite / deviceFrameBytes;
            ULONG bytes = frames * deviceFrameBytes;
            ULONG writtenFrames = 0;
            while (writtenFrames < frames) {
                if (m_CaptureQueue.Count == 0) { prefetch(); }
                ULONG available = m_CaptureQueue.Available();
                if (available == 0) { break; }
                ULONG copyFrames = min(frames - writtenFrames, available);
                for (ULONG frame = 0; frame < copyFrames; ++frame) {
                    for (ULONG channel = 0; channel < bridgeChannels; ++channel) {
                        WriteBridgeSample(
                            m_pDmaBuffer + bufferOffset +
                                (writtenFrames + frame) * deviceFrameBytes +
                                channel * deviceBytesPerSample,
                            m_pWfExt,
                            captureBlocks[m_CaptureQueue.Head][
                                (m_CaptureQueue.Offset + frame) * bridgeChannels +
                                channel]);
                    }
                }
                m_CaptureQueue.Consume(copyFrames);
                writtenFrames += copyFrames;
                prefetch();
                if (copyFrames == 0) {
                    break;
                }
            }
            if (writtenFrames < frames) {
                // The producer had no newer block: silence, counted (§5.4).
                activity.UnderrunFrames += frames - writtenFrames;
                RtlZeroMemory(m_pDmaBuffer + bufferOffset +
                                  writtenFrames * deviceFrameBytes,
                              (frames - writtenFrames) * deviceFrameBytes);
            }
            if (bytes < runWrite) {
                RtlZeroMemory(m_pDmaBuffer + bufferOffset + bytes,
                              runWrite - bytes);
            }
        }

        bufferOffset = (bufferOffset + runWrite) % m_ulDmaBufferSize;
        ByteDisplacement -= runWrite;
    }
    if (captureLeaseActive) {
        RecordBridgeActivity(AR_BRIDGE_DIRECTION_CAPTURE_SINK, &activity);
    }
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::ReadBytes
(
    _In_ ULONG ByteDisplacement
)
/*++

Routine Description:

Convert render DMA into complete bridge quanta, retaining bursts until read.

Arguments:

ByteDisplacement - # of bytes to process.

--*/
{
    if (m_pDmaBuffer == NULL || m_ulDmaBufferSize == 0) {
        return;
    }
    const BOOLEAN renderFormatMismatch = RefreshBridgePublishShape();
    ULONG bufferOffset = m_ullLinearPosition % m_ulDmaBufferSize;
    ULONGLONG linearByte = m_ullLinearPosition;
    const ULONG packetBytes = m_ulNotificationsPerBuffer == 0 ? 0 :
        m_ulDmaBufferSize / m_ulNotificationsPerBuffer;
    const BOOLEAN bridgeFormat = IsBridgePcmFormat(m_pWfExt);
    AR_BRIDGE_STREAM_ACTIVITY activity = {};
    activity.FormatMismatches = renderFormatMismatch ? 1 : 0;
    DrainRenderQueue();

    // After more than one lap, older DMA bytes no longer exist. Read only the
    // surviving lap, count the lost frames, and never replay the same ring in
    // an unbounded catch-up loop. UpdatePosition still advances the full clock.
    if (ByteDisplacement > m_ulDmaBufferSize) {
        const ULONG skipped = ByteDisplacement - m_ulDmaBufferSize;
        bufferOffset = AdvanceDmaOffset(bufferOffset, skipped, m_ulDmaBufferSize);
        linearByte += skipped;
        ByteDisplacement = m_ulDmaBufferSize;
        if (m_BridgePublishFrames != 0 && bridgeFormat) {
            activity.OverrunFrames += skipped / m_pWfExt->Format.nBlockAlign +
                m_BridgeScratchFrames;
        }
        m_BridgeScratchFrames = 0;
    }

    // At most one surviving DMA lap (two segments across the wrap).
    while (ByteDisplacement > 0)
    {
        ULONG runWrite = min(ByteDisplacement, m_ulDmaBufferSize - bufferOffset);
        ULONG frameBytes = bridgeFormat ? m_pWfExt->Format.nBlockAlign : 0;
        ULONG bytesPerSample = bridgeFormat
            ? m_pWfExt->Format.wBitsPerSample / 8 : 0;
        ULONG frames = frameBytes == 0 ? 0 : runWrite / frameBytes;
        if (m_BridgePublishFrames != 0 && frames != 0) {
            ULONG consumedFrames = 0;
            while (consumedFrames < frames) {
                if (m_BridgeScratchFrames > m_BridgePublishFrames) {
                    // Preserve the callback invariant before the subtraction
                    // below; malformed state fails closed for this quantum.
                    m_BridgeScratchFrames = 0;
                    m_BridgeScratchFrameOffset = 0;
                }
                ULONG needed = m_BridgePublishFrames - m_BridgeScratchFrames;
                ULONG copyFrames = min(needed, frames - consumedFrames);
                for (ULONG frame = 0; frame < copyFrames; ++frame) {
                    const bool committed = m_RenderCommits.Contains(
                        linearByte + (consumedFrames + frame) * frameBytes, packetBytes,
                        m_ulNotificationsPerBuffer);
                    if (!committed) { ++activity.UnderrunFrames; }
                    for (ULONG channel = 0;
                         channel < m_BridgePublishChannels; ++channel) {
                        m_BridgeScratch[
                            (m_BridgeScratchFrames + frame) *
                                m_BridgePublishChannels + channel] =
                            committed ? ReadBridgeSample(
                                m_pDmaBuffer + bufferOffset +
                                    (consumedFrames + frame) * frameBytes +
                                    channel * bytesPerSample,
                                m_pWfExt) : 0.0;
                    }
                }
                m_BridgeScratchFrames += copyFrames;
                consumedFrames += copyFrames;
                if (m_BridgeScratchFrames == m_BridgePublishFrames) {
                    DrainRenderQueue();
                    if (m_RenderQueue.Push(m_BridgePrefetch, m_BridgeScratch)) {
                        activity.OverrunFrames += m_BridgePublishFrames;
                    }
                    DrainRenderQueue();
                    m_BridgeScratchFrames = 0;
                }
            }
        }
        bufferOffset = (bufferOffset + runWrite) % m_ulDmaBufferSize;
        linearByte += runWrite;
        ByteDisplacement -= runWrite;
    }
    if (m_bLastBufferRendered && m_BridgeScratchFrames != 0 && m_BridgePublishFrames != 0) {
        // The wire shape is fixed. Preserve the final partial quantum and pad
        // only its unused tail with silence rather than discarding valid audio.
        AudioRouterPadRenderTail(m_BridgeScratch, m_BridgeScratchFrames,
                                 m_BridgePublishFrames, m_BridgePublishChannels);
        if (m_RenderQueue.Push(m_BridgePrefetch, m_BridgeScratch)) {
            activity.OverrunFrames += m_BridgePublishFrames;
        }
        m_BridgeScratchFrames = 0;
        DrainRenderQueue();
    }
    if (m_BridgePublishFrames != 0 || renderFormatMismatch) {
        RecordBridgeActivity(AR_BRIDGE_DIRECTION_RENDER_SOURCE, &activity);
    }
}

#pragma code_seg()
VOID CMiniportWaveRTStream::DrainRenderQueue()
{
    m_RenderQueue.Drain(m_BridgePrefetch, [&](const DOUBLE* samples, ULONG count, ULONGLONG generation) {
        return AudioRouterPublishLeaseBlockForDirection(
            static_cast<USHORT>(m_pMiniport->GetCableBusIndex()),
            AR_BRIDGE_DIRECTION_RENDER_SOURCE, generation,
            static_cast<USHORT>(m_BridgePublishFrames),
            static_cast<USHORT>(m_BridgePublishChannels), samples, count) == STATUS_SUCCESS;
    });
}

//=============================================================================
#pragma code_seg()
VOID CMiniportWaveRTStream::RecordBridgeActivity(
    _In_ USHORT Direction,
    _Inout_ AR_BRIDGE_STREAM_ACTIVITY* Activity)
{
    // DISPATCH_LEVEL safe: QPC read, integer division and a lock-free,
    // rundown-protected counter update. No allocation or logging.
    const ULONG blockAlign = m_pWfExt != NULL ? m_pWfExt->Format.nBlockAlign : 0;
    Activity->DevicePositionFrames = blockAlign != 0 ? m_ullLinearPosition / blockAlign : 0;
    Activity->QpcTime = static_cast<ULONGLONG>(KeQueryPerformanceCounter(NULL).QuadPart);
    (void)AudioRouterRecordLeaseActivityForDirection(
        static_cast<USHORT>(m_pMiniport->GetCableBusIndex()), Direction,
        Direction == AR_BRIDGE_DIRECTION_RENDER_SOURCE ? m_BridgeGeneration : m_BridgeReadGeneration,
        Activity);
}

//=============================================================================
#pragma code_seg()
BOOLEAN CMiniportWaveRTStream::RefreshBridgePublishShape()
{
    BOOLEAN formatMismatch = FALSE;
    ULONG previousFrames = m_BridgePublishFrames;
    ULONG previousChannels = m_BridgePublishChannels;
    USHORT frames = 0;
    USHORT channels = 0;
    ULONG sampleRate = 0;
    ULONGLONG generation = 0;
    const BOOLEAN leaseActive = !m_bCapture && NT_SUCCESS(AudioRouterGetLeaseShapeForDirection(
            static_cast<USHORT>(m_pMiniport->GetCableBusIndex()),
            AR_BRIDGE_DIRECTION_RENDER_SOURCE, &frames, &channels, &sampleRate,
            &generation)) && frames != 0 && channels != 0;
    const BOOLEAN bridgeFormat = !m_bCapture && IsBridgePcmFormat(m_pWfExt);
    if (bridgeFormat && leaseActive &&
        channels == m_pWfExt->Format.nChannels &&
        sampleRate == m_pWfExt->Format.nSamplesPerSec) {
        m_BridgePublishFrames = frames;
        m_BridgePublishChannels = channels;
    } else {
        // An active lease with another shape receives nothing (never a
        // wrong-speed stream); the caller counts it as a format mismatch.
        formatMismatch = leaseActive;
        m_BridgePublishFrames = 0;
        m_BridgePublishChannels = 0;
    }
    if (m_BridgePublishFrames != previousFrames ||
        m_BridgePublishChannels != previousChannels ||
        AudioRouterStreamGenerationChanged(m_BridgeGeneration, generation)) {
        // A lease may be replaced with a different quantum while this
        // stream still owns a partial scratch block. Never subtract the new
        // shape from stale frame state in the callback.
        m_BridgeScratchFrames = 0;
        m_BridgeScratchFrameOffset = 0;
        m_RenderQueue.Reset(m_BridgePublishFrames, m_BridgePublishChannels, generation);
    }
    if (AudioRouterStreamGenerationChanged(m_BridgeGeneration, generation)) {
        // Partial samples are unreachable after the index reset and are fully
        // overwritten before publication; keep generation changes bounded.
        m_BridgeScratchFrames = 0;
        m_BridgeScratchFrameOffset = 0;
        m_BridgeGeneration = generation;
    }
    return formatMismatch;
}

//=============================================================================
#pragma code_seg("PAGE")
STDMETHODIMP_(NTSTATUS)
CMiniportWaveRTStream::SetContentId
(
    _In_  ULONG                   contentId,
    _In_  PCDRMRIGHTS             drmRights
)
/*++

Routine Description:

  Sets DRM content Id for this stream. Also updates the Mixed content Id.

Arguments:

  contentId - new content id

  drmRights - rights for this stream.

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();

    DPF_ENTER(("[CMiniportWaveRT::SetContentId]"));

    if (drmRights == NULL || m_pMiniport == NULL)
    {
        return STATUS_INVALID_PARAMETER;
    }

    NTSTATUS    ntStatus;
    ULONG       ulOldContentId = contentId;

    m_ulContentId = contentId;

    //
    // Miniport should create a mixed DrmRights.
    //
    ntStatus = m_pMiniport->UpdateDrmRights();

    //
    // Restore the passed-in content Id.
    //
    if (!NT_SUCCESS(ntStatus))
    {
        m_ulContentId = ulOldContentId;
    }

    //
    //
    // From MSDN:
    //
    // This sample doesn't forward protected content, but if your driver uses
    // lower layer drivers or a different stack to properly work, please see the
    // following info from MSDN:
    //
    // "Before allowing protected content to flow through a data path, the system
    // verifies that the data path is secure. To do so, the system authenticates
    // each module in the data path beginning at the upstream end of the data path
    // and moving downstream. As each module is authenticated, that module gives
    // the system information about the next module in the data path so that it
    // can also be authenticated. To be successfully authenticated, a module's
    // binary file must be signed as DRM-compliant.
    //
    // Two adjacent modules in the data path can communicate with each other in
    // one of several ways. If the upstream module calls the downstream module
    // through IoCallDriver, the downstream module is part of a WDM driver. In
    // this case, the upstream module calls the DrmForwardContentToDeviceObject
    // function to provide the system with the device object representing the
    // downstream module. (If the two modules communicate through the downstream
    // module's COM interface or content handlers, the upstream module calls
    // DrmForwardContentToInterface or DrmAddContentHandlers instead.)
    //
    // DrmForwardContentToDeviceObject performs the same function as
    // PcForwardContentToDeviceObject and IDrmPort2::ForwardContentToDeviceObject."
    //
    // Other supported DRM DDIs for down-level module validation are:
    // DrmForwardContentToInterfaces and DrmAddContentHandlers.
    //
    // For more information, see MSDN's DRM Functions and Interfaces.
    //

    return ntStatus;
} // SetContentId

//=============================================================================
#pragma code_seg()
void
TimerNotifyRT
(
    _In_      PEX_TIMER    Timer,
    _In_opt_  PVOID        DeferredContext
)
{
    LARGE_INTEGER qpc;
    LARGE_INTEGER qpcFrequency;
    BOOL bufferCompleted = FALSE;

    UNREFERENCED_PARAMETER(Timer);

    _IRQL_limited_to_(DISPATCH_LEVEL);

    CMiniportWaveRTStream* _this = (CMiniportWaveRTStream*)DeferredContext;

    if (NULL == _this)
    {
        return;
    }

    KIRQL oldIrql;
    KeAcquireSpinLock(&_this->m_PositionSpinLock, &oldIrql);

    if (_this->m_KsState != KSSTATE_RUN) {
        if (_this->m_KsState == KSSTATE_PAUSE && !_this->m_bCapture) {
            _this->RefreshBridgePublishShape();
            _this->DrainRenderQueue();
        }
        if (_this->m_KsState != KSSTATE_PAUSE || _this->m_RenderQueue.Count == 0) {
            ExCancelTimer(_this->m_pNotificationTimer, NULL);
        }
        goto End;
    }
    if (_this->m_bEosCompletionNotified) {
        ExCancelTimer(_this->m_pNotificationTimer, NULL);
        goto End;
    }

    qpc = KeQueryPerformanceCounter(&qpcFrequency);

    if (_this->m_ullPerformanceCounterFrequency.QuadPart == 0 ||
        qpc.QuadPart < 0 ||
        _this->m_hnsNotificationInterval == 0)
    {
        goto End;
    }

    // Service both bridge directions each tick, including pending render
    // blocks when DMA displacement is zero. Notifications retain their cadence.
    _this->UpdatePosition(qpc);
    bufferCompleted = _this->m_llPacketCounter > _this->m_llLastNotifiedPacketCounter;

    // Do not tell PortCls the final packet is complete while valid tail audio
    // is still private: STOP in response would legitimately discard that tail.
    if (_this->m_bLastBufferRendered && _this->m_RenderQueue.Count != 0) {
        goto End;
    }

    if (!bufferCompleted && !_this->m_bEoSReceived)
    {
        goto End;
    }

    if (_this->m_KsState != KSSTATE_RUN)
    {
        goto End;
    }

    if (_this->m_pMiniport == NULL)
    {
        goto End;
    }
    PADAPTERCOMMON  pAdapterComm = _this->m_pMiniport->GetAdapterCommObj();
    if (pAdapterComm == NULL)
    {
        goto End;
    }

    // Simple buffer underrun detection.
    if (!_this->IsCurrentWaveRTWritePositionUpdated() && !_this->m_bEoSReceived)
    {
        //Event type: eMINIPORT_GLITCH_REPORT
        //Parameter 1: Current linear buffer position
        //Parameter 2: Previous WaveRtBufferWritePosition that the driver received
        //Parameter 3: Major glitch code: 1:WaveRT buffer is underrun
        //Parameter 4: Minor code for the glitch cause
        pAdapterComm->WriteEtwEvent(eMINIPORT_GLITCH_REPORT,
                                    _this->m_ullLinearPosition,
                                    _this->GetCurrentWaveRTWritePosition(),
                                    1,      // WaveRT buffer is underrun
                                    0);
    }

    // Send buffer completion event if either of the following is true
    // 1. Driver consumed a complete buffer for this stream
    // 2. Driver consumed a partial buffer containing EoS for this stream

    if (!IsListEmpty(&_this->m_NotificationList) &&
        (bufferCompleted || _this->m_bLastBufferRendered))
    {
        // One coalesced event is enough: PortCls resynchronizes from the
        // completed-packet count. Position queries must not consume this event.
        _this->m_llLastNotifiedPacketCounter = _this->m_llPacketCounter;
        PLIST_ENTRY leCurrent = _this->m_NotificationList.Flink;
        while (leCurrent != &_this->m_NotificationList)
        {
            NotificationListEntry* nleCurrent = CONTAINING_RECORD( leCurrent, NotificationListEntry, ListEntry);
            KeSetEvent(nleCurrent->NotificationEvent, 0, 0);

            leCurrent = leCurrent->Flink;
        }
    }

    if (_this->m_bLastBufferRendered && _this->m_RenderQueue.Count == 0)
    {
        _this->m_bEosCompletionNotified = TRUE;
        if (_this->m_pNotificationTimer != NULL)
        {
            ExCancelTimer(_this->m_pNotificationTimer, NULL);
        }
    }

End:
    KeReleaseSpinLock(&_this->m_PositionSpinLock, oldIrql);
    return;
}
//=============================================================================
