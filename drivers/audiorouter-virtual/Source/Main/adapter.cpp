/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    adapter.cpp

Abstract:

    Setup and miniport installation.  No resources are used by simple audio sample.
    This sample is to demonstrate how to develop a full featured audio miniport driver.
--*/

#pragma warning (disable : 4127)

//
// All the GUIDS for all the miniports end up in this object.
//
#define PUT_GUIDS_HERE

#include "definitions.h"
#include "bridgeio.h"
#include "endpoints.h"
#include "minipairs.h"

typedef void (*fnPcDriverUnload) (PDRIVER_OBJECT);
fnPcDriverUnload gPCDriverUnloadRoutine = NULL;
extern "C" DRIVER_UNLOAD DriverUnload;
PDEVICE_OBJECT g_BridgeControlDevice = NULL;

typedef struct _AR_BRIDGE_LEASE_STATE {
    KSPIN_LOCK Lock;
    EX_RUNDOWN_REF Rundown;
    BOOLEAN Active;
    BOOLEAN Retiring;
    BOOLEAN RundownStarted;
    ULONGLONG LastHeartbeat100ns;
    AR_BRIDGE_OPEN_REQUEST Request;
    PVOID SectionObject;
    volatile PVOID MappedView;
    volatile ULONG MappedBytes;
    volatile LONG64 NextSequence;
} AR_BRIDGE_LEASE_STATE;

#define AR_BRIDGE_LEASE_SLOTS 2
AR_BRIDGE_LEASE_STATE g_BridgeLeases[AR_BRIDGE_LEASE_SLOTS] = {};

// This helper is intentionally independent of the sample's timer callback.
// It is safe for a future PortCls callback: rundown protects the mapped view
// from CLOSE/expiry/unload, and the callback takes no lease spin lock.
NTSTATUS AudioRouterCopyLeaseBlock(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ ULONGLONG MinimumSequence,
    _Out_writes_(DestinationCapacitySamples) FLOAT* Destination,
    _In_ SIZE_T DestinationCapacitySamples,
    _Out_ AR_BRIDGE_BLOCK_HEADER* Header)
{
    if (Lease == NULL || Destination == NULL || Header == NULL ||
        !ExAcquireRundownProtection(&Lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&Lease->MappedView, NULL, NULL);
    ULONG mappedBytes = Lease->MappedBytes;
    USHORT direction = Lease->Request.Direction;
    ULONGLONG generation = InterlockedCompareExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0, 0);
    KeMemoryBarrier();
    NTSTATUS status = STATUS_DEVICE_NOT_READY;
    if (direction == AR_BRIDGE_DIRECTION_RENDER_SOURCE &&
        view != NULL && mappedBytes != 0 && generation != 0) {
        volatile LONG64* state = reinterpret_cast<volatile LONG64*>(
            static_cast<UCHAR*>(view) + AR_BRIDGE_STATE_OFFSET);
        ULONGLONG stateBefore = static_cast<ULONGLONG>(
            InterlockedCompareExchange64(state, 0, 0));
        if (stateBefore == 0 || (stateBefore & 1) != 0) {
            ExReleaseRundownProtection(&Lease->Rundown);
            return STATUS_DEVICE_BUSY;
        }
        status = AudioRouterCopyBridgeBlock(
            static_cast<const UCHAR*>(view), mappedBytes, generation,
            MinimumSequence, Destination, DestinationCapacitySamples, Header);
        KeMemoryBarrier();
        if (NT_SUCCESS(status) && static_cast<ULONGLONG>(
                InterlockedCompareExchange64(state, 0, 0)) != stateBefore) {
            status = STATUS_RETRY;
        }
    }
    ExReleaseRundownProtection(&Lease->Rundown);
    return status;
}

// Publish one capture quantum into the mapped capture-sink slot. The caller
// supplies an already-interleaved float32 buffer; this routine performs no
// allocation, waits, logging, endpoint access, or control I/O.
NTSTATUS AudioRouterPublishLeaseBlock(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_ USHORT Frames,
    _In_ USHORT Channels,
    _In_reads_(SampleCapacity) const FLOAT* Samples,
    _In_ SIZE_T SampleCapacity)
{
    if (Lease == NULL || Samples == NULL || Frames == 0 ||
        Channels == 0 || Channels > AR_BRIDGE_MAX_CHANNELS ||
        Frames > AR_BRIDGE_MAX_FRAMES) {
        return STATUS_INVALID_PARAMETER;
    }
    SIZE_T sampleCount = static_cast<SIZE_T>(Frames) * Channels;
    if (SampleCapacity < sampleCount) {
        return STATUS_BUFFER_TOO_SMALL;
    }
    if (!ExAcquireRundownProtection(&Lease->Rundown)) {
        return STATUS_DEVICE_NOT_READY;
    }
    PVOID view = InterlockedCompareExchangePointer(&Lease->MappedView, NULL, NULL);
    ULONG mappedBytes = Lease->MappedBytes;
    USHORT direction = Lease->Request.Direction;
    ULONGLONG generation = InterlockedCompareExchange64(
        reinterpret_cast<volatile LONG64*>(&Lease->Request.Generation), 0, 0);
    NTSTATUS status = STATUS_DEVICE_NOT_READY;
    if (direction != AR_BRIDGE_DIRECTION_CAPTURE_SINK || view == NULL || mappedBytes < AR_BRIDGE_PAYLOAD_OFFSET +
            sampleCount * sizeof(FLOAT) || generation == 0) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return status;
    }
    for (SIZE_T index = 0; index < sampleCount; ++index) {
        if (Samples[index] != Samples[index] ||
            Samples[index] > 3.402823466e+38F ||
            Samples[index] < -3.402823466e+38F) {
            ExReleaseRundownProtection(&Lease->Rundown);
            return STATUS_DATA_ERROR;
        }
    }
    volatile LONG64* state = reinterpret_cast<volatile LONG64*>(
        static_cast<UCHAR*>(view) + AR_BRIDGE_STATE_OFFSET);
    ULONGLONG current = static_cast<ULONGLONG>(
        InterlockedCompareExchange64(state, 0, 0));
    if (current & 1) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return STATUS_DEVICE_BUSY;
    }
    if (InterlockedCompareExchange64(
            state, static_cast<LONG64>(current + 1),
            static_cast<LONG64>(current)) != static_cast<LONG64>(current)) {
        ExReleaseRundownProtection(&Lease->Rundown);
        return STATUS_DEVICE_BUSY;
    }
    ULONGLONG sequence = static_cast<ULONGLONG>(
        InterlockedIncrement64(&Lease->NextSequence));
    AR_BRIDGE_BLOCK_HEADER header = { generation, sequence, Frames, Channels,
        static_cast<ULONG>(sampleCount * sizeof(FLOAT)) };
    RtlCopyMemory(static_cast<UCHAR*>(view) + AR_BRIDGE_HEADER_OFFSET,
                  &header, sizeof(header));
    RtlCopyMemory(static_cast<UCHAR*>(view) + AR_BRIDGE_PAYLOAD_OFFSET,
                  Samples, sampleCount * sizeof(FLOAT));
    KeMemoryBarrier();
    InterlockedExchange64(state, static_cast<LONG64>(current + 2));
    ExReleaseRundownProtection(&Lease->Rundown);
    return STATUS_SUCCESS;
}

