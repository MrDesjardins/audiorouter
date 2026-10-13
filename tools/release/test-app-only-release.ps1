$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$config = Get-Content (Join-Path $repo 'src-tauri/tauri.app-only.release.conf.json') -Raw | ConvertFrom-Json
if ($config.bundle.windows.nsis.installMode -ne 'currentUser' -or $config.bundle.windows.nsis.installerHooks) {
    throw 'App-only installer must be current-user without cable uninstall hooks'
}
foreach ($resource in $config.bundle.resources.PSObject.Properties) {
    if ($resource.Name -match 'driver' -or $resource.Value -match 'driver') { throw 'App-only installer includes driver resources' }
}
$root = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-app-only-test-' + [guid]::NewGuid().ToString('N'))
$verifier = Join-Path $PSScriptRoot 'verify-artifacts.ps1'
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $names = @('AudioRouter_0.0.16_x64-setup.exe', 'sbom.cargo.json', 'sbom.npm.json', 'sbom.npm.package-lock.json',
        'com.mrdesjardins.audiorouter.streamDeckPlugin', 'sbom.streamdeck.json', 'sbom.streamdeck.package-lock.json',
        'THIRD-PARTY-NOTICES.txt', 'tunnel-client-runtime-LICENSE.txt', 'tunnel-client-runtime-NOTICE.txt',
        'tunnel-client-runtime-licenses.txt', 'tunnel-client-runtime.spdx.json')
    $entries = @(foreach ($name in $names) {
        $path = Join-Path $root $name
        [IO.File]::WriteAllText($path, '{}')
        [ordered]@{ file = $name; bytes = 2; sha256 = (Get-FileHash $path).Hash.ToLowerInvariant() }
    })
    $sums = Join-Path $root 'SHA256SUMS.txt'
    @($entries | ForEach-Object { "$($_.sha256) *$($_.file)" }) | Set-Content $sums -Encoding utf8
    $entries += [ordered]@{ file = 'SHA256SUMS.txt'; bytes = (Get-Item $sums).Length; sha256 = (Get-FileHash $sums).Hash.ToLowerInvariant() }
    $manifest = [ordered]@{
        format = 'audiorouter.release-preparation'; schemaVersion = 1; version = '0.0.16'; releaseTag = 'v0.0.16';
        architecture = 'x64'; sourceRevision = ('a' * 40); includesDriver = $false;
        build = @{ profile = 'release'; target = 'x86_64-pc-windows-msvc'; rustc = '1.96.0'; cargo = 'cargo 1.96.0' };
        artifacts = $entries; signed = $false; publicationReady = $false; blockers = @('experimental test fixture')
    }
    $manifestPath = Join-Path $root 'release-manifest.json'
    $manifest | ConvertTo-Json -Depth 5 | Set-Content $manifestPath -Encoding utf8
    & $verifier $manifestPath | Out-Null
    $manifest.includesDriver = $true
    $manifest | ConvertTo-Json -Depth 5 | Set-Content $manifestPath -Encoding utf8
    try {
        & $verifier $manifestPath | Out-Null
        throw 'Driver release accepted missing driver assets'
    } catch { if ($_.Exception.Message -notmatch 'missing required artifact: audioroutervirtual.inf') { throw } }
    $manifest.includesDriver = 'false'
    $manifest | ConvertTo-Json -Depth 5 | Set-Content $manifestPath -Encoding utf8
    try {
        & $verifier $manifestPath | Out-Null
        throw 'Verifier accepted a string driver flag'
    } catch { if ($_.Exception.Message -notmatch 'includesDriver must be a boolean') { throw } }
    $manifest.includesDriver = $false
    $driver = Join-Path $root 'audioroutervirtual.sys'
    [IO.File]::WriteAllText($driver, '{}')
    $manifest.artifacts += [ordered]@{ file = 'audioroutervirtual.sys'; bytes = 2; sha256 = (Get-FileHash $driver).Hash.ToLowerInvariant() }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content $manifestPath -Encoding utf8
    try {
        & $verifier $manifestPath | Out-Null
        throw 'App-only release accepted driver content'
    } catch { if ($_.Exception.Message -notmatch 'app-only release contains driver content') { throw } }
    Write-Output 'App-only profile and manifest regression checks passed'
} finally {
    $resolved = [IO.Path]::GetFullPath($root)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath()), [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Unexpected fixture cleanup path'
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
