[CmdletBinding()]
param([string]$DriverPackage = $env:AUDIOROUTER_SIGNED_DRIVER_PACKAGE)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$hookChecks = Join-Path $repositoryRoot 'tools/release/test-installer-hooks.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hookChecks
if ($LASTEXITCODE -ne 0) { throw "WP-11 installer resource/hook checks failed: $LASTEXITCODE" }

if ([string]::IsNullOrWhiteSpace($DriverPackage)) {
    Write-Output 'SKIP: no Microsoft-signed driver package supplied; static installer wiring checks passed, NSIS generation and VM install qualification were not run.'
    return
}

$outputParent = Join-Path $repositoryRoot 'target'
$output = Join-Path $outputParent ('m08-installer-smoke-' + [guid]::NewGuid().ToString('N'))
$prepare = Join-Path $repositoryRoot 'tools/release/prepare-artifacts.ps1'
$verify = Join-Path $repositoryRoot 'tools/release/verify-artifacts.ps1'
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $prepare -OutputDirectory $output -DriverPackage $DriverPackage
    if ($LASTEXITCODE -ne 0) { throw "signed-driver NSIS preparation failed: $LASTEXITCODE" }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verify -ManifestPath (Join-Path $output 'release-manifest.json')
    if ($LASTEXITCODE -ne 0) { throw "prepared artifact verification failed: $LASTEXITCODE" }

    $manifest = Get-Content -LiteralPath (Join-Path $output 'release-manifest.json') -Raw | ConvertFrom-Json
    foreach ($name in @('audioroutervirtual.inf', 'audioroutervirtual.sys', 'audioroutervirtual.cat', 'LICENSE-MS-PL.txt')) {
        if (@($manifest.artifacts | Where-Object { $_.file -eq $name }).Count -ne 1) {
            throw "release manifest must contain exactly one $name entry"
        }
    }
    $installerName = "AudioRouter_$($manifest.version)_x64-setup.exe"
    if (-not (Test-Path -LiteralPath (Join-Path $output $installerName) -PathType Leaf)) {
        throw "release preparation did not produce the NSIS installer: $installerName"
    }
    Write-Output 'M08 signed-driver installer preparation smoke passed. The installer was not executed; VM installation, update and uninstall remain separate gates.'
}
finally {
    if (Test-Path -LiteralPath $output) {
        $resolved = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $output).Path)
        if ([IO.Path]::GetDirectoryName($resolved) -ne [IO.Path]::GetFullPath($outputParent)) {
            throw "Refusing to remove smoke output outside target: $resolved"
        }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
