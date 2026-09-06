param(
    [string]$Output = (Join-Path $PSScriptRoot 'main.exe')
)

$ErrorActionPreference = 'Stop'
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw "Visual Studio discovery tool is missing: $vswhere"
}
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) {
    throw 'No Visual Studio installation with the native C++ workload was found'
}
$msvcRoot = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $msvcRoot) { throw "MSVC tools are missing from $installation" }
$cl = Join-Path $msvcRoot.FullName 'bin\Hostx64\x64\cl.exe'
$vcInclude = Join-Path $msvcRoot.FullName 'include'
$vcLib = Join-Path $msvcRoot.FullName 'lib\x64'
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$kitRoot = Get-ChildItem -LiteralPath (Join-Path $kits 'Include') -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h') } |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $kitRoot) { throw "Windows SDK headers are missing from $kits" }
$version = $kitRoot.Name
$include = $kitRoot.FullName
$umLib = Join-Path $kits "Lib\$version\um\x64"
$ucrtLib = Join-Path $kits "Lib\$version\ucrt\x64"
$source = Join-Path $PSScriptRoot 'main.cpp'
$output = [System.IO.Path]::GetFullPath($Output)
$object = Join-Path $PSScriptRoot 'main.obj'

foreach ($path in @($cl, $vcInclude, $vcLib, "$include\um\Windows.h", "$include\um\audioclientactivationparams.h", "$umLib\Mmdevapi.lib")) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required native toolchain path is missing: $path"
    }
}

& $cl /nologo /EHsc /std:c++20 `
    "/I$vcInclude" "/I$include\shared" "/I$include\um" "/I$include\ucrt" "/I$include\winrt" `
    $source /Fo:$object /Fe:$output /link "/LIBPATH:$vcLib" "/LIBPATH:$umLib" "/LIBPATH:$ucrtLib" `
    ole32.lib uuid.lib avrt.lib Mmdevapi.lib
if ($LASTEXITCODE -ne 0) {
    throw "Native WASAPI probe build failed with exit code $LASTEXITCODE"
}
