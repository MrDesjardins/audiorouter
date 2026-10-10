<# Definitions only: inspect a PE file without loading or executing it. #>
function Get-VmToolImports {
    param([Parameter(Mandatory=$true)][string] $Path)
    if ((Get-Item -LiteralPath $Path).Length -gt 16MB) { throw 'VM diagnostic PE exceeds 16 MB.' }
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256 -or $bytes[0] -ne 77 -or $bytes[1] -ne 90) { throw 'Not a PE executable.' }
    $pe = [BitConverter]::ToUInt32($bytes,60)
    if ($pe -gt $bytes.Length - 256 -or [BitConverter]::ToUInt32($bytes,$pe) -ne 17744) { throw 'Invalid PE header.' }
    $sectionCount = [BitConverter]::ToUInt16($bytes,$pe+6)
    $optional = $pe + 24
    $optionalSize = [BitConverter]::ToUInt16($bytes,$pe+20)
    if ($sectionCount -lt 1 -or $sectionCount -gt 96 -or $optionalSize -lt 128 -or
        [BitConverter]::ToUInt16($bytes,$optional) -ne 523 -or [BitConverter]::ToUInt32($bytes,$optional+108) -lt 2) { throw 'Expected bounded PE32+ headers.' }
    $table = $optional + $optionalSize
    if ($table + $sectionCount * 40 -gt $bytes.Length) { throw 'Truncated PE section table.' }
    $sections = @()
    for ($i=0; $i -lt $sectionCount; $i++) {
        $offset = $table + $i*40
        $sections += @{ Rva=[long][BitConverter]::ToUInt32($bytes,$offset+12);
            Size=[long][BitConverter]::ToUInt32($bytes,$offset+16); Raw=[long][BitConverter]::ToUInt32($bytes,$offset+20) }
    }
    function Resolve-PeRva([long] $Rva, [long] $Length) {
        foreach ($section in $sections) {
            $delta = $Rva - $section.Rva
            if ($delta -ge 0 -and $delta + $Length -le $section.Size -and $section.Raw + $delta + $Length -le $bytes.Length) {
                return [int]($section.Raw + $delta)
            }
        }
        throw 'PE import RVA is outside file-backed sections.'
    }
    $importRva = [BitConverter]::ToUInt32($bytes,$optional+120)
    $importSize = [BitConverter]::ToUInt32($bytes,$optional+124)
    if ($importRva -eq 0 -or $importSize -lt 20 -or $importSize -gt 5120) { throw 'Missing or oversized PE import table.' }
    $imports = @()
    for ($i=0; $i -lt [Math]::Floor($importSize/20); $i++) {
        $offset = Resolve-PeRva ($importRva+$i*20) 20
        $nameRva = [BitConverter]::ToUInt32($bytes,$offset+12)
        if ($nameRva -eq 0) { return $imports }
        $name = ''
        for ($j=0; $j -lt 256; $j++) {
            $character = $bytes[(Resolve-PeRva ($nameRva+$j) 1)]
            if ($character -eq 0) { break }
            if ($character -lt 32 -or $character -gt 126) { throw 'Invalid DLL import name.' }
            $name += [char]$character
        }
        if ($j -eq 256 -or $name -notmatch '^[a-zA-Z0-9_.-]+\.dll$') { throw 'Invalid DLL import name.' }
        $imports += $name
    }
    throw 'Unterminated PE import table.'
}

function Assert-PortableVmTool {
    param([Parameter(Mandatory=$true)][string] $Path)
    $imports = @(Get-VmToolImports $Path)
    if ($imports.Count -eq 0) { throw 'VM tool imports are missing.' }
    $runtime = @($imports | Where-Object { $_ -match '^(vcruntime|msvcp|msvcr|concrt)\d.*\.dll$' })
    if ($runtime.Count -gt 0) { throw "VM tool requires a Visual C++ redistributable: $($runtime -join ', '). Build with +crt-static." }
    return $imports
}
