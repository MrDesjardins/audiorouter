$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$uiRoot = Join-Path $repositoryRoot 'ui'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ('audiorouter-ui-build-' + [guid]::NewGuid().ToString('N'))

function Invoke-Npm([string[]]$Arguments) {
    Push-Location $uiRoot
    try {
        & npm.cmd @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "npm $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

try {
    Invoke-Npm @('run', 'typecheck')
    Invoke-Npm @('test')
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    Invoke-Npm @('exec', 'vite', '--', 'build', '--configLoader', 'runner', '--outDir', $output)
    $fileCount = (Get-ChildItem -LiteralPath $output -Recurse -File).Count
    if ($fileCount -lt 1) {
        throw 'temporary UI build produced no files'
    }
    Write-Output "M05 UI acceptance passed: typecheck, tests, and temporary production build ($fileCount files)."
} finally {
    if (Test-Path -LiteralPath $output) {
        Remove-Item -LiteralPath $output -Recurse -Force
    }
}

Write-Output 'Scope: UI-only validation; no audio, driver, or machine configuration changes.'
