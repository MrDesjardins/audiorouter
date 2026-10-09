// Read-only COM snapshot: no stream activation or setter is exposed.
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
namespace AudioRouterVmEvidence {
    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
    class MMDeviceEnumerator { }
    [ComImport, Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IDeviceEnumerator {
        [PreserveSig] int EnumAudioEndpoints(int flow, uint stateMask, out IDeviceCollection devices);
        [PreserveSig] int GetDefaultAudioEndpoint(int flow, int role, out IDevice device);
        [PreserveSig] int GetDevice([MarshalAs(UnmanagedType.LPWStr)] string id, out IDevice device);
        [PreserveSig] int RegisterEndpointNotificationCallback(IntPtr callback);
        [PreserveSig] int UnregisterEndpointNotificationCallback(IntPtr callback);
    }
    [ComImport, Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IDevice {
        [PreserveSig] int Activate(ref Guid iid, uint context, IntPtr parameters, out IntPtr result);
        [PreserveSig] int OpenPropertyStore(uint access, out IntPtr properties);
        [PreserveSig] int GetId([MarshalAs(UnmanagedType.LPWStr)] out string id);
        [PreserveSig] int GetState(out uint state);
    }
    [ComImport, Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IDeviceCollection {
        [PreserveSig] int GetCount(out uint count);
        [PreserveSig] int Item(uint index, out IDevice device);
    }
    public sealed class EndpointState {
        public string Id { get; set; }
        public int Flow { get; set; }
        public uint State { get; set; }
    }
    public static class EndpointStates {
        // SDK IMMDeviceCollection ABI; enumerate all states (active=1,
        // disabled=2, not-present=4, unplugged=8) without activating streams.
        // Each device, collection and enumerator COM reference is released.
        public static EndpointState[] Read() {
            var result = new List<EndpointState>();
            var enumerator = (IDeviceEnumerator)new MMDeviceEnumerator();
            try {
                for (int flow = 0; flow < 2; flow++) {
                    IDeviceCollection devices = null;
                    try {
                        Marshal.ThrowExceptionForHR(enumerator.EnumAudioEndpoints(flow, 0xF, out devices));
                        uint count;
                        Marshal.ThrowExceptionForHR(devices.GetCount(out count));
                        for (uint index = 0; index < count; index++) {
                            IDevice device = null;
                            try {
                                Marshal.ThrowExceptionForHR(devices.Item(index, out device));
                                string id;
                                uint state;
                                Marshal.ThrowExceptionForHR(device.GetId(out id));
                                Marshal.ThrowExceptionForHR(device.GetState(out state));
                                result.Add(new EndpointState { Id = id, Flow = flow, State = state });
                            } finally { if (device != null) Marshal.ReleaseComObject(device); }
                        }
                    } finally { if (devices != null) Marshal.ReleaseComObject(devices); }
                }
                return result.ToArray();
            } finally { Marshal.ReleaseComObject(enumerator); }
        }
    }
    public static class DefaultEndpoints {
        public static string[] Read() {
            var result = new string[6];
            var enumerator = (IDeviceEnumerator)new MMDeviceEnumerator();
            try {
                for (int flow = 0; flow < 2; flow++) for (int role = 0; role < 3; role++) {
                    IDevice device;
                    int hr = enumerator.GetDefaultAudioEndpoint(flow, role, out device);
                    string id = "<none>";
                    try {
                        if (hr == 0) { Marshal.ThrowExceptionForHR(device.GetId(out id)); }
                        else if (hr != unchecked((int)0x80070490)) { Marshal.ThrowExceptionForHR(hr); }
                        result[flow * 3 + role] = flow + ":" + role + "=" + id;
                    } finally { if (device != null) Marshal.ReleaseComObject(device); }
                }
                return result;
            } finally { Marshal.ReleaseComObject(enumerator); }
        }
    }
}
