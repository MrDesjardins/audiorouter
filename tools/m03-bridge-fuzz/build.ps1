[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$compiler = @(& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe') | Select-Object -First 1
if (-not $compiler) { throw 'MSVC x64 compiler not found.' }
$vcRoot = (Resolve-Path (Join-Path (Split-Path -Parent $compiler) '../../..')).Path
$kitRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
$kit = Get-ChildItem (Join-Path $kitRoot 'Include') -Directory |
    Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
if (-not $kit) { throw 'Windows SDK headers not found.' }
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$driverRoot = Join-Path $workspace 'drivers/audiorouter-virtual'
. (Join-Path $driverRoot 'package-tools.ps1')
$output = Join-Path $workspace 'target/m03-bridge-fuzz'
New-Item -ItemType Directory -Path $output -Force | Out-Null
$exe = Join-Path $output 'm03-bridge-fuzz.exe'
$arguments = @('/nologo', '/std:c++17', '/EHsc', '/O2', '/W4', '/WX', "/I$vcRoot\include",
    "/I$($kit.FullName)\ucrt", "/I$($kit.FullName)\shared", "/I$($kit.FullName)\um",
    (Join-Path $PSScriptRoot 'bridge_fuzz.cpp'), "/Fe:$exe", "/Fo:$output\bridge_fuzz.obj",
    '/link', "/LIBPATH:$vcRoot\lib\x64", "/LIBPATH:$kitRoot\Lib\$($kit.Name)\ucrt\x64",
    "/LIBPATH:$kitRoot\Lib\$($kit.Name)\um\x64")
$null = Invoke-DriverTool $compiler $arguments (Join-Path $output 'compile.log')
Write-Output "Built VM-only fuzzer: $exe (not run; it opens the loaded driver device)."