static void RetireBridgeResources(
    _In_ AR_BRIDGE_LEASE_STATE* Lease,
    _In_opt_ PVOID MappedView,
    _In_opt_ PVOID SectionObject,
    _In_ BOOLEAN RundownStarted)
{
    if (RundownStarted) {
        ExWaitForRundownProtectionRelease(&Lease->Rundown);
    }
    if (MappedView != NULL) {
        MmUnmapViewInSystemSpace(MappedView);
    }
    if (SectionObject != NULL) {
        ObDereferenceObject(SectionObject);
    }
}

static AR_BRIDGE_LEASE_STATE* BridgeLeaseForDirection(_In_ USHORT Direction)
{
    if (Direction == AR_BRIDGE_DIRECTION_RENDER_SOURCE) {
        return &g_BridgeLeases[0];
    }
    if (Direction == AR_BRIDGE_DIRECTION_CAPTURE_SINK) {
        return &g_BridgeLeases[1];
    }
    return NULL;
}

//-----------------------------------------------------------------------------
// Referenced forward.
//-----------------------------------------------------------------------------

DRIVER_ADD_DEVICE AddDevice;

NTSTATUS
StartDevice
(
    _In_  PDEVICE_OBJECT,
    _In_  PIRP,
    _In_  PRESOURCELIST
);

_Dispatch_type_(IRP_MJ_PNP)
DRIVER_DISPATCH PnpHandler;

_Dispatch_type_(IRP_MJ_CREATE)
_Dispatch_type_(IRP_MJ_CLOSE)
DRIVER_DISPATCH BridgeControlCreateClose;

_Dispatch_type_(IRP_MJ_DEVICE_CONTROL)
DRIVER_DISPATCH BridgeControlDeviceControl;

//
// Rendering streams are not saved to a file by default. Use the registry value
// DoNotCreateDataFiles (DWORD) = 0 to override this default.
//
DWORD g_DoNotCreateDataFiles = 1;  // default is off.
DWORD g_DisableToneGenerator = 0;  // default is to generate tones.
UNICODE_STRING g_RegistryPath;      // This is used to store the registry settings path for the driver

//-----------------------------------------------------------------------------
// Functions
//-----------------------------------------------------------------------------

static NTSTATUS CompleteBridgeIrp(_In_ PIRP Irp, _In_ NTSTATUS Status)
{
    Irp->IoStatus.Status = Status;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return Status;
}

NTSTATUS BridgeControlCreateClose(_In_ PDEVICE_OBJECT, _In_ PIRP Irp)
{
    return CompleteBridgeIrp(Irp, STATUS_SUCCESS);
}

