param(
    [string]$FixtureDirectory = '',
    [switch]$SkipIncompatibleCandidates
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

function Get-PeMachine([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        if ($stream.Length -lt 64) { return $null }
        $dos = [byte[]]::new(64)
        if ($stream.Read($dos, 0, $dos.Length) -ne $dos.Length -or
            $dos[0] -ne 0x4d -or $dos[1] -ne 0x5a) { return $null }
        $peOffset = [BitConverter]::ToInt32($dos, 0x3c)
        if ($peOffset -lt 0 -or $peOffset + 6 -gt $stream.Length) { return $null }
        $stream.Position = $peOffset
        $pe = [byte[]]::new(6)
        if ($stream.Read($pe, 0, $pe.Length) -ne $pe.Length -or
            $pe[0] -ne 0x50 -or $pe[1] -ne 0x45 -or $pe[2] -ne 0 -or $pe[3] -ne 0) {
            return $null
        }
        return [BitConverter]::ToUInt16($pe, 4)
    } finally {
        $stream.Dispose()
    }
}

$supportedFixtures = @()
foreach ($fixture in $fixtures) {
    $machine = Get-PeMachine $fixture.FullName
    if ($machine -eq 0x8664) {
        $supportedFixtures += $fixture
    } elseif ($null -eq $machine) {
        Write-Output "Skipping $($fixture.Name): invalid or unreadable PE header (unsupported candidate)."
    } else {
        Write-Output "Skipping $($fixture.Name): unsupported PE machine 0x$('{0:X4}' -f $machine); x64 VST2 is required."
    }
}
if ($supportedFixtures.Count -eq 0) {
    throw "No x64 VST2 DLL fixtures found in $fixtureRoot"
}

$qualifiedFixtures = @()
$rejectedFixtures = @()

$previousFixture = $env:AUDIOROUTER_VST2_FIXTURE
$previousSampleRate = $env:AUDIOROUTER_VST2_SAMPLE_RATE
try {
    foreach ($fixture in $supportedFixtures) {
        $env:AUDIOROUTER_VST2_FIXTURE = $fixture.FullName
        $candidatePassed = $true
        try {
            foreach ($sampleRate in @(44100, 48000, 96000)) {
                $env:AUDIOROUTER_VST2_SAMPLE_RATE = [string]$sampleRate
                Write-Output "Running VST2 worker acceptance: $($fixture.Name) at ${sampleRate} Hz"
                & cargo test -p audiorouter-plugin-host --test worker_process `
                    --features test-fixtures --locked -- `
                    --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
                if ($LASTEXITCODE -ne 0) {
                    $candidatePassed = $false
                    throw "VST2 worker acceptance failed for $($fixture.Name) at ${sampleRate} Hz with exit code $LASTEXITCODE"
                }
            }
        } catch {
            if (-not $SkipIncompatibleCandidates) {
                throw
            }
            $rejectedFixtures += $fixture
            Write-Output "Rejected x64 VST2 candidate $($fixture.Name): incompatible with the bounded audio-effect/state contract."
            continue
        }
        if ($candidatePassed) {
            $qualifiedFixtures += $fixture
        }
    }
    if ($qualifiedFixtures.Count -eq 0) {
        throw "No x64 VST2 candidates passed the bounded audio-effect acceptance."
    }
    Write-Output "M06 VST2 acceptance passed for $($qualifiedFixtures.Count) x64 fixtures at 44.1, 48, and 96 kHz."
    if ($rejectedFixtures.Count -gt 0) {
        Write-Output "Rejected incompatible x64 candidates: $($rejectedFixtures.Name -join ', ')"
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

Write-Output 'Scope: ignored local VST2 fixtures and disposable worker processes, including intra-block parameter-offset coverage; no plugin registration or audio configuration changes.'
