param(
    [switch]$SkipBuild,
    [switch]$AllowStateUnsupported,
    [switch]$SingleStreamOnly,
    [string]$FixturePath = ''
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$workerBuild = Join-Path $repositoryRoot 'tools\m06-vst3-worker\build.ps1'
$worker = Join-Path $repositoryRoot 'tools\m06-vst3-worker\m06-vst3-worker.exe'
$workerObject = Join-Path $repositoryRoot 'tools\m06-vst3-worker\m06-vst3-worker.obj'
$iidObject = Join-Path $repositoryRoot 'tools\m06-vst3-worker\vstinitiids.obj'
$workerSource = Get-Content -LiteralPath (Join-Path $repositoryRoot 'tools\m06-vst3-worker\main.cpp') -Raw
foreach ($required in @(
        'case ''\b'':',
        'case ''\n'':',
        'static_cast<unsigned char>(character) < 0x20',
        'SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX)',
        'WerSetFlags(WER_FAULT_REPORTING_NO_UI)')) {
    if (-not $workerSource.Contains($required)) {
        throw "native VST3 worker safety invariant is missing: $required"
    }
}
$defaultFixture = Join-Path $repositoryRoot 'third_party\vst3sdk-build\VST3\Release\again.vst3'
if ([string]::IsNullOrWhiteSpace($FixturePath)) {
    $fixture = $defaultFixture
} else {
    if (-not [System.IO.Path]::IsPathRooted($FixturePath)) {
        throw "FixturePath must be an absolute VST3 bundle path: $FixturePath"
    }
    $fixture = $FixturePath
}
if (-not (Test-Path -LiteralPath $fixture -PathType Container) -and
    -not (Test-Path -LiteralPath $fixture -PathType Leaf)) {
    throw "VST3 fixture bundle or module is missing: $fixture"
}
$fixtureItem = Get-Item -LiteralPath $fixture
if (-not $fixtureItem.PSIsContainer -and $fixtureItem.Extension -ne '.vst3') {
    throw "FixturePath must name a .vst3 bundle or module: $fixture"
}
$fixture = $fixtureItem.FullName
if (-not $SkipBuild) {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $workerBuild
    if ($LASTEXITCODE -ne 0) { throw "native VST3 worker build failed with exit code $LASTEXITCODE" }
}
foreach ($path in @($worker, $fixture)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "required native VST3 worker path is missing: $path" }
}
$previousFixture = $env:AUDIOROUTER_VST3_FIXTURE
$previousWorker = $env:AUDIOROUTER_VST3_NATIVE_WORKER
$previousStateUnsupported = $env:AUDIOROUTER_VST3_ALLOW_STATE_UNSUPPORTED
try {
    $env:AUDIOROUTER_VST3_FIXTURE = $fixture
    $env:AUDIOROUTER_VST3_NATIVE_WORKER = $worker
    if ($AllowStateUnsupported) { $env:AUDIOROUTER_VST3_ALLOW_STATE_UNSUPPORTED = '1' }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --ignored --exact verified_native_vst3_worker_processes_an_opt_in_fixture --nocapture
    if ($LASTEXITCODE -ne 0) { throw "native VST3 worker acceptance failed with exit code $LASTEXITCODE" }
    if (-not $SingleStreamOnly) {
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --ignored --exact verified_native_vst3_worker_processes_an_opt_in_multi_bus_fixture --nocapture
        if ($LASTEXITCODE -ne 0) { throw "native VST3 multi-bus worker acceptance failed with exit code $LASTEXITCODE" }
        & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --ignored --exact verified_native_vst3_async_bus_worker_bridges_the_graph_scheduler --nocapture
        if ($LASTEXITCODE -ne 0) { throw "native VST3 asynchronous worker acceptance failed with exit code $LASTEXITCODE" }
    }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --exact supervised_bus_worker_loop_silences_after_a_bounded_worker_failure --nocapture
    if ($LASTEXITCODE -ne 0) { throw "supervised multi-bus failure recovery acceptance failed with exit code $LASTEXITCODE" }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --exact supervised_bus_worker_loop_keeps_repeated_quanta_bounded --nocapture
    if ($LASTEXITCODE -ne 0) { throw "supervised multi-bus repeated-quantum acceptance failed with exit code $LASTEXITCODE" }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --exact supervised_worker_restart_restores_validated_state --nocapture
    if ($LASTEXITCODE -ne 0) { throw "supervised worker state-restoration acceptance failed with exit code $LASTEXITCODE" }
} finally {
    if ($null -eq $previousFixture) { Remove-Item Env:AUDIOROUTER_VST3_FIXTURE -ErrorAction SilentlyContinue } else { $env:AUDIOROUTER_VST3_FIXTURE = $previousFixture }
    if ($null -eq $previousWorker) { Remove-Item Env:AUDIOROUTER_VST3_NATIVE_WORKER -ErrorAction SilentlyContinue } else { $env:AUDIOROUTER_VST3_NATIVE_WORKER = $previousWorker }
    if ($null -eq $previousStateUnsupported) { Remove-Item Env:AUDIOROUTER_VST3_ALLOW_STATE_UNSUPPORTED -ErrorAction SilentlyContinue } else { $env:AUDIOROUTER_VST3_ALLOW_STATE_UNSUPPORTED = $previousStateUnsupported }
    Remove-Item -LiteralPath $worker,$workerObject,$iidObject -Force -ErrorAction SilentlyContinue
}
$busScope = if ($SingleStreamOnly) { 'isolated single-stream processing' } else { 'isolated single-stream and auxiliary-bus processing, asynchronous graph staging' }
Write-Output "M06 native VST3 worker acceptance passed for fixture ${fixture}: $busScope, bounded restart/quarantine recovery, validated state restoration where supported, repeated-quantum timing, finite transformed output, and bounded shutdown."
Write-Output 'Scope: one explicitly selected local VST3 fixture and the repository native worker; no plugin registration, audio stream, or machine audio configuration changes.'
