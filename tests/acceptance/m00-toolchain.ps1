param()

$ErrorActionPreference = 'Stop'
$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw "Visual Studio discovery tool is missing: $vswhere"
}

$installation = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $installation) { throw 'No Visual Studio installation with the native C++ workload was found' }
$msvc = Get-ChildItem -LiteralPath (Join-Path $installation 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $msvc) { throw "MSVC tools are missing from $installation" }

$kits = 'C:\Program Files (x86)\Windows Kits\10'
$sdk = Get-ChildItem -LiteralPath (Join-Path $kits 'Include') -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'um\Windows.h') } |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $sdk) { throw "Windows SDK headers are missing from $kits" }
$sdkVersion = $sdk.Name
$required = @(
    (Join-Path $msvc.FullName 'bin\Hostx64\x64\cl.exe'),
    (Join-Path $msvc.FullName 'include'),
    (Join-Path $kits "Lib\$sdkVersion\um\x64\Mmdevapi.lib"),
    (Join-Path $kits "DesignTime\CommonConfiguration\Neutral\WDK\$sdkVersion\WDK.props"),
    (Join-Path $kits "bin\$sdkVersion\x64\stampinf.exe")
)
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Required toolchain component is missing: $path" }
}

$kitLine = if ($sdkVersion -match '^10\.0\.(\d+)\.') { $Matches[1] } else { '' }
if ($kitLine -ne '28000') { throw "Unexpected SDK/WDK kit line: $sdkVersion" }

Write-Output "M00 toolchain compatibility acceptance passed: VS=$installation MSVC=$($msvc.Name) SDK=$sdkVersion WDK=matching-$kitLine"
Write-Output 'Scope: read-only toolchain discovery; no SDK installation, driver action, signing-mode change, or audio configuration action.'