NTSTATUS BridgeControlDeviceControl(_In_ PDEVICE_OBJECT, _In_ PIRP Irp)
{
    PIO_STACK_LOCATION stack = IoGetCurrentIrpStackLocation(Irp);
    ULONG code = stack->Parameters.DeviceIoControl.IoControlCode;
    NTSTATUS status = STATUS_INVALID_DEVICE_REQUEST;

    if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN ||
        code == IOCTL_AUDIOROUTER_BRIDGE_CLOSE ||
        code == IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT) {
        if (stack->Parameters.DeviceIoControl.InputBufferLength !=
            sizeof(AR_BRIDGE_OPEN_REQUEST) || Irp->AssociatedIrp.SystemBuffer == NULL) {
            return CompleteBridgeIrp(Irp, STATUS_INVALID_PARAMETER);
        }

        PAR_BRIDGE_OPEN_REQUEST request =
            static_cast<PAR_BRIDGE_OPEN_REQUEST>(Irp->AssociatedIrp.SystemBuffer);
        status = AudioRouterValidateBridgeOpenRequest(request);
        if (NT_SUCCESS(status)) {
            AR_BRIDGE_LEASE_STATE* lease = BridgeLeaseForDirection(request->Direction);
            if (lease == NULL) {
                return CompleteBridgeIrp(Irp, STATUS_INVALID_PARAMETER);
            }
            PVOID sectionObject = NULL;
            PVOID mappedView = NULL;
            SIZE_T mappedBytes = request->MappingBytes;
            // A section is acquired only for OPEN. CLOSE and HEARTBEAT are
            // lease operations and must validate the existing identity under
            // the lease lock without touching a user handle or mapping.
            if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN &&
                request->SectionHandle != 0) {
                SIZE_T requiredBytes = AR_BRIDGE_HEADER_BYTES +
                    static_cast<SIZE_T>(request->Channels) *
                    static_cast<SIZE_T>(request->FramesPerQuantum) * sizeof(float);
                if (mappedBytes < requiredBytes) {
                    status = STATUS_BUFFER_TOO_SMALL;
                } else {
                    status = ObReferenceObjectByHandle(
                        reinterpret_cast<HANDLE>(static_cast<ULONG_PTR>(request->SectionHandle)),
                        SECTION_MAP_READ | SECTION_MAP_WRITE, NULL, UserMode,
                        &sectionObject, NULL);
                    if (NT_SUCCESS(status)) {
                        status = MmMapViewInSystemSpace(
                            sectionObject, &mappedView, &mappedBytes);
                        if (NT_SUCCESS(status) && mappedBytes < requiredBytes) {
                            MmUnmapViewInSystemSpace(mappedView);
                            mappedView = NULL;
                            status = STATUS_BUFFER_TOO_SMALL;
                        }
                        if (!NT_SUCCESS(status)) {
                            ObDereferenceObject(sectionObject);
                            sectionObject = NULL;
                        }
                    }
                }
            }
            if (!NT_SUCCESS(status)) {
                if (mappedView != NULL) {
                    MmUnmapViewInSystemSpace(mappedView);
                }
                if (sectionObject != NULL) {
                    ObDereferenceObject(sectionObject);
                }
                return CompleteBridgeIrp(Irp, status);
            }

            PVOID oldSectionObject = NULL;
            PVOID oldMappedView = NULL;
            BOOLEAN oldRundownStarted = FALSE;
            BOOLEAN publishAfterRetire = FALSE;
            KIRQL oldIrql;
            KeAcquireSpinLock(&lease->Lock, &oldIrql);
            ULONGLONG now = KeQueryInterruptTime();
            ULONGLONG leaseTicks = static_cast<ULONGLONG>(request->LeaseMs) * _100NS_PER_MILLISECOND;
            BOOLEAN expired = lease->Active &&
                (now - lease->LastHeartbeat100ns > leaseTicks);

            if (code == IOCTL_AUDIOROUTER_BRIDGE_OPEN) {
                if ((lease->Active && !expired) || lease->Retiring) {
                    status = STATUS_DEVICE_BUSY;
                } else {
                    oldSectionObject = lease->SectionObject;
                    BOOLEAN priorRundownStarted = lease->RundownStarted;
                    oldMappedView = InterlockedExchangePointer(
                        &lease->MappedView, NULL);
                    oldRundownStarted = oldMappedView != NULL;
                    lease->RundownStarted = oldRundownStarted;
                    lease->MappedBytes = 0;
                    lease->SectionObject = NULL;
                    lease->Active = FALSE;
                    if (oldRundownStarted) {
                        lease->Retiring = TRUE;
                        publishAfterRetire = TRUE;
                    } else {
                        if (priorRundownStarted) {
                            ExReInitializeRundownProtection(&lease->Rundown);
                            lease->RundownStarted = FALSE;
                        }
                        lease->Request = *request;
                        lease->LastHeartbeat100ns = now;
                        lease->Active = TRUE;
                        lease->SectionObject = sectionObject;
                        lease->MappedView = mappedView;
                        lease->MappedBytes = static_cast<ULONG>(request->MappingBytes);
                        InterlockedExchange64(&lease->NextSequence, 0);
                        sectionObject = NULL;
                        mappedView = NULL;
                        status = STATUS_SUCCESS;
                    }
                }
            } else if (!lease->Active || expired || lease->Retiring ||
                       RtlCompareMemory(&lease->Request, request,
                                        sizeof(AR_BRIDGE_OPEN_REQUEST)) !=
                           sizeof(AR_BRIDGE_OPEN_REQUEST)) {
                // Expiry is terminal for the mapped callback view. Detach it
                // before returning the rejected maintenance request, then
                // wait for any callback reader before unmapping below.
                if (expired && lease->Active && !lease->Retiring) {
                    oldSectionObject = lease->SectionObject;
                    oldMappedView = InterlockedExchangePointer(
                        &lease->MappedView, NULL);
                    oldRundownStarted = oldMappedView != NULL;
                    lease->RundownStarted = oldRundownStarted;
                    lease->SectionObject = NULL;
                    lease->MappedBytes = 0;
                    lease->Active = FALSE;
                    lease->Retiring = oldRundownStarted;
                    if (!oldRundownStarted) {
                        RtlZeroMemory(&lease->Request, sizeof(lease->Request));
                    }
                }
                status = STATUS_INVALID_DEVICE_STATE;
            } else if (code == IOCTL_AUDIOROUTER_BRIDGE_CLOSE) {
                oldSectionObject = lease->SectionObject;
                oldMappedView = InterlockedExchangePointer(
                    &lease->MappedView, NULL);
                oldRundownStarted = oldMappedView != NULL;
                lease->RundownStarted = oldRundownStarted;
                lease->Retiring = oldRundownStarted;
                lease->Active = FALSE;
                lease->LastHeartbeat100ns = 0;
                lease->SectionObject = NULL;
                lease->MappedBytes = 0;
                status = STATUS_SUCCESS;
            } else {
                lease->LastHeartbeat100ns = now;
                status = STATUS_SUCCESS;
            }
            KeReleaseSpinLock(&lease->Lock, oldIrql);
            if (publishAfterRetire) {
                RetireBridgeResources(lease, oldMappedView, oldSectionObject,
                                      oldRundownStarted);
                oldMappedView = NULL;
                oldSectionObject = NULL;
                KeAcquireSpinLock(&lease->Lock, &oldIrql);
                ExReInitializeRundownProtection(&lease->Rundown);
                lease->RundownStarted = FALSE;
                lease->Request = *request;
                lease->LastHeartbeat100ns = now;
                lease->SectionObject = sectionObject;
                lease->MappedBytes = static_cast<ULONG>(request->MappingBytes);
                InterlockedExchange64(&lease->NextSequence, 0);
                KeMemoryBarrier();
                lease->MappedView = mappedView;
                lease->Active = TRUE;
                lease->Retiring = FALSE;
                KeReleaseSpinLock(&lease->Lock, oldIrql);
                sectionObject = NULL;
                mappedView = NULL;
                status = STATUS_SUCCESS;
            } else if (oldMappedView != NULL || oldSectionObject != NULL) {
                RetireBridgeResources(lease, oldMappedView, oldSectionObject,
                                      oldRundownStarted);
                if (lease->Retiring) {
                    KeAcquireSpinLock(&lease->Lock, &oldIrql);
                    RtlZeroMemory(&lease->Request, sizeof(lease->Request));
                    lease->Retiring = FALSE;
                    KeReleaseSpinLock(&lease->Lock, oldIrql);
                }
            }
            if (mappedView != NULL) {
                MmUnmapViewInSystemSpace(mappedView);
            }
            if (sectionObject != NULL) {
                ObDereferenceObject(sectionObject);
            }
        }
    }

    return CompleteBridgeIrp(Irp, status);
}

