param(
    [string]$FixtureDirectory = ''
)

$ErrorActionPreference = 'Stop'

if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'The VST2 worker acceptance requires Windows.'
}

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if ([string]::IsNullOrWhiteSpace($FixtureDirectory)) {
    $FixtureDirectory = Join-Path $repositoryRoot 'third_party\local-test-fixtures\ReaPlugs'
}
$fixtureRoot = (Resolve-Path -LiteralPath $FixtureDirectory).Path
$fixtures = @(Get-ChildItem -LiteralPath $fixtureRoot -Filter '*.dll' -File | Sort-Object Name)
if ($fixtures.Count -eq 0) {
    throw "No VST2 DLL fixtures found in $fixtureRoot"
}

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
$previousSampleRate = $env:AUDIOROUTER_VST2_SAMPLE_RATE
try {
    foreach ($fixture in $fixtures) {
        $env:AUDIOROUTER_VST2_FIXTURE = $fixture.FullName
        foreach ($sampleRate in @(44100, 48000, 96000)) {
            $env:AUDIOROUTER_VST2_SAMPLE_RATE = [string]$sampleRate
            Write-Output "Running VST2 worker acceptance: $($fixture.Name) at ${sampleRate} Hz"
            & cargo test -p audiorouter-plugin-host --test worker_process `
                --features test-fixtures --locked -- `
                --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
            if ($LASTEXITCODE -ne 0) {
                throw "VST2 worker acceptance failed for $($fixture.Name) at ${sampleRate} Hz with exit code $LASTEXITCODE"
            }
        }
    }
    Write-Output "M06 VST2 acceptance passed for $($fixtures.Count) local fixtures at 44.1, 48, and 96 kHz."
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

Write-Output 'Scope: ignored local VST2 fixtures and disposable worker processes; no plugin registration or audio configuration changes.'
