[CmdletBinding(DefaultParameterSetName = 'Install')]
param(
    [Parameter(Mandatory = $true, ParameterSetName = 'Install')]
    [switch] $Install,
    [Parameter(Mandatory = $true, ParameterSetName = 'Uninstall')]
    [switch] $Uninstall,
    [switch] $Preview,
    [switch] $AllowDriverInstall,
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string] $Inf,
    [string] $State = (Join-Path ([IO.Path]::GetDirectoryName((Resolve-Path $Inf).Path)) 'audiorouter-driver-state.json')
)

$ErrorActionPreference = 'Stop'
$maxStateBytes = 64KB
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
    $currentPath = [IO.Path]::GetFullPath($Path)
    $stopPath = [IO.Path]::GetFullPath($StopAt)
    while ($null -ne $currentPath) {
        $current = Get-Item -LiteralPath $currentPath -Force
        if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "The driver package path cannot contain a reparse point: $($current.FullName)"
        }
        if ([string]::Equals($currentPath.TrimEnd('\'), $stopPath.TrimEnd('\'),
                [StringComparison]::OrdinalIgnoreCase)) {
            return
        }
        $parent = [IO.Directory]::GetParent($currentPath)
        $currentPath = if ($null -eq $parent) { $null } else { $parent.FullName }
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

function Read-BoundedState {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )
    $item = Get-Item -LiteralPath $Path -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "The lifecycle state file cannot be a reparse point: $($item.FullName)"
    }
    if ($item.Length -gt $maxStateBytes) {
        throw "The lifecycle state file exceeds the $maxStateBytes byte limit: $($item.FullName)"
    }
    return [IO.File]::ReadAllText($item.FullName)
}

if (-not $Preview -and -not $AllowDriverInstall) {
    throw 'Driver lifecycle changes require -AllowDriverInstall in addition to -Install or -Uninstall.'
}
if (-not [IO.Path]::IsPathRooted($statePath)) {
    throw 'State must be an absolute path.'
}
if (-not $infPath.StartsWith($driverRootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The INF must be inside the AudioRouter driver package directory.'
}
if (-not [string]::Equals([IO.Path]::GetExtension($infPath), '.inf',
        [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The lifecycle package input must be a generated .inf file.'
}
Assert-NoReparsePath -Path $infPath -StopAt $driverRoot
Assert-NoReparseAncestors -Path (Split-Path -Parent $statePath)

$pnputil = Join-Path $env:WINDIR 'System32\pnputil.exe'
if (-not (Test-Path -LiteralPath $pnputil -PathType Leaf)) {
    throw "pnputil.exe was not found at $pnputil."
}

if ($Preview) {
    $action = if ($Install) { 'install' } else { 'uninstall' }
    $plan = [ordered]@{
        schemaVersion = 1
        action = $action
        mutates = $false
        ready = $true
        inf = $infPath
        state = $statePath
        tool = $pnputil
        requiredConsent = 'AllowDriverInstall for execution'
        consentProvided = [bool]$AllowDriverInstall
    }
    if ($Install) {
        $plan.statePresent = Test-Path -LiteralPath $statePath -PathType Leaf
        if ($plan.statePresent) {
            $plan.ready = $false
            $plan.blocker = 'lifecycle state already exists; install would overwrite ownership state'
        }
        $plan.command = @('/add-driver', $infPath, '/install')
    } else {
        $plan.statePresent = Test-Path -LiteralPath $statePath -PathType Leaf
        if (-not $plan.statePresent) {
            $plan.ready = $false
            $plan.blocker = 'no lifecycle state exists; uninstall refuses package-wide deletion'
        } else {
            try {
                Assert-NoReparsePath -Path $statePath -StopAt (Split-Path -Parent $statePath)
                $previewRecord = Read-BoundedState -Path $statePath | ConvertFrom-Json
                if ($previewRecord.schemaVersion -ne 1 -or
                    [string]::IsNullOrWhiteSpace($previewRecord.publishedName) -or
                    $previewRecord.publishedName -notmatch '^oem\d+\.inf$') {
                    throw 'lifecycle state is invalid or ambiguous'
                }
                if ([string]::IsNullOrWhiteSpace($previewRecord.inf) -or
                    -not [string]::Equals((Resolve-Path -LiteralPath $previewRecord.inf).Path, $infPath,
                        [StringComparison]::OrdinalIgnoreCase)) {
                    throw 'lifecycle state INF does not match the requested INF'
                }
                $plan.publishedName = $previewRecord.publishedName
                $plan.command = @('/delete-driver', $previewRecord.publishedName, '/uninstall')
            } catch {
                $plan.ready = $false
                $plan.blocker = $_.Exception.Message
            }
        }
    }
    Write-Output ($plan | ConvertTo-Json -Depth 4)
    exit 0
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
    try {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
        Assert-NoReparseAncestors -Path $parent
        $temporaryStatePath = Join-Path $parent ('.audiorouter-driver-state.' + [guid]::NewGuid().ToString('N') + '.tmp')
        $record | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $temporaryStatePath -Encoding UTF8 -NoNewline
        Move-Item -LiteralPath $temporaryStatePath -Destination $statePath
    } catch {
        if ($temporaryStatePath -and (Test-Path -LiteralPath $temporaryStatePath)) {
            Remove-Item -LiteralPath $temporaryStatePath -Force -ErrorAction SilentlyContinue
        }
        $rollback = & $pnputil '/delete-driver' $published '/uninstall' 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Lifecycle state publication failed and automatic driver rollback also failed (exit $LASTEXITCODE): $($rollback -join ' ')"
        }
        throw "Lifecycle state publication failed; the newly installed package $published was rolled back: $($_.Exception.Message)"
    }
    Write-Output "Installed $published and recorded rollback state at $statePath"
    exit 0
}

if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) {
    throw "No lifecycle state exists at $statePath; refusing package-wide deletion."
}
Assert-NoReparsePath -Path $statePath -StopAt (Split-Path -Parent $statePath)
$record = Read-BoundedState -Path $statePath | ConvertFrom-Json
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
