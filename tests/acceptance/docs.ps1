$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $workspace
try {
    node .\tools\docs\validate.mjs
    if ($LASTEXITCODE -ne 0) { throw "documentation validation failed" }
    Write-Output 'Documentation acceptance passed'
}
finally {
    Pop-Location
}
