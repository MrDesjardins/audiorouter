param(
    [string]$PluginPath = 'C:\Program Files\Common Files\VST3\Pitchproof\pitchproof-x64.dll'
)

$ErrorActionPreference = 'Stop'

if (-not [System.IO.Path]::IsPathRooted($PluginPath)) {
    throw "PluginPath must be an absolute path: $PluginPath"
}
if (-not (Test-Path -LiteralPath $PluginPath -PathType Leaf)) {
    throw "Installed VST2 fixture was not found: $PluginPath"
}

$hash = (Get-FileHash -LiteralPath $PluginPath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Output "Running installed VST2 worker acceptance: $PluginPath"
Write-Output "SHA-256: $hash"

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
$previousSampleRate = $env:AUDIOROUTER_VST2_SAMPLE_RATE
try {
    $env:AUDIOROUTER_VST2_FIXTURE = $PluginPath
    foreach ($sampleRate in @(44100, 48000, 96000)) {
        $env:AUDIOROUTER_VST2_SAMPLE_RATE = [string]$sampleRate
        Write-Output "Running installed VST2 processing acceptance at ${sampleRate} Hz"
        & cargo test -p audiorouter-plugin-host --test worker_process `
            --features test-fixtures --locked -- `
            --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Installed VST2 worker acceptance failed at ${sampleRate} Hz with exit code $LASTEXITCODE"
        }
    }
    & cargo test -p audiorouter-plugin-host --test worker_process `
        --features test-fixtures --locked -- `
        --ignored --exact dedicated_vst2_editor_thread_bounds_a_nonreturning_native_editor --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Installed VST2 editor-thread containment failed with exit code $LASTEXITCODE"
    }
    & cargo test -p audiorouter-plugin-host --test worker_process `
        --features test-fixtures --locked -- `
        --ignored --exact supervised_vst2_editor_timeout_kills_the_worker_and_records_failure --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Installed VST2 supervised editor containment failed with exit code $LASTEXITCODE"
    }
    Write-Output 'Installed VST2 worker acceptance passed.'
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

Write-Output 'Scope: one explicitly selected user-installed VST2 DLL; no copy, registration, or audio configuration changes.'
