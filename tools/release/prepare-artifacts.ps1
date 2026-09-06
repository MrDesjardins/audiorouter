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

Push-Location $workspace
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

    New-Item -ItemType Directory -Path $output | Out-Null

    $binaries = @(
        "audiorouter-cli.exe",
        "audiorouter-plugin-worker.exe"
    )
    foreach ($binary in $binaries) {
        $source = Join-Path $workspace "target/release/$binary"
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Expected release binary was not produced: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $output $binary)
    }

    $metadata = & cargo metadata --locked --format-version 1
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed with exit code $LASTEXITCODE"
    }
    $metadata -join [Environment]::NewLine | Set-Content -LiteralPath (Join-Path $output "sbom.cargo.json") -Encoding utf8

    $files = Get-ChildItem -LiteralPath $output -File | Sort-Object Name
    $checksums = foreach ($file in $files) {
        $hash = Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256
        [ordered]@{ file = $file.Name; sha256 = $hash.Hash.ToLowerInvariant(); bytes = $file.Length }
    }
    $revision = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{40}$') {
        throw "could not determine the source revision for release provenance"
    }
    $manifest = [ordered]@{
        format = "audiorouter.release-preparation"
        schemaVersion = 1
        architecture = "x64"
        sourceRevision = $revision
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
    Pop-Location
}

Write-Output "Prepared unsigned artifacts in $output"