NTSTATUS CreateBridgeControlDevice(_In_ PDRIVER_OBJECT DriverObject)
{
    UNICODE_STRING deviceName;
    UNICODE_STRING dosName;
    RtlInitUnicodeString(&deviceName, AUDIOROUTER_BRIDGE_DEVICE_NAME);
    RtlInitUnicodeString(&dosName, AUDIOROUTER_BRIDGE_DOS_NAME);

    NTSTATUS status = IoCreateDeviceSecure(
        DriverObject, 0, &deviceName, FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN, FALSE,
        &AUDIOROUTER_BRIDGE_DEVICE_SDDL,
        &PID_AUDIOROUTERVIRTUAL, &g_BridgeControlDevice);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = IoCreateSymbolicLink(&dosName, &deviceName);
    if (!NT_SUCCESS(status)) {
        IoDeleteDevice(g_BridgeControlDevice);
        g_BridgeControlDevice = NULL;
        return status;
    }
    g_BridgeControlDevice->Flags &= ~DO_DEVICE_INITIALIZING;
    return STATUS_SUCCESS;
}

void DeleteBridgeControlDevice()
{
    UNICODE_STRING dosName;
    RtlInitUnicodeString(&dosName, AUDIOROUTER_BRIDGE_DOS_NAME);

    for (ULONG index = 0; index < AR_BRIDGE_LEASE_SLOTS; ++index) {
        PVOID sectionObject = NULL;
        PVOID mappedView = NULL;
        BOOLEAN rundownStarted = FALSE;
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_BridgeLeases[index].Lock, &oldIrql);
        sectionObject = g_BridgeLeases[index].SectionObject;
        mappedView = InterlockedExchangePointer(
            &g_BridgeLeases[index].MappedView, NULL);
        rundownStarted = mappedView != NULL;
        g_BridgeLeases[index].RundownStarted = rundownStarted;
        g_BridgeLeases[index].Retiring = rundownStarted;
        g_BridgeLeases[index].SectionObject = NULL;
        g_BridgeLeases[index].MappedView = NULL;
        g_BridgeLeases[index].MappedBytes = 0;
        g_BridgeLeases[index].Active = FALSE;
        RtlZeroMemory(&g_BridgeLeases[index].Request,
                      sizeof(g_BridgeLeases[index].Request));
        g_BridgeLeases[index].LastHeartbeat100ns = 0;
        KeReleaseSpinLock(&g_BridgeLeases[index].Lock, oldIrql);

        RetireBridgeResources(&g_BridgeLeases[index], mappedView,
                              sectionObject, rundownStarted);
    }
    if (g_BridgeControlDevice != NULL) {
        IoDeleteSymbolicLink(&dosName);
        IoDeleteDevice(g_BridgeControlDevice);
        g_BridgeControlDevice = NULL;
    }
}

#pragma code_seg("PAGE")
void ReleaseRegistryStringBuffer()
{
    PAGED_CODE();

    if (g_RegistryPath.Buffer != NULL)
    {
        ExFreePool(g_RegistryPath.Buffer);
        g_RegistryPath.Buffer = NULL;
        g_RegistryPath.Length = 0;
        g_RegistryPath.MaximumLength = 0;
    }
}

//=============================================================================
#pragma code_seg("PAGE")
extern "C"
void DriverUnload
(
    _In_ PDRIVER_OBJECT DriverObject
)
/*++

Routine Description:

  Our driver unload routine. This just frees the WDF driver object.

Arguments:

  DriverObject - pointer to the driver object

Environment:

    PASSIVE_LEVEL

--*/
{
    PAGED_CODE();

    DPF(D_TERSE, ("[DriverUnload]"));

    DeleteBridgeControlDevice();
    ReleaseRegistryStringBuffer();

    if (DriverObject == NULL)
    {
        goto Done;
    }

    //
    // Invoke first the port unload.
    //
    if (gPCDriverUnloadRoutine != NULL)
    {
        gPCDriverUnloadRoutine(DriverObject);
    }

    //
    // Unload WDF driver object.
    //
    if (WdfGetDriver() != NULL)
    {
        WdfDriverMiniportUnload(WdfGetDriver());
    }
Done:
    return;
}

