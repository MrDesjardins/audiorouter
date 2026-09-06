[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$runbook = Join-Path $PSScriptRoot "../../docs/operations/release-qualification.md"
$text = Get-Content -LiteralPath $runbook -Raw
if ($text -match "verify-artifacts\.ps1 -ReleaseDirectory") {
    throw "release runbook uses the obsolete -ReleaseDirectory parameter"
}
if ($text -notmatch "verify-artifacts\.ps1 -ManifestPath <prepared-directory>\\release-manifest\.json") {
    throw "release runbook does not document the verifier's -ManifestPath invocation"
}
Write-Output "Release qualification command documentation passed"
