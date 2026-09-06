[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    & cargo test --locked -p audiorouter-control -p audiorouter-cli -p audiorouter-plugin-host
    if ($LASTEXITCODE -ne 0) { throw "headless/control tests failed with exit code $LASTEXITCODE" }

    & cargo clippy --locked -p audiorouter-control -p audiorouter-cli -p audiorouter-plugin-host --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "headless/control Clippy failed with exit code $LASTEXITCODE" }

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m01-cli.ps1
    if ($LASTEXITCODE -ne 0) { throw "M01 CLI parity acceptance failed with exit code $LASTEXITCODE" }

    & git diff --check
    if ($LASTEXITCODE -ne 0) { throw "diff check failed with exit code $LASTEXITCODE" }

    Write-Output "M07 headless acceptance passed"
    Write-Output "Scope: control/CLI/MCP/plugin-host validation; no audio device, driver, or machine configuration changes."
}
finally {
    Pop-Location
}