//=============================================================================
#pragma code_seg("INIT")
__drv_requiresIRQL(PASSIVE_LEVEL)
NTSTATUS
CopyRegistrySettingsPath(
    _In_ PUNICODE_STRING RegistryPath
)
/*++

Routine Description:

Copies the following registry path to a global variable.

\REGISTRY\MACHINE\SYSTEM\ControlSetxxx\Services\<driver>\Parameters

Arguments:

RegistryPath - Registry path passed to DriverEntry

Returns:

NTSTATUS - SUCCESS if able to configure the framework

--*/

{
    // Initializing the unicode string, so that if it is not allocated it will not be deallocated too.
    RtlInitUnicodeString(&g_RegistryPath, NULL);

    g_RegistryPath.MaximumLength = RegistryPath->Length + sizeof(WCHAR);

    g_RegistryPath.Buffer = (PWCH)ExAllocatePool2(POOL_FLAG_PAGED, g_RegistryPath.MaximumLength, MINADAPTER_POOLTAG);

    if (g_RegistryPath.Buffer == NULL)
    {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    RtlAppendUnicodeToString(&g_RegistryPath, RegistryPath->Buffer);

    return STATUS_SUCCESS;
}

//=============================================================================
#pragma code_seg("INIT")
__drv_requiresIRQL(PASSIVE_LEVEL)
NTSTATUS
GetRegistrySettings(
    _In_ PUNICODE_STRING RegistryPath
   )
/*++

Routine Description:

    Initialize Driver Framework settings from the driver
    specific registry settings under

    \REGISTRY\MACHINE\SYSTEM\ControlSetxxx\Services\<driver>\Parameters

Arguments:

    RegistryPath - Registry path passed to DriverEntry

Returns:

    NTSTATUS - SUCCESS if able to configure the framework

--*/

{
    NTSTATUS                    ntStatus;
    PDRIVER_OBJECT              DriverObject;
    HANDLE                      DriverKey;
    RTL_QUERY_REGISTRY_TABLE    paramTable[] = {
    // QueryRoutine     Flags                                               Name                     EntryContext             DefaultType                                                    DefaultData              DefaultLength
        { NULL,   RTL_QUERY_REGISTRY_DIRECT | RTL_QUERY_REGISTRY_TYPECHECK, L"DoNotCreateDataFiles", &g_DoNotCreateDataFiles, (REG_DWORD << RTL_QUERY_REGISTRY_TYPECHECK_SHIFT) | REG_DWORD, &g_DoNotCreateDataFiles, sizeof(ULONG)},
        { NULL,   RTL_QUERY_REGISTRY_DIRECT | RTL_QUERY_REGISTRY_TYPECHECK, L"DisableToneGenerator", &g_DisableToneGenerator, (REG_DWORD << RTL_QUERY_REGISTRY_TYPECHECK_SHIFT) | REG_DWORD, &g_DisableToneGenerator, sizeof(ULONG)},
        { NULL,   0,                                                        NULL,                    NULL,                    0,                                                             NULL,                    0}
    };

    DPF(D_TERSE, ("[GetRegistrySettings]"));

    PAGED_CODE();
    UNREFERENCED_PARAMETER(RegistryPath);

    DriverObject = WdfDriverWdmGetDriverObject(WdfGetDriver());
    DriverKey = NULL;
    ntStatus = IoOpenDriverRegistryKey(DriverObject,
                                 DriverRegKeyParameters,
                                 KEY_READ,
                                 0,
                                 &DriverKey);

    if (!NT_SUCCESS(ntStatus))
    {
        return ntStatus;
    }

    ntStatus = RtlQueryRegistryValues(RTL_REGISTRY_HANDLE,
                                  (PCWSTR) DriverKey,
                                  &paramTable[0],
                                  NULL,
                                  NULL);

    if (!NT_SUCCESS(ntStatus))
    {
        DPF(D_VERBOSE, ("RtlQueryRegistryValues failed, using default values, 0x%x", ntStatus));
        //
        // Don't return error because we will operate with default values.
        //
    }

    //
    // Dump settings.
    //
    DPF(D_VERBOSE, ("DoNotCreateDataFiles: %u", g_DoNotCreateDataFiles));
    DPF(D_VERBOSE, ("DisableToneGenerator: %u", g_DisableToneGenerator));

    if (DriverKey)
    {
        ZwClose(DriverKey);
    }

    return STATUS_SUCCESS;
}

#pragma code_seg("INIT")
extern "C" DRIVER_INITIALIZE DriverEntry;
extern "C" NTSTATUS
DriverEntry
(
    _In_  PDRIVER_OBJECT          DriverObject,
    _In_  PUNICODE_STRING         RegistryPathName
)
{
/*++

Routine Description:

  Installable driver initialization entry point.
  This entry point is called directly by the I/O system.

  All audio adapter drivers can use this code without change.

Arguments:

  DriverObject - pointer to the driver object

  RegistryPath - pointer to a unicode string representing the path,
                   to driver-specific key in the registry.

Return Value:

  STATUS_SUCCESS if successful,
  STATUS_UNSUCCESSFUL otherwise.

--*/
    NTSTATUS                    ntStatus;
    WDF_DRIVER_CONFIG           config;

    DPF(D_TERSE, ("[DriverEntry]"));

    // Copy registry Path name in a global variable to be used by modules inside driver.
    // !! NOTE !! Inside this function we are initializing the registrypath, so we MUST NOT add any failing calls
    // before the following call.
    ntStatus = CopyRegistrySettingsPath(RegistryPathName);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("Registry path copy error 0x%x", ntStatus)),
        Done);

    WDF_DRIVER_CONFIG_INIT(&config, WDF_NO_EVENT_CALLBACK);
    for (ULONG index = 0; index < AR_BRIDGE_LEASE_SLOTS; ++index) {
        KeInitializeSpinLock(&g_BridgeLeases[index].Lock);
        ExInitializeRundownProtection(&g_BridgeLeases[index].Rundown);
    }
    //
    // Set WdfDriverInitNoDispatchOverride flag to tell the framework
    // not to provide dispatch routines for the driver. In other words,
    // the framework must not intercept IRPs that the I/O manager has
    // directed to the driver. In this case, they will be handled by Audio
    // port driver.
    //
    config.DriverInitFlags |= WdfDriverInitNoDispatchOverride;
    config.DriverPoolTag    = MINADAPTER_POOLTAG;

    ntStatus = WdfDriverCreate(DriverObject,
                               RegistryPathName,
                               WDF_NO_OBJECT_ATTRIBUTES,
                               &config,
                               WDF_NO_HANDLE);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("WdfDriverCreate failed, 0x%x", ntStatus)),
        Done);

    ntStatus = CreateBridgeControlDevice(DriverObject);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("CreateBridgeControlDevice failed, 0x%x", ntStatus)),
        Done);
    DriverObject->MajorFunction[IRP_MJ_CREATE] = BridgeControlCreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE] = BridgeControlCreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = BridgeControlDeviceControl;

    //
    // Get registry configuration.
    //
    ntStatus = GetRegistrySettings(RegistryPathName);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("Registry Configuration error 0x%x", ntStatus)),
        Done);

    //
    // Tell the class driver to initialize the driver.
    //
    ntStatus =  PcInitializeAdapterDriver(DriverObject,
                                          RegistryPathName,
                                          (PDRIVER_ADD_DEVICE)AddDevice);
    IF_FAILED_ACTION_JUMP(
        ntStatus,
        DPF(D_ERROR, ("PcInitializeAdapterDriver failed, 0x%x", ntStatus)),
        Done);

    //
    // To intercept stop/remove/surprise-remove.
    //
    DriverObject->MajorFunction[IRP_MJ_PNP] = PnpHandler;

    //
    // Hook the port class unload function
    //
    gPCDriverUnloadRoutine = DriverObject->DriverUnload;
    DriverObject->DriverUnload = DriverUnload;

    //
    // All done.
    //
    ntStatus = STATUS_SUCCESS;

