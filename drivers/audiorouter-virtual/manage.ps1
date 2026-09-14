[CmdletBinding(DefaultParameterSetName = 'Install')]
param(
    [Parameter(Mandatory = $true, ParameterSetName = 'Install')]
    [switch] $Install,
    [Parameter(Mandatory = $true, ParameterSetName = 'Uninstall')]
    [switch] $Uninstall,
    [Parameter(Mandatory = $true)]
    [switch] $AllowDriverInstall,
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string] $Inf,
    [string] $State = (Join-Path ([IO.Path]::GetDirectoryName((Resolve-Path $Inf).Path)) 'audiorouter-driver-state.json')
)

$ErrorActionPreference = 'Stop'
$infPath = (Resolve-Path -LiteralPath $Inf).Path
$statePath = [IO.Path]::GetFullPath($State)
$driverRoot = (Resolve-Path (Join-Path $PSScriptRoot '.')).Path.TrimEnd('\')
$driverRootPrefix = $driverRoot + '\'

function Assert-NoReparsePath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path,
        [Parameter(Mandatory = $true)]
        [string] $StopAt
    )
    $current = Get-Item -LiteralPath $Path -Force
    while ($null -ne $current) {
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "The driver package path cannot contain a reparse point: $($current.FullName)"
        }
        if ([string]::Equals($current.FullName.TrimEnd('\'), $StopAt.TrimEnd('\'),
                [StringComparison]::OrdinalIgnoreCase)) {
            return
        }
        $current = $current.Parent
    }
    throw "The driver package path does not resolve beneath $StopAt."
}

function Assert-NoReparseAncestors {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )
    $current = [IO.DirectoryInfo]::new([IO.Path]::GetFullPath($Path))
    while ($null -ne $current) {
        if (Test-Path -LiteralPath $current.FullName) {
            $item = Get-Item -LiteralPath $current.FullName -Force
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "The lifecycle state path cannot contain a reparse point: $($item.FullName)"
            }
        }
        $current = $current.Parent
    }
}

if (-not $AllowDriverInstall) {
    throw 'Driver lifecycle changes require -AllowDriverInstall in addition to -Install or -Uninstall.'
}
if (-not [IO.Path]::IsPathRooted($statePath)) {
    throw 'State must be an absolute path.'
}
if (-not $infPath.StartsWith($driverRootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The INF must be inside the AudioRouter driver package directory.'
}
Assert-NoReparsePath -Path $infPath -StopAt $driverRoot
Assert-NoReparseAncestors -Path (Split-Path -Parent $statePath)

$pnputil = Join-Path $env:WINDIR 'System32\pnputil.exe'
if (-not (Test-Path -LiteralPath $pnputil -PathType Leaf)) {
    throw "pnputil.exe was not found at $pnputil."
}

if ($Install) {
    if (Test-Path -LiteralPath $statePath -PathType Leaf) {
        throw "Refusing to overwrite existing lifecycle state: $statePath"
    }
    $result = & $pnputil '/add-driver' $infPath '/install' 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "pnputil failed to install the package (exit $LASTEXITCODE): $($result -join ' ')"
    }
    $published = [regex]::Match(($result -join "`n"), '(?im)Published Name\s*:\s*(oem\d+\.inf)').Groups[1].Value
    if ([string]::IsNullOrWhiteSpace($published)) {
        throw 'Installation succeeded but pnputil did not report a published package name; refusing unmanaged cleanup.'
    }
    $record = [ordered]@{
        schemaVersion = 1
        installedAtUtc = [DateTime]::UtcNow.ToString('O')
        inf = $infPath
        publishedName = $published
    }
    $parent = Split-Path -Parent $statePath
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $record | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $statePath -Encoding UTF8 -NoNewline
    Write-Output "Installed $published and recorded rollback state at $statePath"
    exit 0
}

if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) {
    throw "No lifecycle state exists at $statePath; refusing package-wide deletion."
}
$record = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
if ($record.schemaVersion -ne 1 -or [string]::IsNullOrWhiteSpace($record.publishedName) -or
    $record.publishedName -notmatch '^oem\d+\.inf$') {
    throw "Lifecycle state is invalid or ambiguous: $statePath"
}
if ([string]::IsNullOrWhiteSpace($record.inf) -or
    -not [string]::Equals((Resolve-Path -LiteralPath $record.inf).Path, $infPath,
        [StringComparison]::OrdinalIgnoreCase)) {
    throw "Lifecycle state INF does not match the requested INF: $statePath"
}
$result = & $pnputil '/delete-driver' $record.publishedName '/uninstall'
if ($LASTEXITCODE -ne 0) {
    throw "pnputil failed to uninstall $($record.publishedName) (exit $LASTEXITCODE): $($result -join ' ')"
}
Remove-Item -LiteralPath $statePath -Force
Write-Output "Uninstalled $($record.publishedName) and removed rollback state."
