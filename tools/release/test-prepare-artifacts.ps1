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

    # A bounded PE import-table fixture proves the release import guard accepts
    # OS DLLs and rejects a VC runtime dependency without executing a binary.
    . (Join-Path $PSScriptRoot '../vm/portable-tool-support.ps1')
    function New-ImportFixture([string]$Path, [string]$ImportName) {
        $bytes = [byte[]]::new(1024)
        $bytes[0] = 77; $bytes[1] = 90
        [BitConverter]::GetBytes([int32]128).CopyTo($bytes, 60)
        [BitConverter]::GetBytes([int32]17744).CopyTo($bytes, 128)
        [BitConverter]::GetBytes([uint16]1).CopyTo($bytes, 134)
        [BitConverter]::GetBytes([uint16]240).CopyTo($bytes, 148)
        $optional = 152
        [BitConverter]::GetBytes([uint16]523).CopyTo($bytes, $optional)
        [BitConverter]::GetBytes([int32]16).CopyTo($bytes, $optional + 108)
        [BitConverter]::GetBytes([int32]4096).CopyTo($bytes, $optional + 120)
        [BitConverter]::GetBytes([int32]40).CopyTo($bytes, $optional + 124)
        $section = $optional + 240
        [BitConverter]::GetBytes([int32]4096).CopyTo($bytes, $section + 12)
        [BitConverter]::GetBytes([int32]512).CopyTo($bytes, $section + 16)
        [BitConverter]::GetBytes([int32]512).CopyTo($bytes, $section + 20)
        [BitConverter]::GetBytes([int32]4160).CopyTo($bytes, 524)
        [Text.Encoding]::ASCII.GetBytes($ImportName).CopyTo($bytes, 576)
        [IO.File]::WriteAllBytes($Path, $bytes)
    }
    $portable = Join-Path $root 'portable-imports.exe'
    New-ImportFixture $portable 'kernel32.dll'
    $null = @(Assert-PortableVmTool $portable)
    $dependent = Join-Path $root 'runtime-imports.exe'
    New-ImportFixture $dependent 'VCRUNTIME140.dll'
    try {
        $null = @(Assert-PortableVmTool $dependent)
        throw 'import guard accepted the VC runtime'
    } catch {
        if ($_.Exception.Message -notmatch 'requires a Visual C\+\+ redistributable') { throw }
    }
    $missing = Join-Path $root 'missing\output'
    try {
        & $prepare -OutputDirectory $missing -DriverPackage $root | Out-Null
        throw 'prepare script accepted an output directory with a missing parent'
    } catch {
        if ($_.Exception.Message -eq 'prepare script accepted an output directory with a missing parent') { throw }
        if ($_.Exception.Message -notmatch 'parent must already exist') { throw }
    }

    $target = Join-Path $root 'target'
    New-Item -ItemType Directory -Path $target | Out-Null
    $missingPayloadOutput = Join-Path $root 'missing-payload-output'
    try {
        & $prepare -OutputDirectory $missingPayloadOutput -DriverPackage $root | Out-Null
        throw 'prepare script accepted a signed driver directory without the catalog payload'
    } catch {
        if ($_.Exception.Message -eq 'prepare script accepted a signed driver directory without the catalog payload') { throw }
        if ($_.Exception.Message -notmatch 'missing a regular audioroutervirtual\.inf') { throw }
    }
    if (Test-Path -LiteralPath $missingPayloadOutput) { throw 'prepare script created output before rejecting the missing driver payload' }

    $link = Join-Path $root 'redirected-parent'
    try {
        New-Item -ItemType SymbolicLink -Path $link -Target $target -ErrorAction Stop | Out-Null
        try {
            & $prepare -OutputDirectory (Join-Path $link 'output') -DriverPackage $root | Out-Null
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
