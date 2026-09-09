param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$sdkRoot = Join-Path $repositoryRoot 'third_party\vst3sdk'
$cmake = Join-Path $repositoryRoot 'third_party\cmake-4.4.0\bin\cmake.exe'
$buildRoot = Join-Path $repositoryRoot 'third_party\vst3sdk-build'
$bundle = Join-Path $buildRoot 'VST3\Release\mda-vst3.vst3'
$againBundle = Join-Path $buildRoot 'VST3\Release\again.vst3'
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

function Invoke-NativeCapture([string]$File, [string[]]$Arguments) {
    $output = @(& $File @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "$File failed with exit code $LASTEXITCODE`n$($output -join "`n")"
    }
    return $output
}

function Invoke-ExpectedFailure([string]$File, [string[]]$Arguments, [string]$ExpectedText) {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $File
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    # ArgumentList is unavailable on the Windows PowerShell/.NET runtime used
    # by the acceptance host; these arguments are paths/options without quote
    # characters, so ordinary quoted Windows command-line arguments suffice.
    $startInfo.Arguments = ($Arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $exitCode = $process.ExitCode
    $output = @($stdout, $stderr)
    if ($exitCode -eq 0) {
        throw "$File unexpectedly accepted the unsupported fixture"
    }
    if (($output -join "`n") -notmatch [regex]::Escape($ExpectedText)) {
        throw "$File failed for an unexpected reason; expected '$ExpectedText'`n$($output -join "`n")"
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
        if (-not (Test-Path -LiteralPath $buildRoot -PathType Container)) {
            Invoke-Native $cmake @(
                '-S', $sdkRoot,
                '-B', $buildRoot,
                '-G', 'Visual Studio 18 2026',
                '-A', 'x64',
                '-DSMTG_CREATE_PLUGIN_LINK=0'
            )
        }
        # Forward node-reuse suppression to the Visual Studio generator so
        # disposable acceptance runs do not strand MSBuild workers after the
        # CMake child exits.
        Invoke-Native $cmake @('--build', $buildRoot, '--config', 'Release', '--target', 'mda-vst3', '--parallel', '4', '--', '/nr:false')
        Invoke-Native $cmake @('--build', $buildRoot, '--config', 'Release', '--target', 'again', '--parallel', '4', '--', '/nr:false')
    }
    Require-File $validator 'built VST3 validator'
    Require-File (Join-Path $bundle 'Contents\x86_64-win\mda-vst3.vst3') 'built mda VST3 binary'
    Require-File (Join-Path $againBundle 'Contents\x86_64-win\again.vst3') 'built AGain VST3 binary'

    Invoke-Native $validator @($bundle)
    Invoke-Native $validator @($againBundle)
    Invoke-Native 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $loaderScript)
    Invoke-Native $loader @($againBundle, '--class-index', '0', '--parameter-value', '0.75', '--require-output-change')
    Invoke-ExpectedFailure $loader @($againBundle, '--class-index', '2') 'probe requires one input and output bus'
    Invoke-Native $loader @($againBundle, '--class-index', '2', '--multi-bus')
    $defaultLoaderOutput = Invoke-NativeCapture $loader @($bundle)
    if (-not (($defaultLoaderOutput -join "`n") -match 'parameter_descriptors=\d+')) {
        throw 'offline loader did not report a bounded parameter descriptor catalog'
    }
    $matrixClasses = @(0, 4, 6, 8, 10)
    foreach ($classIndex in $matrixClasses) {
        Invoke-Native $loader @($bundle, '--class-index', "$classIndex")
    }
    Write-Output 'M06 VST3 SDK acceptance passed: pinned checkout, build, validator, offline loader, AGain main and auxiliary-bus classes, explicit single-bus rejection, and five-class mda matrix.'
} finally {
    foreach ($generated in @($loader, $loaderObject)) {
        if (Test-Path -LiteralPath $generated) {
            Remove-Item -LiteralPath $generated -Force
        }
    }
}

Write-Output 'Scope: repository-local SDK and offline plugin fixture; no system installation or audio configuration changes.'
