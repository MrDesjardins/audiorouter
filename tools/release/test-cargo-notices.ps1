param([switch]$RealMetadata)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "cargo-notices.ps1")
function Package($name, $version = "1.0.0", $license = "MIT", $source = "registry+fixture") {
    [PSCustomObject]@{ name = $name; version = $version; license = $license; source = $source }
}
$root = [PSCustomObject]@{ packages = @((Package "shared"), (Package "root-only"), (Package "dual")) }
$shell = [PSCustomObject]@{ packages = @((Package "shared"), (Package "tauri"), (Package "dual" "2.0.0"), (Package "local" "1.0.0" "" "")) }
$lines = @(Get-CargoPackageNotices -MetadataObjects @($root, $shell))
if ($lines.Count -ne 6) { throw "Missing shell packages or duplicate shared packages: $lines" }
foreach ($name in @("root-only", "tauri", "shared", "dual 1.0.0", "dual 2.0.0")) {
    if (-not ($lines -match [regex]::Escape($name))) { throw "Missing $name" }
}
if ($lines -notcontains "- local 1.0.0 - license metadata unavailable - workspace") { throw "Missing fallback labels" }
$reverse = @(Get-CargoPackageNotices -MetadataObjects @($shell, $root))
if (($lines -join "`n") -ne ($reverse -join "`n")) { throw "Input order changed notices" }
$conflict = [PSCustomObject]@{ packages = @((Package "shared" "1.0.0" "Apache-2.0")) }
$rejected = $false
try { Get-CargoPackageNotices -MetadataObjects @($root, $conflict) | Out-Null } catch { $rejected = $_.Exception.Message -like "Conflicting license*" }
if (-not $rejected) { throw "Conflicting licenses were silently discarded" }
Write-Output "PASS: root and shell notices, shared deduplication, versions, fallback labels and conflict detection"
if ($RealMetadata) {
    $workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
    $rootJson = & cargo metadata --locked --offline --format-version 1 --manifest-path (Join-Path $workspace "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "Root metadata failed" }
    $shellJson = & cargo metadata --locked --offline --format-version 1 --manifest-path (Join-Path $workspace "src-tauri/Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "Shell metadata failed" }
    $actual = @(Get-CargoPackageNotices -MetadataObjects @(($rootJson | ConvertFrom-Json), ($shellJson | ConvertFrom-Json)))
    foreach ($name in @("tauri", "chacha20", "cipher", "inout")) {
        if (-not ($actual -match "^- $name ")) { throw "Real combined notices omit $name" }
    }
    $actual | Set-Content -LiteralPath (Join-Path $workspace "target/review-cargo-notices.txt") -Encoding utf8
    Write-Output "PASS: real locked root and shell metadata, $($actual.Count) combined packages"
}
