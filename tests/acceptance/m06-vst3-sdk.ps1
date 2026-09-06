param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$sdkRoot = Join-Path $repositoryRoot 'third_party\vst3sdk'
$cmake = Join-Path $repositoryRoot 'third_party\cmake-4.4.0\bin\cmake.exe'
$buildRoot = Join-Path $repositoryRoot 'third_party\vst3sdk-build'
$bundle = Join-Path $buildRoot 'VST3\Release\mda-vst3.vst3'
$validator = Join-Path $buildRoot 'bin\Release\validator.exe'
$loaderScript = Join-Path $repositoryRoot 'tools\m06-vst3-loader\build.ps1'
$loader = Join-Path $repositoryRoot 'tools\m06-vst3-loader\m06-vst3-loader.exe'
$loaderObject = Join-Path $repositoryRoot 'tools\m06-vst3-loader\m06-vst3-loader.obj'
$expectedRevision = '3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96'

function Require-File([string]$Path, [string]$Description) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Description is missing: $Path"
    }
}

function Invoke-Native([string]$File, [string[]]$Arguments) {
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$File failed with exit code $LASTEXITCODE"
    }
}

Require-File $cmake 'repository-local CMake'
Require-File (Join-Path $sdkRoot 'CMakeLists.txt') 'VST3 SDK checkout'
Require-File (Join-Path $sdkRoot 'pluginterfaces\base\ipluginbase.h') 'VST3 SDK header'

$revision = (& git -C $sdkRoot rev-parse HEAD).Trim()
if ($revision -ne $expectedRevision) {
    throw "VST3 SDK revision mismatch: expected $expectedRevision, found $revision"
}

try {
    if (-not $SkipBuild) {
        Invoke-Native $cmake @('--build', $buildRoot, '--config', 'Release', '--target', 'mda-vst3')
    }
    Require-File $validator 'built VST3 validator'
    Require-File (Join-Path $bundle 'Contents\x86_64-win\mda-vst3.vst3') 'built mda VST3 binary'

    Invoke-Native $validator @($bundle)
    Invoke-Native 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $loaderScript)
    Invoke-Native $loader @($bundle)
    Write-Output 'M06 VST3 SDK acceptance passed: pinned checkout, build, validator, and offline loader.'
} finally {
    foreach ($generated in @($loader, $loaderObject)) {
        if (Test-Path -LiteralPath $generated) {
            Remove-Item -LiteralPath $generated -Force
        }
    }
}

Write-Output 'Scope: repository-local SDK and offline plugin fixture; no system installation or audio configuration changes.'
