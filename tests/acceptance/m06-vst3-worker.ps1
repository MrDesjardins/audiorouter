param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$workerBuild = Join-Path $repositoryRoot 'tools\m06-vst3-worker\build.ps1'
$worker = Join-Path $repositoryRoot 'tools\m06-vst3-worker\m06-vst3-worker.exe'
$workerObject = Join-Path $repositoryRoot 'tools\m06-vst3-worker\m06-vst3-worker.obj'
$fixture = Join-Path $repositoryRoot 'third_party\vst3sdk-build\VST3\Release\again.vst3'
if (-not (Test-Path -LiteralPath $fixture -PathType Container)) {
    throw "AGain fixture is missing; run m06-vst3-sdk.ps1 first: $fixture"
}
if (-not $SkipBuild) {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $workerBuild
    if ($LASTEXITCODE -ne 0) { throw "native VST3 worker build failed with exit code $LASTEXITCODE" }
}
foreach ($path in @($worker, $fixture)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "required native VST3 worker path is missing: $path" }
}
$previousFixture = $env:AUDIOROUTER_VST3_FIXTURE
$previousWorker = $env:AUDIOROUTER_VST3_NATIVE_WORKER
try {
    $env:AUDIOROUTER_VST3_FIXTURE = $fixture
    $env:AUDIOROUTER_VST3_NATIVE_WORKER = $worker
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --ignored --exact verified_native_vst3_worker_processes_an_opt_in_fixture --nocapture
    if ($LASTEXITCODE -ne 0) { throw "native VST3 worker acceptance failed with exit code $LASTEXITCODE" }
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked -- --ignored --exact verified_native_vst3_worker_processes_an_opt_in_multi_bus_fixture --nocapture
    if ($LASTEXITCODE -ne 0) { throw "native VST3 multi-bus worker acceptance failed with exit code $LASTEXITCODE" }
} finally {
    if ($null -eq $previousFixture) { Remove-Item Env:AUDIOROUTER_VST3_FIXTURE -ErrorAction SilentlyContinue } else { $env:AUDIOROUTER_VST3_FIXTURE = $previousFixture }
    if ($null -eq $previousWorker) { Remove-Item Env:AUDIOROUTER_VST3_NATIVE_WORKER -ErrorAction SilentlyContinue } else { $env:AUDIOROUTER_VST3_NATIVE_WORKER = $previousWorker }
    Remove-Item -LiteralPath $worker,$workerObject -Force -ErrorAction SilentlyContinue
}
Write-Output 'M06 native VST3 worker acceptance passed: isolated AGain single-stream and auxiliary-bus processing, finite output, and bounded shutdown.'
Write-Output 'Scope: repository-local native worker and AGain fixture; no plugin registration, audio stream, or machine audio configuration changes.'
