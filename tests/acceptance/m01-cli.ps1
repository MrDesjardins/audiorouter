[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    $schema = cargo run --quiet -p audiorouter-cli -- --json schema | ConvertFrom-Json
    if ($schema.protocolVersion.major -ne 1) { throw "Unexpected protocol major version" }
    if (-not ($schema.methods.name -contains "graph.plan")) { throw "graph.plan missing from discovery" }

    $status = cargo run --quiet -p audiorouter-cli -- --json status | ConvertFrom-Json
    if ($status.audio -ne "unavailable") { throw "M01 status must identify real audio as unavailable" }

    $devices = cargo run --quiet -p audiorouter-cli -- --json devices list | ConvertFrom-Json
    if ($devices.Count -ne 0) { throw "Offline M01 device discovery must not invent devices" }

    $nodes = cargo run --quiet -p audiorouter-cli -- --json nodes types | ConvertFrom-Json
    $physicalInput = $nodes | Where-Object { $_.type -eq "physical-input@1" }
    if ($null -eq $physicalInput -or $physicalInput.availability.status -ne "unavailable") { throw "Physical input availability boundary missing" }

    Write-Output "M01 CLI acceptance passed"
}
finally {
    Pop-Location
}
