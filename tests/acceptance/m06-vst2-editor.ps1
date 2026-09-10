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

function Get-PeMachine([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        if ($stream.Length -lt 64) { return $null }
        $dos = [byte[]]::new(64)
        if ($stream.Read($dos, 0, $dos.Length) -ne $dos.Length -or $dos[0] -ne 0x4d -or $dos[1] -ne 0x5a) { return $null }
        $peOffset = [BitConverter]::ToInt32($dos, 0x3c)
        if ($peOffset -lt 0 -or $peOffset + 6 -gt $stream.Length) { return $null }
        $stream.Position = $peOffset
        $pe = [byte[]]::new(6)
        if ($stream.Read($pe, 0, $pe.Length) -ne $pe.Length -or $pe[0] -ne 0x50 -or $pe[1] -ne 0x45 -or $pe[2] -ne 0 -or $pe[3] -ne 0) { return $null }
        return [BitConverter]::ToUInt16($pe, 4)
    } finally {
        $stream.Dispose()
    }
}

$x64Fixtures = @()
foreach ($fixture in $fixtures) {
    $machine = Get-PeMachine $fixture.FullName
    if ($machine -eq 0x8664) {
        $x64Fixtures += $fixture
    } elseif ($null -eq $machine) {
        Write-Output "Skipping $($fixture.Name): invalid or unreadable PE header."
    } else {
        Write-Output "Skipping $($fixture.Name): unsupported PE machine 0x$('{0:X4}' -f $machine); x64 VST2 is required."
    }
}
if ($x64Fixtures.Count -eq 0) {
    throw "No x64 VST2 DLL fixtures found in $fixtureRoot"
}

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
try {
    foreach ($fixture in $x64Fixtures) {
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
    Write-Output "M06 VST2 editor containment acceptance passed for $($x64Fixtures.Count) x64 local fixtures."
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:AUDIOROUTER_VST2_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_FIXTURE = $previousFixture
    }
}

Write-Output 'Scope: ignored local VST2 fixtures and disposable worker/editor threads; no plugin registration or audio configuration changes.'
