# Finds the CMake used by the VST3 SDK acceptance script. Dot-source this file.
#
# Order (the first hit wins; an explicit choice that does not exist is an error
# and never falls through to another CMake):
#   1. -CMakePath
#   2. the AUDIOROUTER_CMAKE environment variable (the path of cmake.exe)
#   3. cmake.exe on PATH
#   4. standard installs: Program Files\CMake, the winget package folder and the
#      CMake bundled with Visual Studio
#   5. a repository-local third_party\cmake-*\bin\cmake.exe (newest version)

function Get-NewestCMake([string[]]$Patterns) {
    foreach ($pattern in $Patterns) {
        if ([string]::IsNullOrWhiteSpace($pattern)) { continue }
        $found = @(Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue)
        if ($found.Count -gt 0) {
            # Newest version folder first: sort by the folder names as versions where possible.
            $best = $found | Sort-Object -Descending -Property @{ Expression = {
                    $match = [regex]::Match($_.FullName, 'cmake-(\d+(?:\.\d+){1,3})')
                    if ($match.Success) { [version]$match.Groups[1].Value } else { [version]'0.0' }
                } }, FullName | Select-Object -First 1
            return $best.FullName
        }
    }
    return $null
}

function Find-CMake(
    [string]$CMakePath,
    [string]$EnvironmentOverride = $env:AUDIOROUTER_CMAKE,
    [scriptblock]$PathLookup = { (Get-Command -Name 'cmake.exe' -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source },
    [string[]]$StandardPatterns,
    [string]$RepositoryRoot
) {
    if ($null -eq $StandardPatterns) {
        $StandardPatterns = @()
        foreach ($programFiles in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
            if ($programFiles) {
                $StandardPatterns += (Join-Path $programFiles 'CMake\bin\cmake.exe')
                $StandardPatterns += (Join-Path $programFiles 'Microsoft Visual Studio\*\*\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe')
            }
        }
        if ($env:LOCALAPPDATA) {
            $StandardPatterns += (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\Kitware.CMake_*\cmake-*\bin\cmake.exe')
        }
    }

    foreach ($choice in @(
            @{ Value = $CMakePath; Source = '-CMakePath' },
            @{ Value = $EnvironmentOverride; Source = 'the AUDIOROUTER_CMAKE environment variable' })) {
        if (-not [string]::IsNullOrWhiteSpace($choice.Value)) {
            if (-not (Test-Path -LiteralPath $choice.Value -PathType Leaf)) {
                throw "CMake was chosen with $($choice.Source) but is not a file: $($choice.Value)"
            }
            return (Resolve-Path -LiteralPath $choice.Value).Path
        }
    }

    $onPath = & $PathLookup
    if (-not [string]::IsNullOrWhiteSpace($onPath) -and (Test-Path -LiteralPath $onPath -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $onPath).Path
    }

    $standard = Get-NewestCMake $StandardPatterns
    if ($standard) { return $standard }

    $searched = @($StandardPatterns)
    if ($RepositoryRoot) {
        $local = Join-Path $RepositoryRoot 'third_party\cmake-*\bin\cmake.exe'
        $searched += $local
        $found = Get-NewestCMake @($local)
        if ($found) { return $found }
    }

    throw ("CMake was not found. Install it (for example: winget install Kitware.CMake), put cmake.exe on PATH, " +
        "or set AUDIOROUTER_CMAKE to the full path of cmake.exe. Looked at PATH and: " + ($searched -join '; '))
}
