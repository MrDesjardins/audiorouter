[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-release-verify-" + [guid]::NewGuid().ToString("N"))
$verifier = Join-Path $PSScriptRoot "verify-artifacts.ps1"
try {
    New-Item -ItemType Directory -Path $root | Out-Null
    $artifactPath = Join-Path $root "sample.bin"
    [IO.File]::WriteAllBytes($artifactPath, [byte[]](1, 2, 3, 5, 8))
    @{ packages = @() } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root "sbom.cargo.json") -Encoding utf8
    $hash = (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = [ordered]@{
        format = "audiorouter.release-preparation"
        schemaVersion = 1
        architecture = "x64"
        sourceRevision = ("a" * 40)
        artifacts = @([ordered]@{ file = "sample.bin"; sha256 = $hash; bytes = 5 })
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
    Write-Output "Release artifact verifier tests passed"
}
finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
