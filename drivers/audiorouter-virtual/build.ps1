[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Release',
    [ValidateSet('x64', 'ARM64')]
    [string] $Platform = 'x64',
    [string] $Version = '0.1.0',
    [string] $Output,
    [switch] $KeepOutput,
    [switch] $TestSign
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'package-tools.ps1')
$driverVersion = ConvertTo-DriverVersion $Version
if ($TestSign -and $Platform -ne 'x64') { throw 'Test signing currently supports only x64.' }
$driverRoot = (Resolve-Path (Join-Path $PSScriptRoot '.')).Path
$solution = Join-Path $driverRoot 'AudioRouterVirtual.sln'
$outputWasProvided = [bool]$Output

if (-not $Output) {
    $Output = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-virtual-driver-' + [guid]::NewGuid().ToString('N'))
}
$output = Assert-DriverPackagePath $Output
$outputExistedBeforeBuild = Test-Path -LiteralPath $output
$platformOutputRoot = Join-Path $driverRoot $Platform
if ($outputExistedBeforeBuild -and @(Get-ChildItem -LiteralPath $output -Force).Count -ne 0) {
    throw 'Output must be an empty directory; choose a fresh output directory.'
}
foreach ($name in @('audioroutervirtual.inf', 'audioroutervirtual.sys', 'audioroutervirtual.cat', 'package.json', 'AudioRouterTest.cer', 'LICENSE-MS-PL.txt')) {
    if (Test-Path -LiteralPath (Join-Path $output $name)) { throw "Refusing to overwrite package file $name; choose a fresh output directory." }
}
New-Item -ItemType Directory -Path $output -Force | Out-Null

function Remove-DisposableOutput {
    if ($KeepOutput) { return }
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $outputIsUnderTemp = $output.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        $output.TrimEnd('\') -ne $tempRoot.TrimEnd('\')
    if (-not $outputWasProvided -and $outputIsUnderTemp -and -not $outputExistedBeforeBuild -and
        (Test-Path -LiteralPath $output)) {
        Remove-Item -LiteralPath $output -Recurse -Force
        Write-Host 'Removed disposable build output.'
    } elseif ($outputWasProvided -or $outputExistedBeforeBuild) {
        Write-Host 'Preserved caller-owned build output; use an automatic temporary output for cleanup.'
    }
}

function Find-MSBuild {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $install = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and $install) {
            $candidate = Join-Path $install.Trim() 'MSBuild\Current\Bin\MSBuild.exe'
            if (Test-Path -LiteralPath $candidate) { return $candidate }
        }
    }
    $command = Get-Command msbuild.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw 'MSBuild.exe was not found. Install the VS C++ workload and WDK build tools.'
}

try {
$msbuild = Find-MSBuild
$wdkTargets = @(Get-ChildItem -Path ${env:ProgramFiles(x86)}, ${env:ProgramFiles} -Filter 'WindowsDriver.Default.props' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1)
if (-not $wdkTargets) {
    throw 'WindowsDriver.Default.props was not found. Install the WDK build tools.'
}

$log = Join-Path $output 'build.log'
$buildArguments = @(
    $solution,
    '/t:Rebuild',
    '/m:1',
    '/nr:false',
    "/p:Configuration=$Configuration",
    "/p:Platform=$Platform",
    '/p:SkipPackageVerification=true',
    '/p:ApiValidator_Enable=false',
    '/p:SignMode=Off',
    '/p:TrackFileAccess=false',
    '/v:minimal',
    "/flp:LogFile=$log;Verbosity=normal"
)
Write-Host "Build-only AudioRouter virtual driver qualification"
Write-Host "MSBuild: $msbuild"
Write-Host "WDK target: $($wdkTargets[0].FullName)"
# Normalize duplicate Path/PATH in this build process only. Invoke MSBuild with
# an argument array: spaces and shell metacharacters in output paths stay data.
$buildSearchPath = $env:Path
[Environment]::SetEnvironmentVariable('PATH', $null, 'Process')
[Environment]::SetEnvironmentVariable('Path', $buildSearchPath, 'Process')
& $msbuild @buildArguments
if ($LASTEXITCODE -ne 0) { throw "MSBuild failed with exit code $LASTEXITCODE. See $log" }

# Stage only this build's package, never an arbitrary recursive search result.
$builtPackage = Join-Path $platformOutputRoot "$Configuration\package"
Write-Host "Driver binary: $(Join-Path $builtPackage 'audioroutervirtual.sys')"
Write-Host "Driver INF: $(Join-Path $builtPackage 'audioroutervirtual.inf')"
foreach ($name in @('audioroutervirtual.inf', 'audioroutervirtual.sys')) {
    $artifact = Join-Path $builtPackage $name
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) { throw "Missing freshly built artifact: $artifact" }
    Copy-Item -LiteralPath $artifact -Destination (Join-Path $output $name)
}
$stampinf = Find-DriverTool 'stampinf.exe'
$date = (Get-Date).ToString('MM/dd/yyyy', [Globalization.CultureInfo]::InvariantCulture)
$null = Invoke-DriverTool $stampinf @('-f', (Join-Path $output 'audioroutervirtual.inf'), '-d', $date, '-v', $driverVersion) (Join-Path $output 'stampinf.log')
$stampedInf = Get-Content -LiteralPath (Join-Path $output 'audioroutervirtual.inf') -Raw
if ($stampedInf -notmatch ('(?im)^DriverVer\s*=\s*' + [regex]::Escape($date) + ',\s*' + [regex]::Escape($driverVersion) + '\s*$')) {
    throw 'StampInf did not write the requested date/version; check StampInf environment overrides.'
}
Copy-Item -LiteralPath (Join-Path $driverRoot 'LICENSE-MS-PL.txt') -Destination $output
$git = Invoke-DriverTool 'git.exe' @('-C', $driverRoot, 'rev-parse', 'HEAD') ''
$dirty = Invoke-DriverTool 'git.exe' @('-C', $driverRoot, 'status', '--porcelain') ''
$metadata = [ordered]@{ version = $Version; driverVersion = $driverVersion; builtAt = [DateTime]::UtcNow.ToString('o'); gitCommit = $git.Output.Trim(); dirty = [bool]$dirty.Output; platform = $Platform; configuration = $Configuration; signed = 'unsigned' }
Write-DriverPackageMetadata $output $metadata
if ($TestSign) {
    & (Join-Path $driverRoot 'sign-test.ps1') -Package $output
} else {
    $inf2cat = Find-DriverTool 'Inf2Cat.exe'
    $os = if ($Platform -eq 'x64') { '10_X64' } else { '10_CO_ARM64' }
    $null = Invoke-DriverTool $inf2cat @("/driver:$output", "/os:$os", '/uselocaltime') (Join-Path $output 'inf2cat.log')
}
Write-Host "Staged package: $output"
if (-not $TestSign) { Write-Host 'No installation, signing, boot-policy, service, or audio-device action was performed.' }
else { Write-Host 'No installation, host trust, boot-policy, service, or audio-device action was performed.' }
Remove-DisposableOutput
} catch {
    Remove-DisposableOutput
    throw
}
