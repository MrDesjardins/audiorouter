# M03 Software Device API probe

This is the native provisioning seam for the AudioRouter-owned virtual-driver
package. It uses the Windows Software Device API to describe a stable
`SWD\AudioRouterVirtual` hardware ID and a caller-selected instance ID. The
default invocation is a no-side-effect dry run:

```powershell
.\build.ps1 -Output .\temp\m03-swdevice-probe.exe
.\temp\m03-swdevice-probe.exe
```

Temporary creation is separately guarded and is not part of normal acceptance:

```powershell
.\temp\m03-swdevice-probe.exe --create --allow-device-create --instance test-bus --hold-ms 1000
```

The probe closes its handle before exit, so the default handle lifetime removes
the temporary software device. Production integration still needs a
control-plane owner that retains handles, persists the returned PnP instance
IDs, applies enable/disable/delete semantics, and performs rollback on partial
failure. Do not run the create form on the daily workstation; use an isolated
test target after driver package/signing approval.