Done:

    if (!NT_SUCCESS(ntStatus))
    {
        DeleteBridgeControlDevice();
        if (WdfGetDriver() != NULL)
        {
            WdfDriverMiniportUnload(WdfGetDriver());
        }

        ReleaseRegistryStringBuffer();
    }

    return ntStatus;
} // DriverEntry

#pragma code_seg()
// disable prefast warning 28152 because
// DO_DEVICE_INITIALIZING is cleared in PcAddAdapterDevice
#pragma warning(disable:28152)
#pragma code_seg("PAGE")
//=============================================================================
NTSTATUS AddDevice
(
    _In_  PDRIVER_OBJECT    DriverObject,
    _In_  PDEVICE_OBJECT    PhysicalDeviceObject
)
/*++

Routine Description:

  The Plug & Play subsystem is handing us a brand new PDO, for which we
  (by means of INF registration) have been asked to provide a driver.

  We need to determine if we need to be in the driver stack for the device.
  Create a function device object to attach to the stack
  Initialize that device object
  Return status success.

  All audio adapter drivers can use this code without change.

Arguments:

  DriverObject - pointer to a driver object

  PhysicalDeviceObject -  pointer to a device object created by the
                            underlying bus driver.

Return Value:

  NT status code.

--*/
{
    PAGED_CODE();

    NTSTATUS        ntStatus;
    ULONG           maxObjects;

    DPF(D_TERSE, ("[AddDevice]"));

    maxObjects = g_MaxMiniports;

    // Tell the class driver to add the device.
    //
    ntStatus =
        PcAddAdapterDevice
        (
            DriverObject,
            PhysicalDeviceObject,
            PCPFNSTARTDEVICE(StartDevice),
            maxObjects,
            0
        );

    return ntStatus;
} // AddDevice

