param(
    [string]$OutputDirectory = ''
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot 'third_party\local-test-fixtures\Vst2State'
}
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$source = Join-Path $repositoryRoot 'tests\fixtures\vst2-state-fixture.c'
$output = Join-Path $outputRoot 'audiorouter-vst2-state-fixture.dll'
$object = Join-Path $outputRoot 'audiorouter-vst2-state-fixture.obj'
$legacyOutput = Join-Path $outputRoot 'audiorouter-vst2-legacy-main-fixture.dll'
$legacyObject = Join-Path $outputRoot 'audiorouter-vst2-legacy-main-fixture.obj'
$invalidOutput = Join-Path $outputRoot 'audiorouter-vst2-nonfinite-fixture.dll'
$invalidObject = Join-Path $outputRoot 'audiorouter-vst2-nonfinite-fixture.obj'
$crashOutput = Join-Path $outputRoot 'audiorouter-vst2-crash-fixture.dll'
$crashObject = Join-Path $outputRoot 'audiorouter-vst2-crash-fixture.obj'
$hangOutput = Join-Path $outputRoot 'audiorouter-vst2-hang-fixture.dll'
$hangObject = Join-Path $outputRoot 'audiorouter-vst2-hang-fixture.obj'
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) { throw "vswhere is missing: $vswhere" }
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) { throw 'No Visual Studio installation with native C++ tools was found' }
$msvcRoot = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $msvcRoot) { throw "MSVC tools are missing from $installation" }
$cl = Join-Path $msvcRoot.FullName 'bin\Hostx64\x64\cl.exe'
if (-not (Test-Path -LiteralPath $cl -PathType Leaf)) { throw "MSVC compiler is missing: $cl" }
$vcInclude = Join-Path $msvcRoot.FullName 'include'
$vcLib = Join-Path $msvcRoot.FullName 'lib\x64'
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$kitRoot = Get-ChildItem -LiteralPath (Join-Path $kits 'Include') -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h') } |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $kitRoot) { throw "Windows SDK headers are missing from $kits" }
$kitVersion = $kitRoot.Name
$kitLib = Join-Path $kits "Lib\$kitVersion\um\x64"
$ucrtLib = Join-Path $kits "Lib\$kitVersion\ucrt\x64"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Fixture source is missing: $source" }
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
& $cl /nologo /LD /O2 /W4 /TC "/I$vcInclude" "/I$kitRoot\ucrt" "/I$kitRoot\shared" "/I$kitRoot\um" $source "/Fo:$object" "/Fe:$output" /link "/LIBPATH:$vcLib" "/LIBPATH:$kitLib" "/LIBPATH:$ucrtLib"
if ($LASTEXITCODE -ne 0) { throw "VST2 state fixture build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $output -PathType Leaf)) { throw "Fixture build produced no DLL: $output" }
& $cl /nologo /LD /O2 /W4 /TC /DLEGACY_VST2_MAIN "/I$vcInclude" "/I$kitRoot\ucrt" "/I$kitRoot\shared" "/I$kitRoot\um" $source "/Fo:$legacyObject" "/Fe:$legacyOutput" /link "/LIBPATH:$vcLib" "/LIBPATH:$kitLib" "/LIBPATH:$ucrtLib"
if ($LASTEXITCODE -ne 0) { throw "VST2 legacy-main fixture build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $legacyOutput -PathType Leaf)) { throw "Legacy-main fixture build produced no DLL: $legacyOutput" }
& $cl /nologo /LD /O2 /W4 /TC /DVST2_NONFINITE_OUTPUT "/I$vcInclude" "/I$kitRoot\ucrt" "/I$kitRoot\shared" "/I$kitRoot\um" $source "/Fo:$invalidObject" "/Fe:$invalidOutput" /link "/LIBPATH:$vcLib" "/LIBPATH:$kitLib" "/LIBPATH:$ucrtLib"
if ($LASTEXITCODE -ne 0) { throw "VST2 non-finite fixture build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $invalidOutput -PathType Leaf)) { throw "Non-finite fixture build produced no DLL: $invalidOutput" }
& $cl /nologo /LD /O2 /W4 /TC /DVST2_CRASH_OUTPUT "/I$vcInclude" "/I$kitRoot\ucrt" "/I$kitRoot\shared" "/I$kitRoot\um" $source "/Fo:$crashObject" "/Fe:$crashOutput" /link "/LIBPATH:$vcLib" "/LIBPATH:$kitLib" "/LIBPATH:$ucrtLib"
if ($LASTEXITCODE -ne 0) { throw "VST2 crash fixture build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $crashOutput -PathType Leaf)) { throw "Crash fixture build produced no DLL: $crashOutput" }
& $cl /nologo /LD /O2 /W4 /TC /DVST2_HANG_OUTPUT "/I$vcInclude" "/I$kitRoot\ucrt" "/I$kitRoot\shared" "/I$kitRoot\um" $source "/Fo:$hangObject" "/Fe:$hangOutput" /link "/LIBPATH:$vcLib" "/LIBPATH:$kitLib" "/LIBPATH:$ucrtLib"
if ($LASTEXITCODE -ne 0) { throw "VST2 hang fixture build failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $hangOutput -PathType Leaf)) { throw "Hang fixture build produced no DLL: $hangOutput" }

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
$previousSampleRate = $env:AUDIOROUTER_VST2_SAMPLE_RATE
try {
    $env:AUDIOROUTER_VST2_FIXTURE = $output
    foreach ($sampleRate in @(44100, 48000, 96000)) {
        $env:AUDIOROUTER_VST2_SAMPLE_RATE = [string]$sampleRate
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
            --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
        if ($LASTEXITCODE -ne 0) { throw "VST2 state fixture acceptance failed at ${sampleRate} Hz with exit code $LASTEXITCODE" }
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
            --ignored --exact verified_worker_applies_restored_vst2_chunk_state --nocapture
        if ($LASTEXITCODE -ne 0) { throw "VST2 state round-trip acceptance failed at ${sampleRate} Hz with exit code $LASTEXITCODE" }
    }
    $env:AUDIOROUTER_VST2_FIXTURE = $legacyOutput
    foreach ($sampleRate in @(44100, 48000, 96000)) {
        $env:AUDIOROUTER_VST2_SAMPLE_RATE = [string]$sampleRate
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
            --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
        if ($LASTEXITCODE -ne 0) { throw "VST2 legacy-main acceptance failed at ${sampleRate} Hz with exit code $LASTEXITCODE" }
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
            --ignored --exact verified_worker_applies_restored_vst2_chunk_state --nocapture
        if ($LASTEXITCODE -ne 0) { throw "VST2 legacy-main state acceptance failed at ${sampleRate} Hz with exit code $LASTEXITCODE" }
    }
    $env:AUDIOROUTER_VST2_FIXTURE = $invalidOutput
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
        --ignored --exact verified_worker_rejects_nonfinite_vst2_output --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VST2 non-finite acceptance failed with exit code $LASTEXITCODE" }
    foreach ($faultOutput in @($crashOutput, $hangOutput)) {
        $env:AUDIOROUTER_VST2_FIXTURE = $faultOutput
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
            --ignored --exact verified_worker_contains_native_vst2_fault --nocapture
        if ($LASTEXITCODE -ne 0) { throw "VST2 fault acceptance failed for $faultOutput with exit code $LASTEXITCODE" }
    }
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:AUDIOROUTER_VST2_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_FIXTURE = $previousFixture
    }
    if ($null -eq $previousSampleRate) {
        Remove-Item Env:AUDIOROUTER_VST2_SAMPLE_RATE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_SAMPLE_RATE = $previousSampleRate
    }
}
Write-Output 'M06 VST2 chunk-state and legacy-main fixture acceptance passed at 44.1, 48, and 96 kHz.'
Write-Output 'Scope: repository-owned ignored DLLs and disposable workers; no plugin registration or audio configuration changes.'
