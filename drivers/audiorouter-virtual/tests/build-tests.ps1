[CmdletBinding()]
param([switch] $SanitizeAddress)
$ErrorActionPreference = 'Stop'
$driverRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
. (Join-Path $driverRoot 'package-tools.ps1')
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$compiler = @(& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe') | Select-Object -First 1
if (-not $compiler) { throw 'MSVC x64 compiler not found.' }
$vcRoot = (Resolve-Path (Join-Path (Split-Path -Parent $compiler) '../../..')).Path
$kitRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
$kit = Get-ChildItem (Join-Path $kitRoot 'Include') -Directory | Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
$workspace = (Resolve-Path (Join-Path $driverRoot '../..')).Path
$suffix = if ($SanitizeAddress) { 'asan' } else { 'release' }
$output = Join-Path $workspace "target\driver-unit-$suffix"
New-Item -ItemType Directory -Path $output -Force | Out-Null
$exe = Join-Path $output 'bridge-tests.exe'
$arguments = @('/nologo', '/std:c++17', '/EHsc', '/O2', '/fp:strict', '/W4', '/WX', "/I$vcRoot\include", "/I$($kit.FullName)\ucrt", "/I$($kit.FullName)\shared", "/I$($kit.FullName)\um", (Join-Path $PSScriptRoot 'bridge_tests.cpp'), "/Fe:$exe", "/Fo:$output\bridge-tests.obj")
if ($SanitizeAddress) { $arguments += @('/fsanitize=address', '/Zi', "/Fd:$output\bridge-tests.pdb") }
$arguments += @('/link', "/LIBPATH:$vcRoot\lib\x64", "/LIBPATH:$kitRoot\Lib\$($kit.Name)\ucrt\x64", "/LIBPATH:$kitRoot\Lib\$($kit.Name)\um\x64")
$null = Invoke-DriverTool $compiler $arguments (Join-Path $output 'compile.log')
if ($SanitizeAddress) {
    # ASan's runtime DLL lives beside cl.exe; change this process's search path
    # only, never the persistent environment or the user's app launch arguments.
    $env:Path = (Split-Path -Parent $compiler) + ';' + $env:Path
}
$result = Invoke-DriverTool $exe @() (Join-Path $output 'tests.log')
Write-Output $result.Output
