param(
    [string]$SdkInclude = '',
    # Which fixture: Sample64Only or TwoClasses (sources sample64only.cpp, twoclasses.cpp).
    [ValidateSet('Sample64Only', 'TwoClasses')][string]$Name = 'Sample64Only',
    # Folder that receives <Name>.vst3\Contents\x86_64-win\<Name>.vst3
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)

# Builds the 64-bit-samples-only VST3 test fixture (see sample64only.cpp).
$ErrorActionPreference = 'Stop'
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) { throw 'No Visual Studio installation with the native C++ workload was found' }
$msvcRoot = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1
$cl = Join-Path $msvcRoot.FullName 'bin\Hostx64\x64\cl.exe'
$vcInclude = Join-Path $msvcRoot.FullName 'include'
$vcLib = Join-Path $msvcRoot.FullName 'lib\x64'
if (-not $SdkInclude) { $SdkInclude = Join-Path $PSScriptRoot '..\..\..\third_party\vst3sdk' }
$sdk = [System.IO.Path]::GetFullPath($SdkInclude)
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$kitRoot = Get-ChildItem -LiteralPath (Join-Path $kits 'Include') -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h') } |
    Sort-Object Name -Descending | Select-Object -First 1
$include = $kitRoot.FullName
$version = $kitRoot.Name
$umLib = Join-Path $kits "Lib/$version/um/x64"
$ucrtLib = Join-Path $kits "Lib/$version/ucrt/x64"
$bundle = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) "$Name.vst3"
$binaryDirectory = Join-Path $bundle 'Contents\x86_64-win'
New-Item -ItemType Directory -Force -Path $binaryDirectory | Out-Null
$output = Join-Path $binaryDirectory "$Name.vst3"
$work = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) 'obj'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$sources = @(
    (Join-Path $PSScriptRoot ($Name.ToLowerInvariant() + '.cpp')),
    (Join-Path $sdk 'pluginterfaces\base\funknown.cpp'),
    (Join-Path $sdk 'public.sdk\source\vst\vstinitiids.cpp')
)
& $cl /nologo /EHsc /std:c++20 /MT /LD "/I$vcInclude" "/I$sdk" "/I$include/shared" "/I$include/um" "/I$include/ucrt" @sources "/Fo:$work\" "/Fe:$output" /link "/LIBPATH:$vcLib" "/LIBPATH:$umLib" "/LIBPATH:$ucrtLib" ole32.lib user32.lib
if ($LASTEXITCODE -ne 0) { throw "fixture build failed with exit code $LASTEXITCODE" }
Write-Output $bundle
