[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$output = Join-Path ([IO.Path]::GetTempPath()) "audiorouter-m08-release-$PID"
if (Test-Path -LiteralPath $output) {
    throw "Refusing to reuse an existing release output directory: $output"
}

Push-Location $repoRoot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\release\prepare-artifacts.ps1 -OutputDirectory $output
    if ($LASTEXITCODE -ne 0) { throw "release preparation failed with exit code $LASTEXITCODE" }

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\release\verify-artifacts.ps1 -ManifestPath (Join-Path $output "release-manifest.json")
    if ($LASTEXITCODE -ne 0) { throw "release verification failed with exit code $LASTEXITCODE" }

    $manifest = Get-Content -LiteralPath (Join-Path $output "release-manifest.json") -Raw | ConvertFrom-Json
    if ($manifest.signed -ne $false -or $manifest.publicationReady -ne $false) {
        throw "unsigned preparation must not claim signed or publication-ready status"
    }
    if (@($manifest.blockers).Count -lt 3) {
        throw "unsigned preparation must retain all release blockers"
    }

    Write-Output "M08 release preparation acceptance passed"
    Write-Output "Scope: unsigned artifact preparation and verification only; no installer, driver, signing, or audio configuration changes."
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $output) {
        Remove-Item -LiteralPath $output -Recurse -Force
    }
}
