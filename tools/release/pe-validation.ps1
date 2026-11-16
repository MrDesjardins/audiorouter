function Assert-PortableExecutableArchitecture {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][ValidateSet('x64', 'x86')][string]$Architecture
    )

    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if ($item.PSIsContainer -or (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
        throw "release executable must be a regular non-reparse file: $Path"
    }
    if ($item.Length -lt 64) {
        throw "release executable is too small to contain a PE header: $Path"
    }

    $stream = [IO.File]::OpenRead($Path)
    try {
        $dosHeader = [byte[]]::new(64)
        if ($stream.Read($dosHeader, 0, $dosHeader.Length) -ne $dosHeader.Length -or
            $dosHeader[0] -ne 0x4d -or $dosHeader[1] -ne 0x5a) {
            throw "release executable does not have a valid DOS header: $Path"
        }

        $peOffset = [BitConverter]::ToInt32($dosHeader, 0x3c)
        if ($peOffset -lt 64 -or $peOffset -gt ($item.Length - 6)) {
            throw "release executable has an out-of-range PE header offset: $Path"
        }
        $stream.Position = $peOffset
        $coffPrefix = [byte[]]::new(6)
        if ($stream.Read($coffPrefix, 0, $coffPrefix.Length) -ne $coffPrefix.Length -or
            $coffPrefix[0] -ne 0x50 -or $coffPrefix[1] -ne 0x45 -or
            $coffPrefix[2] -ne 0 -or $coffPrefix[3] -ne 0) {
            throw "release executable does not have a valid PE signature: $Path"
        }
        $machine = [BitConverter]::ToUInt16($coffPrefix, 4)
        $expectedMachine = if ($Architecture -eq 'x86') { 0x014c } else { 0x8664 }
        if ($machine -ne $expectedMachine) {
            throw "release executable is not $Architecture (machine 0x$('{0:X4}' -f $machine)): $Path"
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Assert-X64PortableExecutable {
    param([Parameter(Mandatory = $true)][string]$Path)
    Assert-PortableExecutableArchitecture -Path $Path -Architecture x64
}
