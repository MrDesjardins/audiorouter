[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Workspace,
    [Parameter(Mandatory = $true)]
    [string]$DestinationDirectory
)

$ErrorActionPreference = "Stop"
$version = "0.0.16"
$assetName = "tunnel-client-runtime-v$version-windows-amd64.zip"
$assetUrl = "https://github.com/openai/tunnel-client/releases/download/v$version/$assetName"
$expectedSha256 = "639fdfe10d91c672d8f8a49386ebf39acaac4e560a88134735b1739a02ddb82d"
$workspacePath = [IO.Path]::GetFullPath($Workspace)
$targetRoot = [IO.Path]::GetFullPath((Join-Path $workspacePath "target"))
$destination = [IO.Path]::GetFullPath($DestinationDirectory)
if (-not $destination.StartsWith($targetRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Tunnel runtime destination must be inside the workspace target directory."
}

$downloadDirectory = Join-Path $targetRoot ("openai-tunnel-runtime-v{0}" -f $version)
$archive = Join-Path $downloadDirectory $assetName
$stage = Join-Path $targetRoot ("openai-tunnel-runtime-stage-{0}" -f [Guid]::NewGuid().ToString("N"))
if (-not (Test-Path -LiteralPath $targetRoot -PathType Container)) {
    New-Item -ItemType Directory -Path $targetRoot | Out-Null
}
if (-not (Test-Path -LiteralPath $downloadDirectory -PathType Container)) {
    New-Item -ItemType Directory -Path $downloadDirectory | Out-Null
}
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
    Invoke-WebRequest -Uri $assetUrl -OutFile $archive
}
$actualSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha256 -ne $expectedSha256) {
    throw "OpenAI tunnel runtime archive checksum did not match the pinned v$version release."
}
$gh = Get-Command gh.exe -ErrorAction SilentlyContinue
if (-not $gh) { $gh = Get-Command gh -ErrorAction SilentlyContinue }
if (-not $gh) { throw "GitHub CLI is required to verify OpenAI tunnel runtime provenance." }
& $gh.Source attestation verify $archive --repo openai/tunnel-client
if ($LASTEXITCODE -ne 0) { throw "OpenAI tunnel runtime provenance verification failed." }

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName.Replace('\', '/')
        if ([IO.Path]::IsPathRooted($name) -or $name -match '(^|/)\.\.?(/|$)' -or $name.Contains(':')) {
            throw "OpenAI tunnel runtime archive contains an unsafe path."
        }
    }
} finally {
    $zip.Dispose()
}

New-Item -ItemType Directory -Path $stage | Out-Null
try {
    Expand-Archive -LiteralPath $archive -DestinationPath $stage
    $required = @(
        "tunnel-client-runtime.exe",
        "LICENSE",
        "NOTICE",
        "tunnel-client-runtime-v$version-windows-amd64-licenses.txt",
        "tunnel-client-runtime-v$version-windows-amd64.spdx.json"
    )
    foreach ($name in $required) {
        $source = Join-Path $stage $name
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "OpenAI tunnel runtime release is missing required file: $name"
        }
        $item = Get-Item -LiteralPath $source -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "OpenAI tunnel runtime archive extracted a redirected file: $name"
        }
    }
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    $copies = @(
        @{ Source = "tunnel-client-runtime.exe"; Target = "tunnel-client-runtime.exe" }
        @{ Source = "LICENSE"; Target = "tunnel-client-runtime-LICENSE.txt" }
        @{ Source = "NOTICE"; Target = "tunnel-client-runtime-NOTICE.txt" }
        @{ Source = "tunnel-client-runtime-v$version-windows-amd64-licenses.txt"; Target = "tunnel-client-runtime-licenses.txt" }
        @{ Source = "tunnel-client-runtime-v$version-windows-amd64.spdx.json"; Target = "tunnel-client-runtime.spdx.json" }
    )
    foreach ($copy in $copies) {
        Copy-Item -LiteralPath (Join-Path $stage $copy.Source) -Destination (Join-Path $destination $copy.Target) -Force
    }
    $binary = Join-Path $destination "tunnel-client-runtime.exe"
    if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath (Join-Path $stage "tunnel-client-runtime.exe") -Algorithm SHA256).Hash) {
        Write-Output "Staged verified OpenAI tunnel-client-runtime v$version ($expectedSha256)."
    } else {
        throw "Staged OpenAI tunnel runtime does not match the verified archive."
    }
} finally {
    $stagePath = [IO.Path]::GetFullPath($stage)
    if (-not $stagePath.StartsWith($targetRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a tunnel staging directory outside target."
    }
    if (Test-Path -LiteralPath $stagePath) {
        Remove-Item -LiteralPath $stagePath -Recurse -Force
    }
}
