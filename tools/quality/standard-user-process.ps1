# Run only the verification process at Medium integrity. No account, registry,
# policy or UAC settings change. Elevated WebView2 ignores WEBVIEW2_* overrides.
[CmdletBinding()]
param(
    [string]$Executable = '',
    [string]$ArgumentsFile = '',
    [string]$Report = '',
    [switch]$InspectOnly,
    [ValidateRange(1, 3600)][int]$TimeoutSeconds = 1800
)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Security.AccessControl;
using System.Text;

public static class StandardUserProcess {
    [StructLayout(LayoutKind.Sequential)] struct SidAttributes { public IntPtr Sid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct StartupInfo {
        public int cb; public string reserved, desktop, title;
        public uint x, y, width, height, charsX, charsY, fill, flags;
        public ushort show, reservedSize; public IntPtr reservedPointer, input, output, error;
    }
    [StructLayout(LayoutKind.Sequential)] struct SecurityAttributes { public int length; public IntPtr descriptor; [MarshalAs(UnmanagedType.Bool)] public bool inherit; }
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr process, thread; public uint pid, tid; }
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll")] static extern IntPtr GetStdHandle(int id);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool DuplicateHandle(IntPtr sourceProcess, IntPtr source, IntPtr targetProcess, out IntPtr target, uint access, bool inherit, uint options);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern IntPtr CreateFile(string name, uint access, uint share, IntPtr attributes, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)] static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool TerminateProcess(IntPtr process, uint exitCode);
    [DllImport("kernel32.dll", SetLastError = true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool GetExitCodeProcess(IntPtr process, out uint code);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool GetTokenInformation(IntPtr token, int kind, IntPtr data, int size, out int required);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool CreateRestrictedToken(IntPtr existing, uint flags, uint disableCount, IntPtr disable, uint deleteCount, IntPtr delete, uint restrictCount, IntPtr restrict, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool SetTokenInformation(IntPtr token, int kind, ref SidAttributes data, int size);
    [DllImport("advapi32.dll", SetLastError = true, EntryPoint = "SetTokenInformation")] static extern bool SetDefaultDacl(IntPtr token, int kind, ref IntPtr data, int size);
    [DllImport("advapi32.dll", SetLastError = true)] static extern bool SetKernelObjectSecurity(IntPtr handle, uint information, byte[] descriptor);
    [DllImport("advapi32.dll", SetLastError = true)] static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);
    [DllImport("advapi32.dll", SetLastError = true)] static extern IntPtr GetSidSubAuthority(IntPtr sid, uint index);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool CreateProcessAsUser(IntPtr token, string application, StringBuilder command, IntPtr processAttributes, IntPtr threadAttributes, bool inherit, uint flags, IntPtr environment, string directory, ref StartupInfo startup, out ProcessInfo process);

    public sealed class Result {
        public int parentIntegrityRid, childIntegrityRid;
        public bool parentAdmin, childAdmin, restricted;
        public uint pid;
        public int exitCode;
    }
    static void Check(bool value, string action) { if (!value) { int code = Marshal.GetLastWin32Error(); throw new Win32Exception(code, action + " (" + code + "): " + new Win32Exception(code).Message); } }
    static int Integrity(IntPtr token) {
        int size;
        GetTokenInformation(token, 25, IntPtr.Zero, 0, out size); // TokenIntegrityLevel
        if (size <= 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Token integrity size");
        IntPtr buffer = Marshal.AllocHGlobal(size);
        try {
            Check(GetTokenInformation(token, 25, buffer, size, out size), "Token integrity");
            IntPtr sid = Marshal.ReadIntPtr(buffer);
            byte count = Marshal.ReadByte(GetSidSubAuthorityCount(sid));
            return Marshal.ReadInt32(GetSidSubAuthority(sid, (uint)(count - 1)));
        } finally { Marshal.FreeHGlobal(buffer); }
    }
    static bool Admin(IntPtr token) {
        using (var identity = new WindowsIdentity(token))
            return new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
    }
    public static Result Inspect() {
        IntPtr token;
        Check(OpenProcessToken(GetCurrentProcess(), 10, out token), "Open current token");
        try { return new Result { parentIntegrityRid = Integrity(token), parentAdmin = Admin(token) }; }
        finally { CloseHandle(token); }
    }
    static IntPtr InheritedStandardHandle(int id) {
        IntPtr source = GetStdHandle(id), owned = IntPtr.Zero, copy;
        try {
            if (source == IntPtr.Zero || source == new IntPtr(-1)) {
                owned = CreateFile("NUL", 0xC0000000, 3, IntPtr.Zero, 3, 0, IntPtr.Zero);
                Check(owned != new IntPtr(-1), "Open NUL"); source = owned;
            }
            Check(DuplicateHandle(GetCurrentProcess(), source, GetCurrentProcess(), out copy, 0, true, 2), "Duplicate standard handle");
            return copy;
        } finally { if (owned != IntPtr.Zero && owned != new IntPtr(-1)) CloseHandle(owned); }
    }
    public static Result Run(string executable, string arguments, int timeoutSeconds, string directory) {
        var result = Inspect();
        if (result.parentIntegrityRid < 8192) throw new InvalidOperationException("A restricted/Low parent is not a standard desktop session");
        IntPtr current = IntPtr.Zero, restricted = IntPtr.Zero, actualChild = IntPtr.Zero;
        IntPtr input = IntPtr.Zero, output = IntPtr.Zero, error = IntPtr.Zero, sidBuffer = IntPtr.Zero;
        IntPtr descriptorBuffer = IntPtr.Zero, attributesBuffer = IntPtr.Zero, aclBuffer = IntPtr.Zero;
        var process = new ProcessInfo(); bool finished = false, resumed = false;
        try {
            // Restricted handles retain source access rights. WRITE_DAC is used
            // only on the new token, never to modify this source token.
            Check(OpenProcessToken(GetCurrentProcess(), 0x4008B, out current), "Open source token");
            // DISABLE_MAX_PRIVILEGE | LUA_TOKEN removes administrative groups and
            // privileges. It never disables AppLocker or changes the caller token.
            Check(CreateRestrictedToken(current, 5, 0, IntPtr.Zero, 0, IntPtr.Zero, 0, IntPtr.Zero, out restricted), "Create limited token");
            var medium = new SecurityIdentifier("S-1-16-8192"); var bytes = new byte[medium.BinaryLength]; medium.GetBinaryForm(bytes, 0);
            sidBuffer = Marshal.AllocHGlobal(bytes.Length); Marshal.Copy(bytes, 0, sidBuffer, bytes.Length);
            var label = new SidAttributes { Sid = sidBuffer, Attributes = 0x20 }; // SE_GROUP_INTEGRITY
            Check(SetTokenInformation(restricted, 25, ref label, Marshal.SizeOf(typeof(SidAttributes)) + bytes.Length), "Set Medium integrity");
            // A High creator's default descriptors can name only Administrators.
            // Grant this same user access to its own derived token and process/
            // thread objects; never change the caller or any machine object.
            string userSid;
            using (var identity = new WindowsIdentity(current)) userSid = identity.User.Value;
            var descriptor = new RawSecurityDescriptor("O:" + userSid + "D:(A;;GA;;;SY)(A;;GA;;;" + userSid + ")S:(ML;;NW;;;ME)");
            var security = new byte[descriptor.BinaryLength]; descriptor.GetBinaryForm(security, 0);
            Check(SetKernelObjectSecurity(restricted, 4, security), "Set derived token object DACL");
            var acl = new byte[descriptor.DiscretionaryAcl.BinaryLength]; descriptor.DiscretionaryAcl.GetBinaryForm(acl, 0);
            aclBuffer = Marshal.AllocHGlobal(acl.Length); Marshal.Copy(acl, 0, aclBuffer, acl.Length);
            Check(SetDefaultDacl(restricted, 6, ref aclBuffer, IntPtr.Size), "Set derived token default DACL");
            descriptorBuffer = Marshal.AllocHGlobal(security.Length); Marshal.Copy(security, 0, descriptorBuffer, security.Length);
            var attributes = new SecurityAttributes { length = Marshal.SizeOf(typeof(SecurityAttributes)), descriptor = descriptorBuffer, inherit = false };
            attributesBuffer = Marshal.AllocHGlobal(attributes.length); Marshal.StructureToPtr(attributes, attributesBuffer, false);
            if (Integrity(restricted) != 8192 || Admin(restricted)) throw new InvalidOperationException("Limited token still has administrative authority");
            input = InheritedStandardHandle(-10); output = InheritedStandardHandle(-11); error = InheritedStandardHandle(-12);
            var startup = new StartupInfo { cb = Marshal.SizeOf(typeof(StartupInfo)), desktop = "winsta0\\default", flags = 0x101, show = 0, input = input, output = output, error = error };
            // Same identity/session/environment and job supervision; no alternate
            // credentials. Suspended creation lets us check the actual token first.
            var command = new StringBuilder("\"" + executable + "\" " + arguments);
            Check(CreateProcessAsUser(restricted, executable, command, attributesBuffer, attributesBuffer, true, 4, IntPtr.Zero, directory, ref startup, out process), "Create standard-user process");
            result.pid = process.pid; result.restricted = true;
            Check(OpenProcessToken(process.process, 10, out actualChild), "Read limited child token");
            result.childIntegrityRid = Integrity(actualChild); result.childAdmin = Admin(actualChild);
            if (result.childIntegrityRid != 8192 || result.childAdmin) throw new InvalidOperationException("Created child is not Medium/non-admin");
            Check(ResumeThread(process.thread) != UInt32.MaxValue, "Resume standard-user process");
            resumed = true;
            uint wait = WaitForSingleObject(process.process, (uint)timeoutSeconds * 1000);
            if (wait == 258) throw new TimeoutException("Standard-user verification timed out");
            Check(wait == 0, "Wait for standard-user process");
            uint code; Check(GetExitCodeProcess(process.process, out code), "Read child exit code");
            result.exitCode = unchecked((int)code); finished = true; return result;
        } finally {
            // All handles/buffers are owned here; child token checks borrow them.
            // A failed suspended launch must never leave an owned process alive.
            if (!finished && process.process != IntPtr.Zero) {
                if (resumed) { try { KillTree(process.pid); } catch { } }
                TerminateProcess(process.process, 1);
            }
            foreach (IntPtr handle in new[] { actualChild, process.thread, process.process, input, output, error, restricted, current })
                if (handle != IntPtr.Zero) CloseHandle(handle);
            foreach (IntPtr buffer in new[] { sidBuffer, descriptorBuffer, attributesBuffer, aclBuffer })
                if (buffer != IntPtr.Zero) Marshal.FreeHGlobal(buffer);
        }
    }
    static void KillTree(uint pid) {
        using (var kill = Process.Start(new ProcessStartInfo(System.IO.Path.Combine(Environment.SystemDirectory, "taskkill.exe"), "/PID " + pid + " /T /F") { UseShellExecute = false, CreateNoWindow = true, WindowStyle = ProcessWindowStyle.Hidden })) kill.WaitForExit();
    }
}
'@
if ($InspectOnly) { [StandardUserProcess]::Inspect() | ConvertTo-Json -Compress; exit 0 }
if (-not $Executable -or -not $ArgumentsFile -or -not $Report) { throw 'Executable, ArgumentsFile and Report are required' }
$launchArguments = [IO.File]::ReadAllText($ArgumentsFile, (New-Object Text.UTF8Encoding($false, $true))) | ConvertFrom-Json
if (@($launchArguments | Where-Object { $_ -isnot [string] -or $_.Contains('"') -or $_.Contains("`n") -or $_.Contains("`r") -or $_.EndsWith('\') }).Count) { throw 'Invalid literal verification arguments' }
$quoted = @($launchArguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
try {
    $result = [StandardUserProcess]::Run((Resolve-Path -LiteralPath $Executable).Path, $quoted, $TimeoutSeconds, (Get-Location).Path)
    $result | ConvertTo-Json | Set-Content -LiteralPath $Report -Encoding UTF8
    exit $result.exitCode
} catch {
    [pscustomobject]@{ parent = [StandardUserProcess]::Inspect(); error = $_.Exception.Message } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Report -Encoding UTF8
    throw
}
