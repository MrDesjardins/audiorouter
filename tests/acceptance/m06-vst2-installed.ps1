param(
    [string]$PluginPath = ''
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($PluginPath)) {
    throw 'PluginPath is required; pass one explicitly selected absolute x64 VST2 DLL path.'
}
if (-not [System.IO.Path]::IsPathRooted($PluginPath)) {
    throw "PluginPath must be an absolute path: $PluginPath"
}
if (-not (Test-Path -LiteralPath $PluginPath -PathType Leaf)) {
    throw "Installed VST2 fixture was not found: $PluginPath"
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

$machine = Get-PeMachine $PluginPath
if ($machine -ne 0x8664) {
    if ($null -eq $machine) { throw 'Selected VST2 binary has an invalid or unreadable PE header; x64 is required.' }
    throw "Selected VST2 binary has unsupported PE machine 0x$('{0:X4}' -f $machine); x64 is required."
}

$initialFile = Get-Item -LiteralPath $PluginPath
$initialSize = $initialFile.Length
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
    $finalFile = Get-Item -LiteralPath $PluginPath
    $finalHash = (Get-FileHash -LiteralPath $PluginPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($finalFile.Length -ne $initialSize -or $finalHash -ne $hash) {
        throw "Selected VST2 binary changed during acceptance: $PluginPath"
    }
    Write-Output 'Installed VST2 worker acceptance passed; binary fingerprint unchanged.'
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

Write-Output 'Scope: one explicitly selected user-installed VST2 DLL; binary fingerprint is checked before/after, with no copy, registration, or audio configuration changes.'
