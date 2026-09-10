$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-release-prepare-' + [guid]::NewGuid().ToString('N'))
$prepare = Join-Path $PSScriptRoot 'prepare-artifacts.ps1'
try {
    New-Item -ItemType Directory -Path $root | Out-Null
    . (Join-Path $PSScriptRoot "pe-validation.ps1")

    function New-TestPe {
        param([Parameter(Mandatory = $true)][string]$Path, [Parameter(Mandatory = $true)][uint16]$Machine)
        $bytes = [byte[]]::new(70)
        $bytes[0] = 0x4d
        $bytes[1] = 0x5a
        [BitConverter]::GetBytes([int32]64).CopyTo($bytes, 0x3c)
        $bytes[64] = 0x50
        $bytes[65] = 0x45
        $bytes[68] = [byte]($Machine -band 0xff)
        $bytes[69] = [byte](($Machine -shr 8) -band 0xff)
        [IO.File]::WriteAllBytes($Path, $bytes)
    }

    $validPe = Join-Path $root 'valid-x64.bin'
    New-TestPe $validPe 0x8664
    Assert-X64PortableExecutable $validPe

    $wrongMachine = Join-Path $root 'wrong-machine.bin'
    New-TestPe $wrongMachine 0x014c
    try {
        Assert-X64PortableExecutable $wrongMachine
        throw 'PE validator accepted a non-x64 machine type'
    } catch {
        if ($_.Exception.Message -eq 'PE validator accepted a non-x64 machine type') { throw }
        if ($_.Exception.Message -notmatch 'not x64') { throw }
    }

    $malformedPe = Join-Path $root 'malformed-pe.bin'
    [IO.File]::WriteAllBytes($malformedPe, [byte[]](0x4d, 0x5a, 0, 0))
    try {
        Assert-X64PortableExecutable $malformedPe
        throw 'PE validator accepted a truncated header'
    } catch {
        if ($_.Exception.Message -eq 'PE validator accepted a truncated header') { throw }
        if ($_.Exception.Message -notmatch 'too small') { throw }
    }

    $missing = Join-Path $root 'missing\output'
    try {
        & $prepare -OutputDirectory $missing | Out-Null
        throw 'prepare script accepted an output directory with a missing parent'
    } catch {
        if ($_.Exception.Message -eq 'prepare script accepted an output directory with a missing parent') { throw }
        if ($_.Exception.Message -notmatch 'parent must already exist') { throw }
    }

    $target = Join-Path $root 'target'
    New-Item -ItemType Directory -Path $target | Out-Null
    $link = Join-Path $root 'redirected-parent'
    try {
        New-Item -ItemType SymbolicLink -Path $link -Target $target -ErrorAction Stop | Out-Null
        try {
            & $prepare -OutputDirectory (Join-Path $link 'output') | Out-Null
            throw 'prepare script accepted a reparse-point output parent'
        } catch {
            if ($_.Exception.Message -eq 'prepare script accepted a reparse-point output parent') { throw }
            if ($_.Exception.Message -notmatch 'parent must not be a reparse point') { throw }
        }
    } catch {
        if ($_.Exception.Message -notmatch 'privilege|symbolic|not permitted|cannot create') { throw }
    }

    Write-Output 'Release preparation path-safety tests passed'
}
finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
