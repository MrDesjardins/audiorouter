[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)

if (Test-Path -LiteralPath $output) {
    throw "Output directory already exists; refusing to overwrite release artifacts: $output"
}
$outputParent = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $outputParent -PathType Container)) {
    throw "Output directory parent must already exist: $outputParent"
}
$parentPath = $outputParent
while (-not [string]::IsNullOrWhiteSpace($parentPath)) {
    $parentItem = Get-Item -LiteralPath $parentPath -Force
    if (($parentItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Output directory parent must not be a reparse point: $parentPath"
    }
    $nextParent = Split-Path -Parent $parentPath
    if ($nextParent -eq $parentPath) { break }
    $parentPath = $nextParent
}

Push-Location $workspace
$uiBuild = Join-Path ([IO.Path]::GetTempPath()) "audiorouter-ui-release-$PID"
try {
    $dirty = & git status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0) {
        throw "could not inspect Git working-tree state"
    }
    if ($dirty) {
        throw "release inputs must come from a clean Git working tree"
    }
    & cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host
    if ($LASTEXITCODE -ne 0) {
        throw "cargo release build failed with exit code $LASTEXITCODE"
    }
    & cargo build --release --locked --manifest-path (Join-Path $workspace "src-tauri/Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "native shell release build failed with exit code $LASTEXITCODE"
    }

    if (Test-Path -LiteralPath $uiBuild) {
        throw "temporary UI build directory already exists; refusing to reuse it: $uiBuild"
    }
    Push-Location (Join-Path $workspace "ui")
    try {
        & npm.cmd run build -- --outDir $uiBuild
        if ($LASTEXITCODE -ne 0) {
            throw "UI release build failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
    if (-not (Test-Path -LiteralPath (Join-Path $uiBuild "index.html") -PathType Leaf)) {
        throw "UI release build did not produce index.html: $uiBuild"
    }
    $uiLock = Join-Path $workspace "ui/package-lock.json"
    if (-not (Test-Path -LiteralPath $uiLock -PathType Leaf)) {
        throw "UI lockfile is missing: $uiLock"
    }
    & node.exe -e "JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8'))" -- $uiLock
    if ($LASTEXITCODE -ne 0) {
        throw "UI lockfile is not valid JSON: $uiLock"
    }

    New-Item -ItemType Directory -Path $output | Out-Null

    $binaries = @(
        @{ Name = "audiorouter-cli.exe"; Source = (Join-Path $workspace "target/release/audiorouter-cli.exe") }
        @{ Name = "audiorouter-plugin-worker.exe"; Source = (Join-Path $workspace "target/release/audiorouter-plugin-worker.exe") }
        @{ Name = "audiorouter-shell.exe"; Source = (Join-Path $workspace "src-tauri/target/release/audiorouter-shell.exe") }
    )
    foreach ($binary in $binaries) {
        $source = $binary.Source
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Expected release binary was not produced: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $output $binary.Name)
    }
    Compress-Archive -Path (Join-Path $uiBuild "*") -DestinationPath (Join-Path $output "audiorouter-ui.zip") -CompressionLevel Optimal
    Copy-Item -LiteralPath $uiLock -Destination (Join-Path $output "sbom.npm.package-lock.json")
    $npmSbom = Join-Path $output "sbom.npm.json"
    & node.exe (Join-Path $workspace "tools/release/generate-npm-sbom.mjs") $uiLock $npmSbom
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $npmSbom -PathType Leaf)) {
        throw "UI npm SBOM generation failed"
    }

    $metadata = & cargo metadata --locked --format-version 1
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed with exit code $LASTEXITCODE"
    }
    $metadataJson = $metadata -join [Environment]::NewLine
    $metadataJson | Set-Content -LiteralPath (Join-Path $output "sbom.cargo.json") -Encoding utf8
    $metadataObject = $metadataJson | ConvertFrom-Json
    $noticeLines = @(
        "AudioRouter dependency notices"
        "Generated from cargo metadata --locked at release preparation time."
        ""
    )
    foreach ($package in @($metadataObject.packages | Sort-Object name, version)) {
        $license = if ([string]::IsNullOrWhiteSpace($package.license)) { "license metadata unavailable" } else { $package.license }
        $source = if ([string]::IsNullOrWhiteSpace($package.source)) { "workspace" } else { $package.source }
        $noticeLines += "- $($package.name) $($package.version) - $license - $source"
    }
    $noticeLines | Set-Content -LiteralPath (Join-Path $output "THIRD-PARTY-NOTICES.txt") -Encoding utf8

    $files = Get-ChildItem -LiteralPath $output -File | Sort-Object Name
    $checksums = foreach ($file in $files) {
        $hash = Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256
        [ordered]@{ file = $file.Name; sha256 = $hash.Hash.ToLowerInvariant(); bytes = $file.Length }
    }
    $revision = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{40}$') {
        throw "could not determine the source revision for release provenance"
    }
    $rustcDetails = & rustc -vV
    if ($LASTEXITCODE -ne 0) {
        throw "could not determine the Rust compiler provenance"
    }
    $rustcVersion = ($rustcDetails | Where-Object { $_ -like "release: *" } | Select-Object -First 1) -replace '^release:\s*', ''
    $rustcHost = ($rustcDetails | Where-Object { $_ -like "host: *" } | Select-Object -First 1) -replace '^host:\s*', ''
    $cargoVersion = (& cargo --version).Trim()
    if ([string]::IsNullOrWhiteSpace($rustcVersion) -or [string]::IsNullOrWhiteSpace($rustcHost) -or [string]::IsNullOrWhiteSpace($cargoVersion)) {
        throw "Rust toolchain provenance is incomplete"
    }
    $manifest = [ordered]@{
        format = "audiorouter.release-preparation"
        schemaVersion = 1
        architecture = "x64"
        sourceRevision = $revision
        build = [ordered]@{
            profile = "release"
            target = $rustcHost
            rustc = $rustcVersion
            cargo = $cargoVersion
        }
        artifacts = $checksums
        signed = $false
        publicationReady = $false
        blockers = @(
            "production code signing credentials and certificate are required"
            "driver package/signing and Windows install qualification are not included"
            "installer and clean-machine acceptance remain pending"
        )
    }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output "release-manifest.json") -Encoding utf8
}
finally {
    if (Test-Path -LiteralPath $uiBuild) {
        Remove-Item -LiteralPath $uiBuild -Recurse -Force
    }
    Pop-Location
}

Write-Output "Prepared unsigned artifacts in $output"
