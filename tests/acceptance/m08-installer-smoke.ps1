[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$manifestPath = Join-Path $repositoryRoot 'src-tauri/Cargo.toml'
$bundleRoot = Join-Path $repositoryRoot 'src-tauri/target/debug/bundle/nsis'
$installerPath = Join-Path $bundleRoot 'AudioRouter_0.1.0_x64-setup.exe'
$manifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash

try {
    if (Test-Path -LiteralPath $bundleRoot) {
        throw "Refusing to reuse an existing disposable NSIS output directory: $bundleRoot"
    }

    Push-Location $repositoryRoot
    try {
        & npm.cmd exec --yes --package '@tauri-apps/cli@2.11.4' -- tauri build `
            --debug --no-sign --ci --bundles nsis --config src-tauri/tauri.conf.json
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
