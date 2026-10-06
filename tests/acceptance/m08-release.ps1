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
    $expectedVersion = ([string](Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version)
    if ($manifest.version -ne $expectedVersion) {
        throw "release manifest version does not match Tauri package version"
    }
    $expectedUiVersion = [string](Get-Content -LiteralPath (Join-Path $repoRoot 'ui/package.json') -Raw | ConvertFrom-Json).version
    if ($manifest.version -ne $expectedUiVersion) {
        throw "release manifest version does not match UI package version"
    }
    if ($manifest.signed -ne $false -or $manifest.publicationReady -ne $false) {
        throw "unsigned preparation must not claim signed or publication-ready status"
    }
    if (@($manifest.blockers).Count -lt 3) {
        throw "unsigned preparation must retain all release blockers"
    }
    foreach ($required in @(
        "AudioRouter_$($manifest.version)_x64-setup.exe",
        "com.mrdesjardins.audiorouter.streamDeckPlugin",
        "sbom.cargo.json",
        "sbom.npm.json",
        "sbom.npm.package-lock.json",
        "THIRD-PARTY-NOTICES.txt",
        "SHA256SUMS.txt"
    )) {
        if (@($manifest.artifacts | Where-Object { $_.file -eq $required }).Count -ne 1) {
            throw "release manifest must include exactly one $required artifact"
        }
    }
    # The installer is the only download users need; the app, CLI, plugin
    # worker and Stream Deck plugin ship inside it, not as separate assets.
    foreach ($bundled in @("audiorouter-cli.exe", "audiorouter-plugin-worker.exe", "audiorouter-shell.exe", "audiorouter-ui.zip")) {
        if (@($manifest.artifacts | Where-Object { $_.file -eq $bundled }).Count -ne 0) {
            throw "release must not publish $bundled separately from the installer"
        }
    }
    $streamDeckPackage = Join-Path $output "com.mrdesjardins.audiorouter.streamDeckPlugin"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($streamDeckPackage)
    try {
        if ($null -eq $archive.GetEntry("com.mrdesjardins.audiorouter.sdPlugin/manifest.json")) {
            throw "Stream Deck plugin package is missing its manifest"
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
    Write-Output "Scope: unsigned per-user NSIS artifact preparation and verification only; no installer execution, driver, app signing, or audio configuration changes."
}
finally {
    Pop-Location
    if (Test-Path -LiteralPath $output) {
        Remove-Item -LiteralPath $output -Recurse -Force
    }
}
