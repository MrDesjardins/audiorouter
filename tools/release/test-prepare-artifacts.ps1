$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-release-prepare-' + [guid]::NewGuid().ToString('N'))
$prepare = Join-Path $PSScriptRoot 'prepare-artifacts.ps1'
try {
    New-Item -ItemType Directory -Path $root | Out-Null

    $missing = Join-Path $root 'missing\output'
    try {
        & $prepare -OutputDirectory $missing | Out-Null
        throw 'prepare script accepted an output directory with a missing parent'
    } catch {
        if ($_.Exception.Message -eq 'prepare script accepted an output directory with a missing parent') { throw }
        if ($_.Exception.Message -notmatch 'parent must already exist') { throw }
    }

    $target = Join-Path $root 'target'
    New-Item -ItemType Directory -Path $target | Out-Null
    $link = Join-Path $root 'redirected-parent'
    try {
        New-Item -ItemType SymbolicLink -Path $link -Target $target -ErrorAction Stop | Out-Null
        try {
            & $prepare -OutputDirectory (Join-Path $link 'output') | Out-Null
            throw 'prepare script accepted a reparse-point output parent'
        } catch {
            if ($_.Exception.Message -eq 'prepare script accepted a reparse-point output parent') { throw }
            if ($_.Exception.Message -notmatch 'parent must not be a reparse point') { throw }
        }
    } catch {
        if ($_.Exception.Message -notmatch 'privilege|symbolic|not permitted|cannot create') { throw }
    }

    Write-Output 'Release preparation path-safety tests passed'
}
finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
