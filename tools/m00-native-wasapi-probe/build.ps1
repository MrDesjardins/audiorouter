$ErrorActionPreference = 'Stop'

$cl = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\cl.exe'
$vcInclude = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\include'
$vcLib = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.51.36231\lib\x64'
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$version = '10.0.28000.0'
$include = Join-Path $kits "Include\$version"
$umLib = Join-Path $kits "Lib\$version\um\x64"
$ucrtLib = Join-Path $kits "Lib\$version\ucrt\x64"
$source = Join-Path $PSScriptRoot 'main.cpp'
$output = Join-Path $PSScriptRoot 'main.exe'
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
