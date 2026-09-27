param(
    [string]$FixtureDirectory = '',
    [string]$LiveDatabase = '',
    [string]$WorkerExecutable = '',
    [switch]$AllowLiveAudio
)

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (-not $FixtureDirectory) {
    $FixtureDirectory = Join-Path $workspace 'third_party/local-test-fixtures/Vst2State'
}
$fixtures = (Resolve-Path -LiteralPath $FixtureDirectory).Path
foreach ($name in @('state', 'legacy-main', 'crash', 'hang', 'nonfinite')) {
    if (-not (Test-Path -LiteralPath (Join-Path $fixtures "audiorouter-vst2-$name-fixture.dll"))) {
        throw 'Build the repository-owned fixtures with m06-vst2-state-fixture.ps1 first.'
    }
}
if ($LiveDatabase -and -not $AllowLiveAudio) { throw 'Live qualification requires explicit -AllowLiveAudio.' }
if ($LiveDatabase) {
    $database = (Resolve-Path -LiteralPath $LiveDatabase).Path
    $targetRoot = (Resolve-Path (Join-Path $workspace 'target')).Path + [IO.Path]::DirectorySeparatorChar
    if (-not $database.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Use a disposable database copy under the workspace target directory.'
    }
    if (Get-Process audiorouter-shell -ErrorAction SilentlyContinue) { throw 'Close AudioRouter before live qualification.' }
}

$previousFixture = $env:AUDIOROUTER_VST2_CHAIN_FIXTURES
$previousDatabase = $env:AUDIOROUTER_LIVE_PATHS_DATABASE
$previousWorker = $env:AUDIOROUTER_PLUGIN_WORKER_PATH
$previousSession = $env:AUDIOROUTER_LIVE_PATHS_SESSION
function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}
try {
    Push-Location $workspace
    $evidenceDirectory = Join-Path $workspace 'target/plugin-chain-qualification'
    New-Item -ItemType Directory -Path $evidenceDirectory -Force | Out-Null
    Start-Transcript -Path (Join-Path $evidenceDirectory 'qualification.log') -Force | Out-Null
    $env:AUDIOROUTER_VST2_CHAIN_FIXTURES = $fixtures
    & cargo test -p audiorouter-plugin-host --test worker_process --features test-fixtures --locked shared_vst2_chain -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Shared VST2 state/parameter/fault qualification failed.' }
    if ($LiveDatabase) {
        if ($WorkerExecutable) {
            $workerPath = (Resolve-Path -LiteralPath $WorkerExecutable).Path
        } else {
            & cargo build -p audiorouter-plugin-host --bin audiorouter-plugin-worker --locked
            if ($LASTEXITCODE -ne 0) { throw 'Production plugin worker build failed.' }
            $workerPath = Join-Path $workspace 'target/debug/audiorouter-plugin-worker.exe'
        }
        $env:AUDIOROUTER_LIVE_PATHS_DATABASE = $database
        $env:AUDIOROUTER_LIVE_PATHS_SESSION = 'patrick-main-session'
        $env:AUDIOROUTER_PLUGIN_WORKER_PATH = $workerPath
        $before = Get-MediaSnapshot
        try {
            & cargo test -p audiorouter-control --locked live_native_paths_start_pump_and_report_signal_timing -- --ignored --nocapture
            if ($LASTEXITCODE -ne 0) { throw 'Privacy-muted Patrick Main Session qualification failed.' }
        } finally {
            $after = Get-MediaSnapshot
            if (Compare-Object $before $after) { throw 'Media-device identity/state changed during live qualification.' }
        }
        Write-Output 'Privacy-muted shared-chain live check passed; media-device identity/state unchanged.'
    }
    Write-Output 'Shared VST2 state, parameter routing, crash, hang and nonfinite containment passed.'
} finally {
    Stop-Transcript -ErrorAction SilentlyContinue | Out-Null
    Pop-Location
    $env:AUDIOROUTER_VST2_CHAIN_FIXTURES = $previousFixture
    $env:AUDIOROUTER_LIVE_PATHS_DATABASE = $previousDatabase
    $env:AUDIOROUTER_PLUGIN_WORKER_PATH = $previousWorker
    $env:AUDIOROUTER_LIVE_PATHS_SESSION = $previousSession
}
