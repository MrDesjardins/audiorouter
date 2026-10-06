[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v\d+\.\d+\.\d+$')]
    [string]$Tag,
    [Parameter(Mandatory = $true)]
    [string]$NotesPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$notes = [IO.Path]::GetFullPath($NotesPath)
$notesItem = Get-Item -LiteralPath $notes -Force -ErrorAction Stop
if ($notesItem.PSIsContainer -or (($notesItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
    throw "release notes must be a regular, non-reparse file: $notes"
}
if ($notesItem.Length -lt 1 -or $notesItem.Length -gt 100KB) {
    throw "release notes must be between 1 byte and 100 KiB"
}

Push-Location $workspace
try {
    $dirty = & git status --porcelain --untracked-files=all
    if ($LASTEXITCODE -ne 0) { throw "could not inspect Git working-tree state" }
    if ($dirty) { throw "draft release inputs must come from a clean Git working tree" }

    $config = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
    if ($Tag -ne "v$($config.version)") { throw "release tag must match the app version v$($config.version)" }
    $tagCommit = (& git rev-parse --verify "$Tag^{commit}").Trim()
    if ($LASTEXITCODE -ne 0) { throw "release tag does not exist locally: $Tag" }
    $headCommit = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $tagCommit -ne $headCommit) {
        throw "checked-out source must be exactly the existing release tag commit"
    }

    $remote = (& git remote get-url origin).Trim()
    if ($LASTEXITCODE -ne 0 -or $remote -notmatch 'github\.com[:/](?<slug>[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+?)(?:\.git)?$') {
        throw "origin must identify a GitHub owner/repository"
    }
    $repository = $Matches.slug
    $null = Get-Command gh -ErrorAction Stop
    & gh auth status --hostname github.com
    if ($LASTEXITCODE -ne 0) { throw "GitHub CLI is not authenticated" }
    # A missing release is the expected case and gh reports it on stderr.
    # Under Windows PowerShell 5.1 a redirected native stderr line becomes an
    # error record, which "Stop" would turn into a failure, so relax it here.
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & gh release view $Tag --repo $repository *> $null
        $releaseExists = $LASTEXITCODE -eq 0
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($releaseExists) { throw "a GitHub release already exists for $Tag; refusing to replace it" }

    $output = [IO.Path]::GetFullPath($OutputDirectory)
    $outputParent = Split-Path -Parent $output
    if (-not (Test-Path -LiteralPath $outputParent -PathType Container)) {
        throw "release output parent must already exist: $outputParent"
    }
    if (Test-Path -LiteralPath $output) {
        throw "release output directory already exists; refusing to overwrite: $output"
    }

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "prepare-artifacts.ps1") -OutputDirectory $output -ReleaseTag $Tag
    if ($LASTEXITCODE -ne 0) { throw "release artifact preparation failed with exit code $LASTEXITCODE" }
    $manifestPath = Join-Path $output "release-manifest.json"
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "verify-artifacts.ps1") -ManifestPath $manifestPath
    if ($LASTEXITCODE -ne 0) { throw "release artifact verification failed with exit code $LASTEXITCODE" }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.releaseTag -ne $Tag -or $manifest.sourceRevision -ne $headCommit -or $manifest.publicationReady -ne $false) {
        throw "release manifest provenance does not match the reviewed tag or draft-only state"
    }

    $assets = @($manifest.artifacts | ForEach-Object { Join-Path $output $_.file }) + @($manifestPath)
    # Every release page opens with the same download guide, so testers know
    # the installer is the only file they need.
    $version = $manifest.version
    $downloadGuide = @"
## Which file do I download?

**Download ``AudioRouter_${version}_x64-setup.exe`` and run it.** That is all you need: it installs the app (window, tray and audio backend), the command-line tool, the plugin worker and the Stream Deck plugin for your Windows account, without administrator rights. Windows may warn that the installer is unsigned; choose **More info**, then **Run anyway**.

- **Stream Deck:** after installing, open AudioRouter, go to **API**, start the API and choose **Install Stream Deck plugin**. ``com.mrdesjardins.audiorouter.streamDeckPlugin`` is the same plugin, for a Stream Deck on another PC that controls AudioRouter over your network.
- **Everything else** (``SHA256SUMS.txt``, ``release-manifest.json``, ``sbom.*``, ``THIRD-PARTY-NOTICES.txt``) is for verifying the download and for license review. You do not need it to use AudioRouter.

"@
    $releaseNotes = Join-Path ([IO.Path]::GetTempPath()) "audiorouter-release-notes-$PID.md"
    if (Test-Path -LiteralPath $releaseNotes) { throw "temporary release notes file already exists: $releaseNotes" }
    # Windows PowerShell 5.1's "-Encoding utf8" writes a BOM, which GitHub
    # keeps in the body and which stops the first heading from rendering.
    # Write BOM-less UTF-8 and keep a blank line between the guide and notes.
    $combinedNotes = $downloadGuide.TrimEnd() + "`n`n" + (Get-Content -LiteralPath $notes -Raw).TrimStart([char]0xFEFF)
    [IO.File]::WriteAllText($releaseNotes, $combinedNotes, (New-Object Text.UTF8Encoding($false)))
    $ghArguments = @(
        "release", "create", $Tag,
        "--repo", $repository,
        "--verify-tag", "--draft",
        "--title", "AudioRouter $Tag",
        "--notes-file", $releaseNotes
    ) + $assets
    if ($PSCmdlet.ShouldProcess("GitHub release $Tag in $repository", "Create draft and upload verified artifacts")) {
        & gh @ghArguments
        if ($LASTEXITCODE -ne 0) { throw "GitHub draft release creation failed with exit code $LASTEXITCODE" }
        Write-Output "Created draft GitHub release $Tag. Review it and publish it manually when the M08 release gates are satisfied."
    }
    Write-Output "Verified release artifacts: $output"
}
finally {
    if ($releaseNotes -and (Test-Path -LiteralPath $releaseNotes)) { Remove-Item -LiteralPath $releaseNotes -Force }
    Pop-Location
}
