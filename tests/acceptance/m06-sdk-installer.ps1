$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$installer = Join-Path $repositoryRoot 'tools\m06-vst3-sdk\install.ps1'
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-sdk-origin-" + [guid]::NewGuid().ToString('N'))

try {
    New-Item -ItemType Directory -Path $fixture | Out-Null
    & git init --quiet $fixture
    if ($LASTEXITCODE -ne 0) { throw 'unable to create disposable Git fixture' }
    & git -C $fixture remote add origin 'https://example.invalid/not-vst3sdk.git'
    if ($LASTEXITCODE -ne 0) { throw 'unable to configure disposable Git fixture' }

    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installer -Destination $fixture 2>&1 | Out-String
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
    if ($LASTEXITCODE -eq 0) {
        throw 'installer accepted a checkout with the wrong origin'
    }
    if ($output -notmatch 'origin mismatch') {
        throw "installer rejected the fixture for an unexpected reason: $output"
    }

    $safeParent = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-sdk-safe-" + [guid]::NewGuid().ToString('N'))
    $linkedParent = Join-Path $fixture 'redirected-parent'
    try {
        New-Item -ItemType Directory -Path $safeParent | Out-Null
        New-Item -ItemType SymbolicLink -Path $linkedParent -Target $safeParent -ErrorAction Stop | Out-Null
        $redirectedDestination = Join-Path $linkedParent 'sdk'
        $previousErrorAction = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $redirectedOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installer -Destination $redirectedDestination 2>&1 | Out-String
        } finally {
            $ErrorActionPreference = $previousErrorAction
        }
        if ($LASTEXITCODE -eq 0 -or $redirectedOutput -notmatch 'Destination parent must not be a reparse point') {
            throw 'installer accepted a destination below a reparse-point parent'
        }
    } catch {
        if ($_.Exception.Message -notmatch 'privilege|symbolic|not permitted|cannot create') {
            throw
        }
    } finally {
        if (Test-Path -LiteralPath $safeParent) {
            Remove-Item -LiteralPath $safeParent -Recurse -Force
        }
    }
    Write-Output 'M06 SDK installer provenance acceptance passed'
} finally {
    if (Test-Path -LiteralPath $fixture) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}

Write-Output 'Scope: disposable Git metadata only; no SDK, plugin, driver, or audio configuration changes.'
