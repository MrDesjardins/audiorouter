$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-probe-{0}.exe" -f ([guid]::NewGuid()))

if (Test-Path -LiteralPath $object) {
    throw "refusing native acceptance build because generated object already exists: $object"
}

try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0) {
        throw "native WASAPI probe build failed with exit code $LASTEXITCODE"
    }
    if (-not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw "native WASAPI probe build did not produce the expected executable: $output"
    }
    Write-Output 'M00 native probe compile acceptance passed'
    Write-Output 'Scope: compile-only validation; no audio stream, driver, signing mode, or machine configuration action.'
}
finally {
    Remove-Item -LiteralPath $output, $object -Force -ErrorAction SilentlyContinue
}
