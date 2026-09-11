param(
    [string]$SdkInclude = (Join-Path $PSScriptRoot '..\..\third_party\vst3sdk'),
    [string]$Output = (Join-Path $PSScriptRoot 'm06-vst3-worker.exe')
)

$ErrorActionPreference = 'Stop'
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) { throw 'No Visual Studio installation with the native C++ workload was found' }
$msvcRoot = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1
$cl = Join-Path $msvcRoot.FullName 'bin\Hostx64\x64\cl.exe'
$vcInclude = Join-Path $msvcRoot.FullName 'include'
$vcLib = Join-Path $msvcRoot.FullName 'lib\x64'
$sdkInclude = [System.IO.Path]::GetFullPath($SdkInclude)
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$kitRoot = Get-ChildItem -LiteralPath (Join-Path $kits 'Include') -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h') } |
    Sort-Object Name -Descending | Select-Object -First 1
$include = $kitRoot.FullName
$version = $kitRoot.Name
$umLib = Join-Path $kits "Lib/$version/um/x64"
$ucrtLib = Join-Path $kits "Lib/$version/ucrt/x64"
$source = Join-Path $PSScriptRoot 'main.cpp'
$iidSource = Join-Path $sdkInclude 'public.sdk\source\vst\vstinitiids.cpp'
$output = [System.IO.Path]::GetFullPath($Output)
$object = Join-Path $PSScriptRoot 'm06-vst3-worker.obj'
$iidObject = Join-Path $PSScriptRoot 'vstinitiids.obj'
foreach ($path in @($cl, "$sdkInclude/pluginterfaces/base/ipluginbase.h", $iidSource, "$include/um/Windows.h")) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Required native toolchain path is missing: $path" }
}
& $cl /nologo /EHsc /std:c++20 "/I$vcInclude" "/I$sdkInclude" "/I$include/shared" "/I$include/um" "/I$include/ucrt" /c $source /Fo:$object
if ($LASTEXITCODE -ne 0) { throw "VST3 worker compile failed with exit code $LASTEXITCODE" }
& $cl /nologo /EHsc /std:c++20 "/I$vcInclude" "/I$sdkInclude" "/I$include/shared" "/I$include/um" "/I$include/ucrt" /c $iidSource /Fo:$iidObject
if ($LASTEXITCODE -ne 0) { throw "VST3 SDK IID compile failed with exit code $LASTEXITCODE" }
& $cl /nologo $object $iidObject /Fe:$output /link "/LIBPATH:$vcLib" "/LIBPATH:$umLib" "/LIBPATH:$ucrtLib" ole32.lib bcrypt.lib wer.lib
if ($LASTEXITCODE -ne 0) { throw "VST3 worker build failed with exit code $LASTEXITCODE" }
