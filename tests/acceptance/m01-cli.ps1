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
    if ($devices.Count -eq 0) { throw "Windows device discovery returned no active endpoints" }
    if ($null -eq ($devices | Where-Object { $_.state -eq "active" -and $_.id })) { throw "Active endpoint metadata missing" }

    $nodes = cargo run --quiet -p audiorouter-cli -- --json nodes types | ConvertFrom-Json
    $physicalInput = $nodes | Where-Object { $_.type -eq "physical-input@1" }
    if ($null -eq $physicalInput -or $physicalInput.availability.status -ne "unavailable") { throw "Physical input availability boundary missing" }

    $suffix = "audiorouter-acceptance-$PID"
    $database = Join-Path ([IO.Path]::GetTempPath()) "$suffix.sqlite"
    $document = Join-Path ([IO.Path]::GetTempPath()) "$suffix.json"
    $bundle = Join-Path ([IO.Path]::GetTempPath()) "$suffix.audiorouter"
    $importedDatabase = Join-Path ([IO.Path]::GetTempPath()) "$suffix-imported.sqlite"
    $staging = Join-Path ([IO.Path]::GetTempPath()) "$suffix-staging"
    try {
        Remove-Item -LiteralPath $database,$document,$bundle,$importedDatabase -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
        Copy-Item -LiteralPath (Join-Path $repoRoot "tests/fixtures/valid-session.json") -Destination $document
        cargo run --quiet -p audiorouter-cli -- import $document --database $database | Out-Null
        cargo run --quiet -p audiorouter-cli -- export-bundle session-fixture --database $database --output $bundle | Out-Null
        New-Item -ItemType Directory -Path $staging | Out-Null
        $imported = cargo run --quiet -p audiorouter-cli -- --json import-bundle $bundle --database $importedDatabase --staging $staging | ConvertFrom-Json
        if ($imported.id -ne "session-fixture") { throw "Bundle round trip returned the wrong session" }
    }
    finally {
        Remove-Item -LiteralPath $database,$document,$bundle,$importedDatabase -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Output "M01 CLI acceptance passed"
}
finally {
    Pop-Location
}
