[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-release-verify-" + [guid]::NewGuid().ToString("N"))
$verifier = Join-Path $PSScriptRoot "verify-artifacts.ps1"
try {
    New-Item -ItemType Directory -Path $root | Out-Null
    $artifactPath = Join-Path $root "sample.bin"
    [IO.File]::WriteAllBytes($artifactPath, [byte[]](1, 2, 3, 5, 8))
    $noticePath = Join-Path $root "THIRD-PARTY-NOTICES.txt"
    Set-Content -LiteralPath $noticePath -Value "- sample 1.0 - MIT - workspace" -Encoding utf8
    $sbomPath = Join-Path $root "sbom.cargo.json"
    @{ packages = @() } | ConvertTo-Json | Set-Content -LiteralPath $sbomPath -Encoding utf8
    $hash = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $noticeHash = (Get-FileHash -LiteralPath $noticePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $sbomHash = (Get-FileHash -LiteralPath $sbomPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = [ordered]@{
        format = "audiorouter.release-preparation"
        schemaVersion = 1
        architecture = "x64"
        sourceRevision = ("a" * 40)
        build = [ordered]@{ profile = "release"; target = "x86_64-pc-windows-msvc"; rustc = "1.96.0"; cargo = "cargo 1.96.0" }
        artifacts = @(
            [ordered]@{ file = "sample.bin"; sha256 = $hash; bytes = 5 }
            [ordered]@{ file = "sbom.cargo.json"; sha256 = $sbomHash; bytes = (Get-Item -LiteralPath $sbomPath).Length }
            [ordered]@{ file = "THIRD-PARTY-NOTICES.txt"; sha256 = $noticeHash; bytes = (Get-Item -LiteralPath $noticePath).Length }
        )
        signed = $false
        publicationReady = $false
        blockers = @("test blocker")
    }
    $manifestPath = Join-Path $root "release-manifest.json"
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    & $verifier $manifestPath | Out-Null

    [IO.File]::AppendAllText($artifactPath, "tamper")
    try {
        & $verifier $manifestPath | Out-Null
        throw "verifier accepted a tampered artifact"
    } catch {
        if ($_.Exception.Message -eq "verifier accepted a tampered artifact") {
            throw
        }
    }

    $extraPath = Join-Path $root "unlisted.bin"
    [IO.File]::WriteAllBytes($extraPath, [byte[]](13, 21, 34))
    try {
        & $verifier $manifestPath | Out-Null
        throw "verifier accepted an unlisted package entry"
    } catch {
        if ($_.Exception.Message -eq "verifier accepted an unlisted package entry") {
            throw
        }
    } finally {
        Remove-Item -LiteralPath $extraPath -Force
    }

    $withoutNotices = [ordered]@{
        format = "audiorouter.release-preparation"
        schemaVersion = 1
        architecture = "x64"
        sourceRevision = ("c" * 40)
        artifacts = @([ordered]@{ file = "sample.bin"; sha256 = $hash; bytes = 5 })
        signed = $false
        publicationReady = $false
        blockers = @("test blocker")
    }
    $withoutNoticesPath = Join-Path $root "without-notices-manifest.json"
    $withoutNotices | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $withoutNoticesPath -Encoding utf8
    try {
        & $verifier $withoutNoticesPath | Out-Null
        throw "verifier accepted a manifest without dependency notices"
    } catch {
        if ($_.Exception.Message -eq "verifier accepted a manifest without dependency notices") {
            throw
        }
    }

    $linkPath = Join-Path $root "linked.bin"
    try {
        New-Item -ItemType SymbolicLink -Path $linkPath -Target $noticePath -ErrorAction Stop | Out-Null
        $linkHash = (Get-FileHash -LiteralPath $linkPath -Algorithm SHA256).Hash.ToLowerInvariant()
        $linkManifest = [ordered]@{
            format = "audiorouter.release-preparation"
            schemaVersion = 1
            architecture = "x64"
            sourceRevision = ("b" * 40)
            artifacts = @([ordered]@{ file = "linked.bin"; sha256 = $linkHash; bytes = (Get-Item -LiteralPath $linkPath).Length })
            signed = $false
            publicationReady = $false
            blockers = @("test blocker")
        }
        $linkManifestPath = Join-Path $root "linked-release-manifest.json"
        $linkManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $linkManifestPath -Encoding utf8
        try {
            & $verifier $linkManifestPath | Out-Null
            throw "verifier accepted a reparse-point artifact"
        } catch {
            if ($_.Exception.Message -eq "verifier accepted a reparse-point artifact") {
                throw
            }
        }
    } catch {
        if ($_.Exception.Message -notmatch "privilege|symbolic|not permitted|cannot create") {
            throw
        }
    }
    Write-Output "Release artifact verifier tests passed"
}
finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
