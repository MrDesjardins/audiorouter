[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,
    [string]$ReleaseTag,
    [string]$DriverPackage = $env:AUDIOROUTER_SIGNED_DRIVER_PACKAGE
)

$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if ([string]::IsNullOrWhiteSpace($DriverPackage)) {
    throw "Set AUDIOROUTER_SIGNED_DRIVER_PACKAGE to the Microsoft-signed Partner Center driver payload directory."
}
$driverInput = [IO.Path]::GetFullPath($DriverPackage)
$driverStage = Join-Path $workspace "target/release/driver"
$driverStageCreated = $false

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

. (Join-Path $PSScriptRoot "pe-validation.ps1")

function Assert-NoReparsePath {
    param([Parameter(Mandatory = $true)][string]$Path)
    $cursor = [IO.Path]::GetFullPath($Path)
    while (-not [string]::IsNullOrWhiteSpace($cursor)) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Release input path must not traverse a reparse point: $cursor"
            }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }
        $cursor = $parent
    }
}

Push-Location $workspace
$bundleRoot = Join-Path $workspace "src-tauri/target/release/bundle/nsis"
$cleanupNsis = $false
try {
    if (-not (Test-Path -LiteralPath $driverInput -PathType Container)) {
        throw "Microsoft-signed driver package directory is missing: $driverInput"
    }
    Assert-NoReparsePath $driverInput
    $driverFiles = @("audioroutervirtual.inf", "audioroutervirtual.sys", "audioroutervirtual.cat")
    foreach ($name in $driverFiles) {
        $file = Join-Path $driverInput $name
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw "Signed driver package is missing a regular $($name): $file"
        }
        Assert-NoReparsePath $file
    }
    if (Test-Path -LiteralPath $driverStage) {
        throw "Driver staging directory already exists; refusing to overwrite it: $driverStage"
    }
    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if (-not $signtool) { throw "signtool.exe is required to verify the Microsoft-signed driver catalog." }
    foreach ($member in @("audioroutervirtual.inf", "audioroutervirtual.sys")) {
        $verification = @(& $signtool.Source verify /kp /v /c (Join-Path $driverInput "audioroutervirtual.cat") (Join-Path $driverInput $member) 2>&1 | ForEach-Object { "$_" })
        $verifyExit = $LASTEXITCODE
        if ($verifyExit -ne 0 -or ($verification -join "`n") -notmatch "Microsoft Windows Hardware Compatibility Publisher") {
            throw "Production catalog verification failed for $($member): $($verification -join "`n")"
        }
    }
    $infText = Get-Content -LiteralPath (Join-Path $driverInput "audioroutervirtual.inf") -Raw
    $versionMatch = [regex]::Match($infText, '(?im)^DriverVer\s*=\s*\d{2}/\d{2}/\d{4},\s*(\d+\.\d+\.\d+\.\d+)\s*$')
    if (-not $versionMatch.Success) { throw "Driver INF has no valid four-part DriverVer." }
    New-Item -ItemType Directory -Path $driverStage | Out-Null
    $driverStageCreated = $true
    foreach ($name in $driverFiles) { Copy-Item -LiteralPath (Join-Path $driverInput $name) -Destination (Join-Path $driverStage $name) }
    Copy-Item -LiteralPath (Join-Path $workspace "drivers/audiorouter-virtual/LICENSE-MS-PL.txt") -Destination (Join-Path $driverStage "LICENSE-MS-PL.txt")
    $packageJson = [ordered]@{ platform = "x64"; signed = "microsoft"; driverVersion = $versionMatch.Groups[1].Value } | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $driverStage "package.json"), $packageJson, [Text.UTF8Encoding]::new($false))

    $dirty = & git status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0) {
        throw "could not inspect Git working-tree state"
    }
    if ($dirty) {
        throw "release inputs must come from a clean Git working tree"
    }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace "tools/release/stage-openai-tunnel-runtime.ps1") -Workspace $workspace -DestinationDirectory (Join-Path $workspace "target/release")
    if ($LASTEXITCODE -ne 0) {
        throw "verified OpenAI tunnel runtime staging failed"
    }
    & cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host
    if ($LASTEXITCODE -ne 0) {
        throw "cargo release build failed with exit code $LASTEXITCODE"
    }
    & cargo build --release --locked -p audiorouter-driver-helper
    if ($LASTEXITCODE -ne 0) { throw "elevated driver helper release build failed with exit code $LASTEXITCODE" }
    & npm.cmd run build --prefix ui
    if ($LASTEXITCODE -ne 0) {
        throw "UI build for native shell embedding failed with exit code $LASTEXITCODE"
    }
    & cargo build --release --locked --features custom-protocol --manifest-path (Join-Path $workspace "src-tauri/Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "native shell release build failed with exit code $LASTEXITCODE"
    }

    if (Test-Path -LiteralPath $bundleRoot) {
        throw "NSIS output directory already exists; refusing to overwrite it: $bundleRoot"
    }
    # The installer bundles the Stream Deck plugin (tauri.release.conf.json
    # resources), so pack it before the NSIS build.
    & npm.cmd ci --prefix tools/streamdeck
    if ($LASTEXITCODE -ne 0) { throw "Stream Deck locked dependency installation failed" }
    & npm.cmd run typecheck --prefix tools/streamdeck
    if ($LASTEXITCODE -ne 0) { throw "Stream Deck typecheck failed" }
    & npm.cmd test --prefix tools/streamdeck
    if ($LASTEXITCODE -ne 0) { throw "Stream Deck tests failed" }
    & npm.cmd run pack --prefix tools/streamdeck
    if ($LASTEXITCODE -ne 0) { throw "Stream Deck validation and packing failed" }
    $pluginPackage = Join-Path $workspace "tools/streamdeck/dist/com.mrdesjardins.audiorouter.streamDeckPlugin"
    if (-not (Test-Path -LiteralPath $pluginPackage -PathType Leaf)) {
        throw "Stream Deck package is missing: $pluginPackage"
    }
    $cleanupNsis = $true
    $tauriCli = Join-Path $workspace "ui/node_modules/.bin/tauri.cmd"
    if (-not (Test-Path -LiteralPath $tauriCli -PathType Leaf)) {
        throw "Tauri CLI is missing; install locked UI dependencies with npm ci --prefix ui: $tauriCli"
    }
    & $tauriCli build --no-sign --ci --bundles nsis --config src-tauri/tauri.release.conf.json
    if ($LASTEXITCODE -ne 0) {
        throw "unsigned per-user NSIS bundle build failed with exit code $LASTEXITCODE"
    }
    $uiLock = Join-Path $workspace "ui/package-lock.json"
    if (-not (Test-Path -LiteralPath $uiLock -PathType Leaf)) {
        throw "UI lockfile is missing: $uiLock"
    }
    & node.exe -e "JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8'))" -- $uiLock
    if ($LASTEXITCODE -ne 0) {
        throw "UI lockfile is not valid JSON: $uiLock"
    }
    $shellConfig = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
    $appVersion = [string]$shellConfig.version
    $cargoText = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/Cargo.toml") -Raw
    $cargoVersionMatch = [regex]::Match($cargoText, '(?m)^version\s*=\s*"([^"]+)"')
    if ([string]::IsNullOrWhiteSpace($appVersion) -or
        -not $cargoVersionMatch.Success -or
        $cargoVersionMatch.Groups[1].Value -ne $appVersion -or
        $appVersion -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
        throw "release application version is missing, invalid, or mismatched between Tauri and Cargo metadata"
    }
    $uiPackage = Get-Content -LiteralPath (Join-Path $workspace "ui/package.json") -Raw | ConvertFrom-Json
    if ($uiPackage.version -ne $appVersion) {
        throw "release application version is mismatched between Tauri and UI package metadata"
    }
    if (-not [string]::IsNullOrWhiteSpace($ReleaseTag)) {
        if ($ReleaseTag -ne "v$appVersion") {
            throw "release tag must be v$appVersion"
        }
        $tagCommitResult = & git rev-parse --verify "$ReleaseTag^{commit}"
        if ($LASTEXITCODE -ne 0) {
            throw "release tag does not exist in this checkout: $ReleaseTag"
        }
        $tagCommit = ($tagCommitResult | Select-Object -First 1).Trim()
        $headCommit = (& git rev-parse HEAD).Trim()
        if ($LASTEXITCODE -ne 0 -or $tagCommit -ne $headCommit) {
            throw "release tag must already exist and point at the checked-out source commit"
        }
    }

    New-Item -ItemType Directory -Path $output | Out-Null

    $binaries = @(
        @{ Name = "audiorouter-cli.exe"; Source = (Join-Path $workspace "target/release/audiorouter-cli.exe") }
        @{ Name = "audiorouter-plugin-worker.exe"; Source = (Join-Path $workspace "target/release/audiorouter-plugin-worker.exe") }
        @{ Name = "audiorouter-driver-helper.exe"; Source = (Join-Path $workspace "target/release/audiorouter-driver-helper.exe") }
        @{ Name = "audiorouter-shell.exe"; Source = (Join-Path $workspace "src-tauri/target/release/audiorouter-shell.exe") }
    )
    foreach ($binary in $binaries) {
        $source = $binary.Source
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Expected release binary was not produced: $source"
        }
        # These ship inside the installer; they are not separate release assets.
        Assert-X64PortableExecutable $source
    }
    $installerName = "AudioRouter_${appVersion}_x64-setup.exe"
    $installerSource = Join-Path $workspace "src-tauri/target/release/bundle/nsis/$installerName"
    if (-not (Test-Path -LiteralPath $installerSource -PathType Leaf)) {
        throw "Tauri did not produce the per-user NSIS installer: $installerSource"
    }
    if ((Get-Item -LiteralPath $installerSource).Length -le 0) {
        throw "Tauri produced an empty NSIS installer: $installerSource"
    }
    Copy-Item -LiteralPath $installerSource -Destination (Join-Path $output $installerName)
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
    $noticeLines += @(
        ""
        "Bundled data"
        "- Head-related impulse responses (surround to headphones) derived from the MIT Media Lab KEMAR set:"
        "  Bill Gardner and Keith Martin, ""HRTF Measurements of a KEMAR Dummy-Head Microphone"","
        "  MIT Media Lab Perceptual Computing Technical Report #280, 1994. Copyright 1994 MIT Media Laboratory;"
        "  provided free with no restrictions on use, provided the authors are cited."
        "  https://sound.media.mit.edu/resources/KEMAR.html"
        ""
        "OpenAI Secure MCP Tunnel runtime"
        "- tunnel-client-runtime v0.0.16, Apache-2.0; LICENSE, NOTICE, license report and SPDX SBOM are included in this release."
        "- Official source: https://github.com/openai/tunnel-client/releases/tag/v0.0.16"
    )
    $noticeLines | Set-Content -LiteralPath (Join-Path $output "THIRD-PARTY-NOTICES.txt") -Encoding utf8

    Copy-Item -LiteralPath $pluginPackage -Destination $output
    foreach ($name in @("tunnel-client-runtime-LICENSE.txt", "tunnel-client-runtime-NOTICE.txt", "tunnel-client-runtime-licenses.txt", "tunnel-client-runtime.spdx.json")) {
        $source = Join-Path $workspace "target/release/$name"
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Verified OpenAI tunnel runtime metadata is missing from the release stage: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $output $name)
    }
    Copy-Item -LiteralPath (Join-Path $workspace "tools/streamdeck/package-lock.json") -Destination (Join-Path $output "sbom.streamdeck.package-lock.json")
    & node.exe (Join-Path $workspace "tools/release/generate-npm-sbom.mjs") (Join-Path $workspace "tools/streamdeck/package-lock.json") (Join-Path $output "sbom.streamdeck.json")
    if ($LASTEXITCODE -ne 0) { throw "Stream Deck npm SBOM generation failed" }

    foreach ($name in @("audioroutervirtual.inf", "audioroutervirtual.sys", "audioroutervirtual.cat", "LICENSE-MS-PL.txt")) {
        Copy-Item -LiteralPath (Join-Path $driverStage $name) -Destination (Join-Path $output $name)
    }
    $files = Get-ChildItem -LiteralPath $output -File | Sort-Object Name
    $checksums = @(foreach ($file in $files) {
        $hash = Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256
        [ordered]@{ file = $file.Name; sha256 = $hash.Hash.ToLowerInvariant(); bytes = $file.Length }
    })
    $checksumLines = @($checksums | ForEach-Object { "$($_.sha256) *$($_.file)" })
    $checksumPath = Join-Path $output "SHA256SUMS.txt"
    $checksumLines | Set-Content -LiteralPath $checksumPath -Encoding utf8
    $checksumHash = Get-FileHash -LiteralPath $checksumPath -Algorithm SHA256
    $checksums += [ordered]@{ file = "SHA256SUMS.txt"; sha256 = $checksumHash.Hash.ToLowerInvariant(); bytes = (Get-Item -LiteralPath $checksumPath).Length }
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
        version = $appVersion
        releaseTag = if ([string]::IsNullOrWhiteSpace($ReleaseTag)) { $null } else { $ReleaseTag }
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
            "app and installer are intentionally unsigned for the first release; disclose the publisher trust state"
            "standard-user install, upgrade, repair, uninstall, and clean-machine acceptance remain pending"
            "endpoint and hardware acceptance remains specific to supported existing-device environments"
        )
    }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output "release-manifest.json") -Encoding utf8
}
finally {
    if ($cleanupNsis -and (Test-Path -LiteralPath $bundleRoot)) {
        $bundleItem = Get-Item -LiteralPath $bundleRoot -Force
        if (-not $bundleItem.PSIsContainer -or (($bundleItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
            throw "generated NSIS bundle path is not a regular directory: $bundleRoot"
        }
        Remove-Item -LiteralPath $bundleRoot -Recurse -Force
    }
    if ($driverStageCreated -and (Test-Path -LiteralPath $driverStage)) {
        $stageItem = Get-Item -LiteralPath $driverStage -Force
        if (-not $stageItem.PSIsContainer -or (($stageItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
            throw "generated driver staging path is not a regular directory: $driverStage"
        }
        Remove-Item -LiteralPath $driverStage -Recurse -Force
    }
    Pop-Location
}

Write-Output "Prepared unsigned artifacts in $output"
