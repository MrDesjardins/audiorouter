[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$manifestPath = Join-Path $repositoryRoot 'src-tauri/Cargo.toml'
$bundleRoot = Join-Path $repositoryRoot 'src-tauri/target/release/bundle/nsis'
$installerPath = Join-Path $bundleRoot 'AudioRouter_0.1.0_x64-setup.exe'
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash

try {
    if (Test-Path -LiteralPath $bundleRoot) {
        throw "Refusing to reuse an existing disposable NSIS output directory: $bundleRoot"
    }

    Push-Location $repositoryRoot
    try {
        & cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host
        if ($LASTEXITCODE -ne 0) {
            throw "CLI and plugin-worker release build failed with exit code $LASTEXITCODE"
        }
        # tauri.release.conf.json bundles the packed Stream Deck plugin.
        & npm.cmd ci --prefix tools/streamdeck
        if ($LASTEXITCODE -ne 0) { throw "Stream Deck locked dependency installation failed" }
        & npm.cmd run pack --prefix tools/streamdeck
        if ($LASTEXITCODE -ne 0) { throw "Stream Deck plugin packing failed" }
        $tauriCli = Join-Path $repositoryRoot 'ui/node_modules/.bin/tauri.cmd'
        if (-not (Test-Path -LiteralPath $tauriCli -PathType Leaf)) {
            throw "Tauri CLI is missing; install locked UI dependencies with npm ci --prefix ui: $tauriCli"
        }
        & $tauriCli build --no-sign --ci --bundles nsis --config src-tauri/tauri.release.conf.json
        if ($LASTEXITCODE -ne 0) {
            throw "unsigned NSIS bundler failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }

    if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
        throw "unsigned NSIS bundler did not produce the expected x64 installer: $installerPath"
    }
    $installer = Get-Item -LiteralPath $installerPath
    if ($installer.Length -le 0) {
        throw "unsigned NSIS bundler produced an empty installer: $installerPath"
    }
    $afterManifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
    if ($afterManifestHash -ne $manifestHash) {
        throw 'Tauri bundling modified src-tauri/Cargo.toml; keep the manifest reproducible.'
    }

    Write-Output "M08 unsigned NSIS installer smoke passed: $($installer.Length) bytes"
    Write-Output 'Scope: disposable unsigned installer generation only; no installer execution, installation, signing, driver, or audio configuration action.'
}
finally {
    if (Test-Path -LiteralPath $bundleRoot) {
        Remove-Item -LiteralPath $bundleRoot -Recurse -Force
    }
}
