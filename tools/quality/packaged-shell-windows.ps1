# Windows-only helpers for the smoke qualifier. No machine settings change.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Snapshot', 'Alive', 'Cleanup', 'Break', 'Launch')][string] $Operation,
    [ValidateRange(1, 2147483647)][int] $RootPid = 1,
    [string] $SnapshotFile = '',
    [string] $Executable = '',
    [string] $ArgumentsFile = '',
    [string] $StdoutFile = '',
    [string] $StderrFile = ''
)
$ErrorActionPreference = 'Stop'

if ($Operation -eq 'Launch') {
    $launchArguments = [IO.File]::ReadAllText($ArgumentsFile, (New-Object Text.UTF8Encoding($false, $true))) | ConvertFrom-Json
    if (@($launchArguments | Where-Object { $_ -isnot [string] -or $_.Contains('"') -or $_.Contains("`n") -or $_.EndsWith('\') }).Count) {
        throw 'Invalid literal runner arguments'
    }
    # Start-Process creates a console for this console executable. Hide that
    # console while retaining it for real CTRL_BREAK delivery. Arguments are
    # literal strings passed to node.exe; no command shell interprets them.
    $quoted = @($launchArguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $launched = Start-Process -FilePath $Executable -ArgumentList $quoted -WorkingDirectory (Get-Location).Path -WindowStyle Hidden -RedirectStandardOutput $StdoutFile -RedirectStandardError $StderrFile -PassThru
    # Cache the process handle before exit. Windows PowerShell can otherwise
    # lose the exit code of a Start-Process result and turn cancellation into 0.
    $ownedHandle = $launched.Handle
    $launched.WaitForExit()
    $code = $launched.ExitCode
    if ($null -eq $code) { throw 'Owned runner exit code is unavailable' }
    [GC]::KeepAlive($ownedHandle)
    $launched.Dispose()
    exit ([int]$code)
}

if ($Operation -eq 'Break') {
    # The qualifier starts its own Node runner in a new, hidden console.
    # Attach only this helper to that console, then deliver a real CTRL_BREAK.
    $target = Get-Process -Id $RootPid
    if ($target.ProcessName -ne 'node') { throw 'Console-break target must be the owned Node runner' }
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PackagedSmokeConsole {
    public delegate bool Handler(uint eventType);
    private static bool Ignore(uint eventType) { return true; }
    public static readonly Handler IgnoreHandler = Ignore;
    [DllImport("kernel32.dll", SetLastError = true)] public static extern bool FreeConsole();
    [DllImport("kernel32.dll", SetLastError = true)] public static extern bool AttachConsole(uint pid);
    [DllImport("kernel32.dll", SetLastError = true)] public static extern bool SetConsoleCtrlHandler(Handler handler, bool add);
    [DllImport("kernel32.dll", SetLastError = true)] public static extern bool GenerateConsoleCtrlEvent(uint eventType, uint groupId);
}
'@
    [void][PackagedSmokeConsole]::FreeConsole()
    if (-not [PackagedSmokeConsole]::AttachConsole([uint32]$RootPid)) {
        throw ('AttachConsole failed: ' + [Runtime.InteropServices.Marshal]::GetLastWin32Error())
    }
    try {
        $handler = [PackagedSmokeConsole]::IgnoreHandler
        if (-not [PackagedSmokeConsole]::SetConsoleCtrlHandler($handler, $true)) { throw 'Could not protect the console helper' }
        if (-not [PackagedSmokeConsole]::GenerateConsoleCtrlEvent(1, 0)) {
            throw ('GenerateConsoleCtrlEvent failed: ' + [Runtime.InteropServices.Marshal]::GetLastWin32Error())
        }
        Start-Sleep -Milliseconds 200
        [GC]::KeepAlive($handler)
    }
    finally { [void][PackagedSmokeConsole]::FreeConsole() }
    Write-Output '{"signal":"CTRL_BREAK_EVENT"}'
    exit 0
}

$processes = @(Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, CreationDate)
if ($Operation -eq 'Snapshot') {
    $wanted = [Collections.Generic.HashSet[uint32]]::new()
    $byPid = @{}
    foreach ($item in $processes) { $byPid[[uint32]$item.ProcessId] = $item }
    [void]$wanted.Add([uint32]$RootPid)
    do {
        $added = $false
        foreach ($item in $processes) {
            $parent = $byPid[[uint32]$item.ParentProcessId]
            # An orphan's old ParentProcessId can have been recycled. A child
            # cannot belong to a newer parent, even when the numeric PID matches.
            if ($wanted.Contains([uint32]$item.ParentProcessId) -and $null -ne $parent.CreationDate -and $null -ne $item.CreationDate -and $item.CreationDate -ge $parent.CreationDate -and $wanted.Add([uint32]$item.ProcessId)) { $added = $true }
        }
    } while ($added)
    $found = @($processes | Where-Object { $wanted.Contains([uint32]$_.ProcessId) } | ForEach-Object {
        @{ pid = [int]$_.ProcessId; created = $_.CreationDate.ToUniversalTime().ToString('o') }
    })
}
else {
    if (-not $SnapshotFile) { throw 'A snapshot file is required' }
    $saved = [IO.File]::ReadAllText($SnapshotFile, (New-Object Text.UTF8Encoding($false, $true))) | ConvertFrom-Json
    # Match creation time as well as PID so a recycled PID is never treated as
    # an owned process. Only the qualifier's captured descendants are eligible.
    $found = @($processes | ForEach-Object {
        $item = $_
        if ($null -eq $item.CreationDate) { return }
        $created = $item.CreationDate.ToUniversalTime().ToString('o')
        if (@($saved | Where-Object { $_.pid -eq [int]$item.ProcessId -and $_.created -eq $created }).Count -gt 0) {
            if ($Operation -eq 'Cleanup') { Stop-Process -Id $item.ProcessId -Force -ErrorAction SilentlyContinue }
            @{ pid = [int]$item.ProcessId; created = $created }
        }
    })
}
ConvertTo-Json -InputObject @($found) -Compress
