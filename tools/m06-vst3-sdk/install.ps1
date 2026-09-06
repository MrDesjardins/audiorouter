param(
    [string]$Destination = (Join-Path $PSScriptRoot '..\..\third_party\vst3sdk'),
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

# Git's submodule helper is a POSIX shell script. Some managed PowerShell
# launchers expose only Git's cmd directory, so provide Git's private helper
# directories for this process without changing the user's persistent PATH.
$gitCommand = (Get-Command git -ErrorAction Stop).Source
$gitRoot = Split-Path -Parent (Split-Path -Parent $gitCommand)
$env:Path = "$gitRoot\cmd;$gitRoot\usr\bin;$gitRoot\mingw64\bin;$env:Path"

$repository = 'https://github.com/steinbergmedia/vst3sdk.git'
$revision = '3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96'
$destinationPath = [System.IO.Path]::GetFullPath($Destination)

function Invoke-Git {
    param([string[]]$Arguments)
    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Update-Submodules {
    $bash = Join-Path $gitRoot 'usr\bin\bash.exe'
    $cygpath = Join-Path $gitRoot 'usr\bin\cygpath.exe'
    if (-not (Test-Path -LiteralPath $bash) -or -not (Test-Path -LiteralPath $cygpath)) {
        throw 'Git for Windows bash/cygpath is required to initialize the SDK submodules'
    }
    $posixDestination = (& $cygpath -u $destinationPath).Trim()
    & $bash -lc "git -C '$posixDestination' submodule update --init --recursive --depth 1"
    if ($LASTEXITCODE -ne 0) {
        throw 'SDK submodule initialization failed'
    }
}

if (Test-Path -LiteralPath $destinationPath) {
    if (-not (Test-Path -LiteralPath (Join-Path $destinationPath '.git'))) {
        throw "Destination exists but is not a git checkout: $destinationPath"
    }
    $current = (& git -C $destinationPath rev-parse HEAD).Trim()
    if ($current -ne $revision) {
        if (-not $Force) {
            throw "SDK checkout is at $current; rerun with -Force to replace it"
        }
        Invoke-Git @('-C', $destinationPath, 'fetch', '--depth', '1', 'origin', $revision)
        Invoke-Git @('-C', $destinationPath, 'checkout', '--detach', $revision)
    }
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destinationPath) | Out-Null
    Invoke-Git @('clone', '--filter=blob:none', '--no-checkout', $repository, $destinationPath)
    Invoke-Git @('-C', $destinationPath, 'checkout', '--detach', $revision)
}

Update-Submodules

$required = @(
    'CMakeLists.txt',
    'pluginterfaces/base/ipluginbase.h',
    'public.sdk/source/vst/hosting/plugprovider.h'
)
foreach ($relativePath in $required) {
    $path = Join-Path $destinationPath $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "SDK checkout is incomplete; missing $relativePath"
    }
}

Write-Output "VST3 SDK ready at $destinationPath"
Write-Output "Revision: $((& git -C $destinationPath rev-parse HEAD).Trim())"
Write-Output 'Scope: repository-local source SDK; no global installation or audio configuration changes.'