#pragma code_seg()
NTSTATUS
_IRQL_requires_max_(DISPATCH_LEVEL)
PowerControlCallback
(
    _In_        LPCGUID PowerControlCode,
    _In_opt_    PVOID   InBuffer,
    _In_        SIZE_T  InBufferSize,
    _Out_writes_bytes_to_(OutBufferSize, *BytesReturned) PVOID OutBuffer,
    _In_        SIZE_T  OutBufferSize,
    _Out_opt_   PSIZE_T BytesReturned,
    _In_opt_    PVOID   Context
)
{
    UNREFERENCED_PARAMETER(PowerControlCode);
    UNREFERENCED_PARAMETER(InBuffer);
    UNREFERENCED_PARAMETER(InBufferSize);
    UNREFERENCED_PARAMETER(OutBuffer);
    UNREFERENCED_PARAMETER(OutBufferSize);
    UNREFERENCED_PARAMETER(BytesReturned);
    UNREFERENCED_PARAMETER(Context);

    return STATUS_NOT_IMPLEMENTED;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallEndpointRenderFilters(
    _In_ PDEVICE_OBJECT     _pDeviceObject,
    _In_ PIRP               _pIrp,
    _In_ PADAPTERCOMMON     _pAdapterCommon,
    _In_ PENDPOINT_MINIPAIR _pAeMiniports
    )
{
    NTSTATUS                    ntStatus                = STATUS_SUCCESS;
    PUNKNOWN                    unknownTopology         = NULL;
    PUNKNOWN                    unknownWave             = NULL;
    PPORTCLSETWHELPER           pPortClsEtwHelper       = NULL;
#ifdef _USE_IPortClsRuntimePower
    PPORTCLSRUNTIMEPOWER        pPortClsRuntimePower    = NULL;
#endif // _USE_IPortClsRuntimePower
    PPORTCLSStreamResourceManager pPortClsResMgr        = NULL;
    PPORTCLSStreamResourceManager2 pPortClsResMgr2      = NULL;

    PAGED_CODE();

    UNREFERENCED_PARAMETER(_pDeviceObject);

    ntStatus = _pAdapterCommon->InstallEndpointFilters(
        _pIrp,
        _pAeMiniports,
        NULL,
        &unknownTopology,
        &unknownWave,
        NULL, NULL);

    if (unknownWave) // IID_IPortClsEtwHelper and IID_IPortClsRuntimePower interfaces are only exposed on the WaveRT port.
    {
        ntStatus = unknownWave->QueryInterface (IID_IPortClsEtwHelper, (PVOID *)&pPortClsEtwHelper);
        if (NT_SUCCESS(ntStatus))
        {
            _pAdapterCommon->SetEtwHelper(pPortClsEtwHelper);
            ASSERT(pPortClsEtwHelper != NULL);
            pPortClsEtwHelper->Release();
        }

#ifdef _USE_IPortClsRuntimePower
        // Let's get the runtime power interface on PortCls.
        ntStatus = unknownWave->QueryInterface(IID_IPortClsRuntimePower, (PVOID *)&pPortClsRuntimePower);
        if (NT_SUCCESS(ntStatus))
        {
            // This interface would typically be stashed away for later use.  Instead,
            // let's just send an empty control with GUID_NULL.
            NTSTATUS ntStatusTest =
                pPortClsRuntimePower->SendPowerControl
                (
                    _pDeviceObject,
                    &GUID_NULL,
                    NULL,
                    0,
                    NULL,
                    0,
                    NULL
                );

            if (NT_SUCCESS(ntStatusTest) || STATUS_NOT_IMPLEMENTED == ntStatusTest || STATUS_NOT_SUPPORTED == ntStatusTest)
            {
                ntStatus = pPortClsRuntimePower->RegisterPowerControlCallback(_pDeviceObject, &PowerControlCallback, NULL);
                if (NT_SUCCESS(ntStatus))
                {
                    ntStatus = pPortClsRuntimePower->UnregisterPowerControlCallback(_pDeviceObject);
                }
            }
            else
            {
                ntStatus = ntStatusTest;
            }

            pPortClsRuntimePower->Release();
        }
#endif // _USE_IPortClsRuntimePower

        //
        // Test: add and remove current thread as streaming audio resource.
        // In a real driver you should only add interrupts and driver-owned threads
        // (i.e., do NOT add the current thread as streaming resource).
        //
        // testing IPortClsStreamResourceManager:
        ntStatus = unknownWave->QueryInterface(IID_IPortClsStreamResourceManager, (PVOID *)&pPortClsResMgr);
        if (NT_SUCCESS(ntStatus))
        {
            PCSTREAMRESOURCE_DESCRIPTOR res;
            PCSTREAMRESOURCE hRes = NULL;
            PDEVICE_OBJECT pdo = NULL;

            PcGetPhysicalDeviceObject(_pDeviceObject, &pdo);
            PCSTREAMRESOURCE_DESCRIPTOR_INIT(&res);
            res.Pdo = pdo;
            res.Type = ePcStreamResourceThread;
            res.Resource.Thread = PsGetCurrentThread();

            NTSTATUS ntStatusTest = pPortClsResMgr->AddStreamResource(NULL, &res, &hRes);
            if (NT_SUCCESS(ntStatusTest))
            {
                pPortClsResMgr->RemoveStreamResource(hRes);
                hRes = NULL;
            }

            pPortClsResMgr->Release();
            pPortClsResMgr = NULL;
        }

        // testing IPortClsStreamResourceManager2:
        ntStatus = unknownWave->QueryInterface(IID_IPortClsStreamResourceManager2, (PVOID *)&pPortClsResMgr2);
        if (NT_SUCCESS(ntStatus))
        {
            PCSTREAMRESOURCE_DESCRIPTOR res;
            PCSTREAMRESOURCE hRes = NULL;
            PDEVICE_OBJECT pdo = NULL;

            PcGetPhysicalDeviceObject(_pDeviceObject, &pdo);
            PCSTREAMRESOURCE_DESCRIPTOR_INIT(&res);
            res.Pdo = pdo;
            res.Type = ePcStreamResourceThread;
            res.Resource.Thread = PsGetCurrentThread();

            NTSTATUS ntStatusTest = pPortClsResMgr2->AddStreamResource2(pdo, NULL, &res, &hRes);
            if (NT_SUCCESS(ntStatusTest))
            {
                pPortClsResMgr2->RemoveStreamResource(hRes);
                hRes = NULL;
            }

            pPortClsResMgr2->Release();
            pPortClsResMgr2 = NULL;
        }
    }

    SAFE_RELEASE(unknownTopology);
    SAFE_RELEASE(unknownWave);

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallAllRenderFilters(
    _In_ PDEVICE_OBJECT _pDeviceObject,
    _In_ PIRP           _pIrp,
    _In_ PADAPTERCOMMON _pAdapterCommon
    )
{
    NTSTATUS            ntStatus;
    PENDPOINT_MINIPAIR* ppAeMiniports   = g_RenderEndpoints;

    PAGED_CODE();

    for(ULONG i = 0; i < g_cRenderEndpoints; ++i, ++ppAeMiniports)
    {
        ntStatus = InstallEndpointRenderFilters(_pDeviceObject, _pIrp, _pAdapterCommon, *ppAeMiniports);
        IF_FAILED_JUMP(ntStatus, Exit);
    }

    ntStatus = STATUS_SUCCESS;

Exit:
    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallEndpointCaptureFilters(
    _In_ PDEVICE_OBJECT     _pDeviceObject,
    _In_ PIRP               _pIrp,
    _In_ PADAPTERCOMMON     _pAdapterCommon,
    _In_ PENDPOINT_MINIPAIR _pAeMiniports
)
{
    NTSTATUS    ntStatus = STATUS_SUCCESS;

    PAGED_CODE();

    UNREFERENCED_PARAMETER(_pDeviceObject);

    ntStatus = _pAdapterCommon->InstallEndpointFilters(
        _pIrp,
        _pAeMiniports,
        NULL,
        NULL,
        NULL,
        NULL, NULL);

    return ntStatus;
}

#pragma code_seg("PAGE")
NTSTATUS
InstallAllCaptureFilters(
    _In_ PDEVICE_OBJECT _pDeviceObject,
    _In_ PIRP           _pIrp,
    _In_ PADAPTERCOMMON _pAdapterCommon
)
{
    NTSTATUS            ntStatus;
    PENDPOINT_MINIPAIR* ppAeMiniports = g_CaptureEndpoints;

    PAGED_CODE();

    for (ULONG i = 0; i < g_cCaptureEndpoints; ++i, ++ppAeMiniports)
    {
        ntStatus = InstallEndpointCaptureFilters(_pDeviceObject, _pIrp, _pAdapterCommon, *ppAeMiniports);
        IF_FAILED_JUMP(ntStatus, Exit);
    }

    ntStatus = STATUS_SUCCESS;

Exit:
    return ntStatus;
}

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
StartDevice
(
    _In_  PDEVICE_OBJECT          DeviceObject,
    _In_  PIRP                    Irp,
    _In_  PRESOURCELIST           ResourceList
)
{
/*++

Routine Description:

  This function is called by the operating system when the device is
  started.
  It is responsible for starting the miniports.  This code is specific to
  the adapter because it calls out miniports for functions that are specific
  to the adapter.

Arguments:

  DeviceObject - pointer to the driver object

  Irp - pointer to the irp

  ResourceList - pointer to the resource list assigned by PnP manager

Return Value:

  NT status code.

--*/
    UNREFERENCED_PARAMETER(ResourceList);

    PAGED_CODE();

    ASSERT(DeviceObject);
    ASSERT(Irp);
    ASSERT(ResourceList);

    NTSTATUS                    ntStatus        = STATUS_SUCCESS;

    PADAPTERCOMMON              pAdapterCommon  = NULL;
    PUNKNOWN                    pUnknownCommon  = NULL;
    PortClassDeviceContext*     pExtension      = static_cast<PortClassDeviceContext*>(DeviceObject->DeviceExtension);

    DPF_ENTER(("[StartDevice]"));

    //
    // create a new adapter common object
    //
    ntStatus = NewAdapterCommon(
                                &pUnknownCommon,
                                IID_IAdapterCommon,
                                NULL,
                                POOL_FLAG_NON_PAGED
                                );
    IF_FAILED_JUMP(ntStatus, Exit);

    ntStatus = pUnknownCommon->QueryInterface( IID_IAdapterCommon,(PVOID *) &pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

    ntStatus = pAdapterCommon->Init(DeviceObject);
    IF_FAILED_JUMP(ntStatus, Exit);

    //
    // register with PortCls for power-management services
    ntStatus = PcRegisterAdapterPowerManagement( PUNKNOWN(pAdapterCommon), DeviceObject);
    IF_FAILED_JUMP(ntStatus, Exit);

    //
    // Install wave+topology filters for render devices
    //
    ntStatus = InstallAllRenderFilters(DeviceObject, Irp, pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

    //
    // Install wave+topology filters for capture devices
    //
    ntStatus = InstallAllCaptureFilters(DeviceObject, Irp, pAdapterCommon);
    IF_FAILED_JUMP(ntStatus, Exit);

Exit:

    //
    // Stash the adapter common object in the device extension so
    // we can access it for cleanup on stop/removal.
    //
    if (pAdapterCommon)
    {
        ASSERT(pExtension != NULL);
        pExtension->m_pCommon = pAdapterCommon;
    }

    //
    // Release the adapter IUnknown interface.
    //
    SAFE_RELEASE(pUnknownCommon);

    return ntStatus;
} // StartDevice

//=============================================================================
#pragma code_seg("PAGE")
NTSTATUS
PnpHandler
(
    _In_ DEVICE_OBJECT *_DeviceObject,
    _Inout_ IRP *_Irp
)
/*++

Routine Description:

  Handles PnP IRPs

Arguments:

  _DeviceObject - Functional Device object pointer.

  _Irp - The Irp being passed

Return Value:

  NT status code.

--*/
{
    NTSTATUS                ntStatus = STATUS_UNSUCCESSFUL;
    IO_STACK_LOCATION      *stack;
    PortClassDeviceContext *ext;

    // Documented https://msdn.microsoft.com/en-us/library/windows/hardware/ff544039(v=vs.85).aspx
    // This method will be called in IRQL PASSIVE_LEVEL
#pragma warning(suppress: 28118)
    PAGED_CODE();

    ASSERT(_DeviceObject);
    ASSERT(_Irp);

    //
    // Check for the REMOVE_DEVICE irp.  If we're being unloaded,
    // uninstantiate our devices and release the adapter common
    // object.
    //
    stack = IoGetCurrentIrpStackLocation(_Irp);

    switch (stack->MinorFunction)
    {
    case IRP_MN_REMOVE_DEVICE:
    case IRP_MN_SURPRISE_REMOVAL:
    case IRP_MN_STOP_DEVICE:
        ext = static_cast<PortClassDeviceContext*>(_DeviceObject->DeviceExtension);

        if (ext->m_pCommon != NULL)
        {
            ext->m_pCommon->Cleanup();

            ext->m_pCommon->Release();
            ext->m_pCommon = NULL;
        }
        break;

    default:
        break;
    }

    ntStatus = PcDispatchIrp(_DeviceObject, _Irp);

    return ntStatus;
}

#pragma code_seg()
