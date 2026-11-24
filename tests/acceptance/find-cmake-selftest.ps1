# Self-test of find-cmake.ps1 with throw-away folders; it builds nothing and
# touches no real CMake. Run: powershell -NoProfile -File tests\acceptance\find-cmake-selftest.ps1
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'find-cmake.ps1')

$root = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-find-cmake-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$failures = New-Object System.Collections.Generic.List[string]

function Check([string]$Name, [bool]$Condition) {
    if ($Condition) { Write-Output "ok   $Name" } else { Write-Output "FAIL $Name"; $failures.Add($Name) }
}
function Make-Exe([string]$Path) {
    New-Item -ItemType Directory -Path (Split-Path $Path) -Force | Out-Null
    Set-Content -LiteralPath $Path -Value 'fake' -Encoding ASCII
}
function Throws([scriptblock]$Block, [string]$Pattern) {
    try { & $Block | Out-Null } catch { return ($_.Exception.Message -match $Pattern) }
    return $false
}
$noPath = { $null }

try {
    $explicit = Join-Path $root 'explicit\cmake.exe'
    $viaEnv = Join-Path $root 'env\cmake.exe'
    $viaPath = Join-Path $root 'path\cmake.exe'
    $standardOld = Join-Path $root 'winget\cmake-4.4.0-windows-x86_64\bin\cmake.exe'
    $standardNew = Join-Path $root 'winget\cmake-4.10.1-windows-x86_64\bin\cmake.exe'
    $repoLocal = Join-Path $root 'repo\third_party\cmake-3.31.0\bin\cmake.exe'
    foreach ($file in @($explicit, $viaEnv, $viaPath, $standardOld, $standardNew, $repoLocal)) { Make-Exe $file }
    $patterns = @((Join-Path $root 'winget\cmake-*\bin\cmake.exe'))

    Check 'parameter beats environment, PATH and standard installs' ((Find-CMake -CMakePath $explicit -EnvironmentOverride $viaEnv -PathLookup { $viaPath } -StandardPatterns $patterns) -eq $explicit)
    Check 'environment override beats PATH' ((Find-CMake -EnvironmentOverride $viaEnv -PathLookup { $viaPath } -StandardPatterns $patterns) -eq $viaEnv)
    Check 'PATH beats the standard folders' ((Find-CMake -EnvironmentOverride '' -PathLookup { $viaPath } -StandardPatterns $patterns) -eq $viaPath)
    Check 'standard folders pick the newest version (4.10.1 over 4.4.0)' ((Find-CMake -EnvironmentOverride '' -PathLookup $noPath -StandardPatterns $patterns) -eq $standardNew)
    Check 'repository-local copy is the last resort' ((Find-CMake -EnvironmentOverride '' -PathLookup $noPath -StandardPatterns @() -RepositoryRoot (Join-Path $root 'repo')) -eq $repoLocal)
    Check 'a PATH entry that no longer exists is skipped' ((Find-CMake -EnvironmentOverride '' -PathLookup { Join-Path $root 'gone\cmake.exe' } -StandardPatterns $patterns) -eq $standardNew)

    Check 'a missing explicit path fails and names the source (no fallback to PATH)' (Throws { Find-CMake -CMakePath (Join-Path $root 'nope\cmake.exe') -EnvironmentOverride '' -PathLookup { $viaPath } -StandardPatterns $patterns } '-CMakePath.*not a file')
    Check 'a missing environment override fails and names the variable (no fallback)' (Throws { Find-CMake -EnvironmentOverride (Join-Path $root 'nope\cmake.exe') -PathLookup { $viaPath } -StandardPatterns $patterns } 'AUDIOROUTER_CMAKE.*not a file')
    Check 'nothing found gives the install hint and the places searched' (Throws { Find-CMake -EnvironmentOverride '' -PathLookup $noPath -StandardPatterns @((Join-Path $root 'missing\cmake-*\bin\cmake.exe')) -RepositoryRoot (Join-Path $root 'empty') } 'CMake was not found.*winget install Kitware\.CMake.*AUDIOROUTER_CMAKE.*missing\\cmake-\*')

    # The real machine: report what the default search finds (informational; a PC without CMake is not a defect here).
    try {
        $real = Find-CMake
        $version = (@(& $real --version) -join ' ')
        Check "default search finds a CMake that reports a version ($real)" ($version -match 'cmake version \d')
    } catch {
        Write-Output "note default search on this PC: $($_.Exception.Message)"
    }
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

if ($failures.Count -gt 0) { throw "find-cmake self-test failed: $($failures -join '; ')" }
Write-Output 'find-cmake self-test passed.'
