param(
    [string]$FixtureDirectory = ''
)

$ErrorActionPreference = 'Stop'

if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'The VST2 editor acceptance requires Windows.'
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
try {
    foreach ($fixture in $fixtures) {
        $env:AUDIOROUTER_VST2_FIXTURE = $fixture.FullName
        Write-Output "Running bounded VST2 editor acceptance: $($fixture.Name)"
        foreach ($testName in @(
                'dedicated_vst2_editor_thread_bounds_a_nonreturning_native_editor',
                'supervised_vst2_editor_timeout_kills_the_worker_and_records_failure')) {
            & cargo test -p audiorouter-plugin-host --test worker_process `
                --features test-fixtures --locked -- `
                --ignored --exact $testName --nocapture
            if ($LASTEXITCODE -ne 0) {
                throw "VST2 editor acceptance failed for $($fixture.Name) ($testName) with exit code $LASTEXITCODE"
            }
        }
    }
    Write-Output "M06 VST2 editor containment acceptance passed for $($fixtures.Count) local fixtures."
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:AUDIOROUTER_VST2_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_FIXTURE = $previousFixture
    }
}

Write-Output 'Scope: ignored local VST2 fixtures and disposable worker/editor threads; no plugin registration or audio configuration changes.'
