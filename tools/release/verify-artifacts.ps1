[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ManifestPath
)

$ErrorActionPreference = "Stop"
$manifestFile = (Resolve-Path -LiteralPath $ManifestPath -ErrorAction Stop).Path
$root = Split-Path -Parent $manifestFile
$manifest = Get-Content -LiteralPath $manifestFile -Raw | ConvertFrom-Json

if ($manifest.format -ne "audiorouter.release-preparation" -or $manifest.schemaVersion -ne 1) {
    throw "unsupported release manifest format or schema version"
}
if ($manifest.architecture -ne "x64" -or $manifest.sourceRevision -notmatch '^[0-9a-f]{40}$') {
    throw "release manifest has invalid architecture or source revision"
}
if ($manifest.signed -ne $false -or $manifest.publicationReady -ne $false) {
    throw "unsigned preparation manifest cannot claim signed or publication-ready status"
}
if (@($manifest.blockers).Count -eq 0) {
    throw "unsigned preparation manifest must retain explicit blockers"
}

$sbom = Join-Path $root "sbom.cargo.json"
if (-not (Test-Path -LiteralPath $sbom -PathType Leaf)) {
    throw "release SBOM is missing: $sbom"
}
$null = Get-Content -LiteralPath $sbom -Raw | ConvertFrom-Json

$names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($entry in @($manifest.artifacts)) {
    if ([string]::IsNullOrWhiteSpace($entry.file) -or $entry.file -match '[\\/]' -or $entry.file -in @('.', '..')) {
        throw "release artifact name must be a single safe filename: $($entry.file)"
    }
    if (-not $names.Add($entry.file)) {
        throw "duplicate release artifact entry: $($entry.file)"
    }
    if ($entry.sha256 -notmatch '^[0-9a-f]{64}$' -or [uint64]$entry.bytes -lt 1) {
        throw "invalid checksum or byte count for release artifact: $($entry.file)"
    }
    $artifact = Join-Path $root $entry.file
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        throw "release artifact is missing: $artifact"
    }
    $actual = Get-FileHash -LiteralPath $artifact -Algorithm SHA256
    if ($actual.Hash -ne $entry.sha256.ToUpperInvariant()) {
        throw "checksum mismatch for release artifact: $($entry.file)"
    }
    if ((Get-Item -LiteralPath $artifact).Length -ne [uint64]$entry.bytes) {
        throw "byte count mismatch for release artifact: $($entry.file)"
    }
}
if ($names.Count -eq 0) {
    throw "release manifest contains no artifacts"
}

Write-Output "Verified unsigned artifacts in $root"
