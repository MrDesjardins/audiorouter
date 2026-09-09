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
try {
    $env:AUDIOROUTER_VST2_FIXTURE = $PluginPath
    & cargo test -p audiorouter-plugin-host --test worker_process `
        --features test-fixtures --locked -- `
        --ignored --exact verified_worker_loads_and_processes_an_opt_in_vst2_fixture --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw "Installed VST2 worker acceptance failed with exit code $LASTEXITCODE"
    }
    Write-Output 'Installed VST2 worker acceptance passed.'
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:AUDIOROUTER_VST2_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:AUDIOROUTER_VST2_FIXTURE = $previousFixture
    }
}

Write-Output 'Scope: one explicitly selected user-installed VST2 DLL; no copy, registration, or audio configuration changes.'
