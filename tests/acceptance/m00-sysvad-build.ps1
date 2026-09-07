param(
    [Parameter(Mandatory = $true)]
    [string]$SourceRoot,
    [switch]$KeepArtifacts
)

$ErrorActionPreference = 'Stop'

$root = (Resolve-Path -LiteralPath $SourceRoot).Path
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
if (-not $root.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to build SysVAD outside the temporary directory: $root"
}

$solution = Join-Path $root 'audio\sysvad\sysvad.sln'
$wilHeader = Join-Path $root 'wil\include\wil\com.h'
foreach ($path in @($solution, $wilHeader)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "SysVAD source prerequisite is missing: $path"
    }
}

$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw "Visual Studio discovery tool is missing: $vswhere"
}
$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) {
    throw 'No Visual Studio installation with the native C++ workload was found'
}
$msbuild = Join-Path $installation 'MSBuild\Current\Bin\amd64\MSBuild.exe'
if (-not (Test-Path -LiteralPath $msbuild -PathType Leaf)) {
    throw "64-bit MSBuild is missing: $msbuild"
}

try {
    Push-Location $root
    & $msbuild $solution /m /t:Rebuild /p:Configuration=Release /p:Platform=x64 /p:ApiValidator_Enable=true /p:SkipPackageVerification=false /v:minimal
    if ($LASTEXITCODE -ne 0) {
        throw "SysVAD x64 validation build failed with exit code $LASTEXITCODE"
    }
    $package = Join-Path $root 'audio\sysvad\x64\Release\package'
    foreach ($name in @('TabletAudioSample.sys', 'sysvad.cat')) {
        if (-not (Test-Path -LiteralPath (Join-Path $package $name) -PathType Leaf)) {
            throw "Validated SysVAD package output is missing: $name"
        }
    }
    Write-Output 'M00 SysVAD x64 compile and package/API validation passed'
    Write-Output 'Scope: disposable source checkout; no driver installation, loading, test-signing mode, or machine audio configuration action.'
}
finally {
    Pop-Location
    if (-not $KeepArtifacts) {
        Get-ChildItem -LiteralPath $root -Directory -Recurse -Filter x64 -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue }
    }
}
