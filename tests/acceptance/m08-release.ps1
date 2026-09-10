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
    foreach ($required in @(
        "audiorouter-cli.exe",
        "audiorouter-plugin-worker.exe",
        "audiorouter-shell.exe",
        "audiorouter-ui.zip",
        "sbom.cargo.json",
        "sbom.npm.json",
        "sbom.npm.package-lock.json",
        "THIRD-PARTY-NOTICES.txt"
    )) {
        if (@($manifest.artifacts | Where-Object { $_.file -eq $required }).Count -ne 1) {
            throw "release manifest must include exactly one $required artifact"
        }
    }
    $uiArtifact = @($manifest.artifacts | Where-Object { $_.file -eq "audiorouter-ui.zip" })
    if ($uiArtifact.Count -ne 1) {
        throw "release manifest must include exactly one audiorouter-ui.zip artifact"
    }
    $uiZip = Join-Path $output "audiorouter-ui.zip"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($uiZip)
    try {
        if ($null -eq $archive.GetEntry("index.html")) {
            throw "UI release archive is missing index.html"
        }
    }
    finally {
        $archive.Dispose()
    }
    foreach ($provenance in @("sbom.npm.json", "sbom.npm.package-lock.json")) {
        if (@($manifest.artifacts | Where-Object { $_.file -eq $provenance }).Count -ne 1) {
            throw "release manifest must include exactly one $provenance artifact"
        }
    }
    $npmSbom = Join-Path $output "sbom.npm.json"
    & node.exe -e "const b=JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8')); if (b.bomFormat !== 'CycloneDX' || b.specVersion !== '1.5' || !Array.isArray(b.components) || b.components.length === 0) process.exit(1)" -- $npmSbom
    if ($LASTEXITCODE -ne 0) {
        throw "generated npm SBOM failed structural validation"
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
