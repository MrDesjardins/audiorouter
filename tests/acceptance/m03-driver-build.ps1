$ErrorActionPreference = 'Stop'

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$build = Join-Path $workspace 'drivers/audiorouter-virtual/build.ps1'

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $build
if ($LASTEXITCODE -ne 0) {
    throw "AudioRouter virtual-driver build failed with exit code $LASTEXITCODE"
}

Write-Output 'M03 AudioRouter virtual-driver build acceptance passed'
Write-Output 'Scope: project-owned x64 WDK compile/signability/catalog qualification only; no installation, loading, signing-mode, boot-policy, service, or audio configuration action.'
