[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Release',
    [ValidateSet('x64', 'ARM64')]
    [string] $Platform = 'x64',
    [string] $Output,
    [switch] $KeepOutput
)

$ErrorActionPreference = 'Stop'
$driverRoot = (Resolve-Path (Join-Path $PSScriptRoot '.')).Path
$solution = Join-Path $driverRoot 'AudioRouterVirtual.sln'

if (-not $Output) {
    $Output = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-virtual-driver-' + [guid]::NewGuid().ToString('N'))
}
$output = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Path $output -Force | Out-Null

function Find-MSBuild {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $install = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and $install) {
            $candidate = Join-Path $install.Trim() 'MSBuild\Current\Bin\MSBuild.exe'
            if (Test-Path -LiteralPath $candidate) { return $candidate }
        }
    }
    $command = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw 'MSBuild.exe was not found. Install the VS C++ workload and WDK build tools.'
}

$msbuild = Find-MSBuild
$wdkTargets = @(Get-ChildItem -Path ${env:ProgramFiles(x86)}, ${env:ProgramFiles} -Filter 'WindowsDriver.Default.props' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1)
if (-not $wdkTargets) {
    throw 'WindowsDriver.Default.props was not found. Install the WDK build tools.'
}

$log = Join-Path $output 'build.log'
$args = @(
    $solution,
    '/m:1',
    '/nr:false',
    "/p:Configuration=$Configuration",
    "/p:Platform=$Platform",
    '/p:SkipPackageVerification=true',
    '/p:ApiValidator_Enable=false',
    '/p:SignMode=Off',
    '/p:TrackFileAccess=false',
    '/v:minimal',
    "/flp:LogFile=$log;Verbosity=normal"
)
Write-Host "Build-only AudioRouter virtual driver qualification"
Write-Host "MSBuild: $msbuild"
Write-Host "WDK target: $($wdkTargets[0].FullName)"
& $msbuild @args
if ($LASTEXITCODE -ne 0) { throw "MSBuild failed with exit code $LASTEXITCODE. See $log" }

$sys = @(Get-ChildItem -LiteralPath $driverRoot -Filter 'AudioRouterVirtual.sys' -Recurse -File | Where-Object { $_.FullName -notlike "$output*" })
$inf = @(Get-ChildItem -LiteralPath $driverRoot -Filter 'AudioRouterVirtual.inf' -Recurse -File | Where-Object { $_.FullName -notlike "$output*" })
if (-not $sys -or -not $inf) {
    throw "Build completed but the expected .sys/.inf package was not produced. See $log"
}
Write-Host "Driver binary: $($sys[0].FullName)"
Write-Host "Driver INF: $($inf[0].FullName)"
Write-Host 'No installation, signing, boot-policy, service, or audio-device action was performed.'
if (-not $KeepOutput) {
    Remove-Item -LiteralPath $output -Recurse -Force
    Write-Host 'Removed disposable build output.'
}
