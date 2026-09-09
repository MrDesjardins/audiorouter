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

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
try {
    $env:AUDIOROUTER_VST2_FIXTURE = $output
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
        --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VST2 state fixture acceptance failed with exit code $LASTEXITCODE" }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
        --ignored --exact verified_worker_applies_restored_vst2_chunk_state --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VST2 state round-trip acceptance failed with exit code $LASTEXITCODE" }
    $env:AUDIOROUTER_VST2_FIXTURE = $legacyOutput
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
        --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VST2 legacy-main acceptance failed with exit code $LASTEXITCODE" }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- `
        --ignored --exact verified_worker_applies_restored_vst2_chunk_state --nocapture
    if ($LASTEXITCODE -ne 0) { throw "VST2 legacy-main state acceptance failed with exit code $LASTEXITCODE" }
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:AUDIOROUTER_VST2_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_FIXTURE = $previousFixture
    }
}
Write-Output 'M06 VST2 chunk-state and legacy-main fixture acceptance passed.'
Write-Output 'Scope: repository-owned ignored DLLs and disposable workers; no plugin registration or audio configuration changes.'
