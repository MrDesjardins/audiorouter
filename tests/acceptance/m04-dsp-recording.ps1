[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo format check failed with exit code $LASTEXITCODE" }

    & cargo test --locked -p audiorouter-dsp -p audiorouter-recording
    if ($LASTEXITCODE -ne 0) { throw "DSP/recording tests failed with exit code $LASTEXITCODE" }

    & cargo clippy --locked -p audiorouter-dsp -p audiorouter-recording --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "DSP/recording Clippy failed with exit code $LASTEXITCODE" }

    & git diff --check
    if ($LASTEXITCODE -ne 0) { throw "diff check failed with exit code $LASTEXITCODE" }

    Write-Output "M04 DSP/recording acceptance passed"
    Write-Output "Scope: portable processing and file-boundary validation; no audio device or machine configuration changes."
}
finally {
    Pop-Location
}
