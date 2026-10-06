[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [string] $Package,
    [string] $Evidence = ('C:\ar\evidence\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N')),
    [switch] $KeepInstalled,
    [switch] $CreateRootDevice,
    [string] $Devcon,
    [ValidateSet('Prototype', 'Cables')] [string] $EndpointProfile = 'Prototype',
    [string] $Cli
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'm03-driver-vm-support.ps1')
# Identity first: the host refusal occurs BEFORE even reading boot configuration.
Assert-DriverTestVmIdentity $env:COMPUTERNAME (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)
$driverRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../drivers/audiorouter-virtual')).Path
. (Join-Path $driverRoot 'package-tools.ps1')
$boot = Invoke-DriverTool (Join-Path $env:WINDIR 'System32\bcdedit.exe') @('/enum', '{current}') '' -AllowFailure
Assert-DriverTestSigning $boot.ExitCode $boot.Output
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run this script in administrator PowerShell inside the VM.' }
$Package = Assert-DriverPackagePath $Package
$metadata = Read-DriverPackage $Package
if ($metadata.signed -ne 'test') { throw 'Stage A needs a verified WP-02 test-signed package.' }
$verification = Get-Content -LiteralPath (Join-Path $Package 'signature-verification.json') -Raw | ConvertFrom-Json
foreach ($name in @('audioroutervirtual.sys', 'audioroutervirtual.inf', 'audioroutervirtual.cat')) {
    $entry = @($verification.checks | Where-Object { $_.file -eq $name })
    if ($entry.Count -ne 1 -or (Get-FileHash (Join-Path $Package $name) -Algorithm SHA256).Hash -ne $entry[0].sha256) { throw "Package hash mismatch: $name" }
}
foreach ($name in @('audioroutervirtual.sys', 'audioroutervirtual.cat')) {
    $signature = Get-AuthenticodeSignature -LiteralPath (Join-Path $Package $name)
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $verification.certificateThumbprint) {
        throw 'Trust the exported test CER inside this VM before running stage A.'
    }
}
if ($CreateRootDevice) {
    if (-not $Devcon -or -not (Test-Path -LiteralPath $Devcon -PathType Leaf)) { throw '-CreateRootDevice requires an explicit WDK -Devcon path.' }
    $Devcon = Assert-DriverPackagePath $Devcon
    if ((Get-AuthenticodeSignature -LiteralPath $Devcon).Status -ne 'Valid') { throw 'Devcon must have a valid signature.' }
}
$Evidence = Assert-DriverPackagePath $Evidence
if (Test-Path -LiteralPath $Evidence) { throw 'Evidence directory must be new.' }
New-Item -ItemType Directory -Path $Evidence | Out-Null
Add-Type -Path (Join-Path $PSScriptRoot 'm03-default-endpoints.cs')
$pnputil = Join-Path $env:WINDIR 'System32\pnputil.exe'
$powershellExe = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$manage = Join-Path $driverRoot 'manage.ps1'
$checks = [Collections.Generic.List[object]]::new()
function Record-Check {
    param([string] $Id, [bool] $Passed, [string] $Detail)
    $checks.Add([ordered]@{ id = $Id; passed = $Passed; detail = $Detail })
}
function Get-RootDevices {
    @(Get-PnpDevice -Class MEDIA -PresentOnly | Where-Object {
        $hardwareIds = (Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName 'DEVPKEY_Device_HardwareIds').Data
        'ROOT\AudioRouterVirtual' -in $hardwareIds -or 'SWD\AudioRouterVirtual' -in $hardwareIds
    })
}
function Snapshot {
    param([string] $Name)
    $null = Invoke-DriverTool $pnputil @('/enum-drivers') (Join-Path $Evidence "$Name-driver-store.txt")
    $null = Invoke-DriverTool $pnputil @('/enum-devices', '/class', 'MEDIA') (Join-Path $Evidence "$Name-media.txt")
    $drivers = @(Get-WindowsDriver -Online -All | ForEach-Object { "$($_.Driver)|$($_.ProviderName)|$($_.OriginalFileName)" })
    $devices = @(Get-PnpDevice -Class MEDIA,AudioEndpoint -PresentOnly | Select-Object FriendlyName,InstanceId,Status)
    $devices | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $Evidence "$Name-devices.json") -Encoding UTF8
    $defaults = [AudioRouterVmEvidence.DefaultEndpoints]::Read()
    $snapshot = [pscustomobject]@{ drivers = $drivers; devices = @($devices | ForEach-Object { "$($_.InstanceId)|$($_.Status)|$($_.FriendlyName)" }); defaults = $defaults }
    $snapshot | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Evidence "$Name-baseline.json") -Encoding UTF8
    if ($Cli) { $null = Invoke-DriverTool $Cli @('devices', 'list', '--json') (Join-Path $Evidence "$Name-cli-devices.json") }
    return $snapshot
}
$baseline = $null
$staged = $null
$state = $null
$failure = $null
$installed = $false
try {
    $baseline = Snapshot 'before'
    if (@(Get-RootDevices).Count -ne 0 -or @($baseline.drivers | Where-Object { $_ -match '\|AudioRouter Project\|' }).Count -ne 0) {
        throw 'Baseline already contains AudioRouter driver/device state; restore the clean checkpoint.'
    }
    Record-Check 'A1' $true 'PnP, driver store and six actual default endpoint roles recorded.'
    # manage.ps1 restricts INFs to its own tree. Stage a unique VM-only copy
    # there rather than weakening that wrapper or installing an arbitrary path.
    $staged = Join-Path $driverRoot ('x64\Release\vm-smoke-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $staged | Out-Null
    foreach ($name in @('audioroutervirtual.inf', 'audioroutervirtual.sys', 'audioroutervirtual.cat')) {
        Copy-Item -LiteralPath (Join-Path $Package $name) -Destination $staged
    }
    $inf = Join-Path $staged 'audioroutervirtual.inf'
    $state = Join-Path $staged 'audiorouter-driver-state.json'
    $preview = Invoke-DriverTool $powershellExe @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage, '-Install', '-Preview', '-Inf', $inf, '-State', $state) (Join-Path $Evidence 'install-preview.json')
    if (-not ($preview.Output | ConvertFrom-Json).ready) { throw 'Install preview is not ready.' }
    $null = Invoke-DriverTool $powershellExe @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage, '-Install', '-AllowDriverInstall', '-Inf', $inf, '-State', $state) (Join-Path $Evidence 'install.txt')
    $installed = $true
    if ($CreateRootDevice) { $null = Invoke-DriverTool $Devcon @('install', $inf, 'ROOT\AudioRouterVirtual') (Join-Path $Evidence 'create-root.txt') }
    $roots = @(Get-RootDevices)
    if ($roots.Count -ne 1) { throw 'A2 needs exactly one root device; retry from the checkpoint with -CreateRootDevice and -Devcon.' }
    Record-Check 'A2' $true 'Owned package and one AudioRouter root device present.'
    $expected = if ($EndpointProfile -eq 'Prototype') { @('AudioRouter - Desktop In', 'AudioRouter - Voice Chat') } else {
        @('AudioRouter Cable A Input', 'AudioRouter Cable A Output', 'AudioRouter Cable B Input', 'AudioRouter Cable B Output')
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $endpoints = @(Get-PnpDevice -Class AudioEndpoint -PresentOnly | Where-Object { $_.FriendlyName -like 'AudioRouter*' })
        $matches = @($endpoints | Where-Object { $_.FriendlyName -in $expected -and $_.Status -eq 'OK' })
        if ($matches.Count -eq $expected.Count -and $endpoints.Count -eq $expected.Count) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($matches.Count -ne $expected.Count -or $endpoints.Count -ne $expected.Count) { throw 'Expected healthy endpoints did not appear within 30 seconds.' }
    $installedSnapshot = Snapshot 'installed'
    $allowedIds = @($roots | ForEach-Object { $_.InstanceId }) + @($endpoints | ForEach-Object { $_.InstanceId })
    $unrelated = [pscustomobject]@{
        drivers = @($installedSnapshot.drivers | Where-Object { $_ -notmatch '\|AudioRouter Project\|' })
        devices = @($installedSnapshot.devices | Where-Object { $_.Split('|')[0] -notin $allowedIds })
        defaults = $baseline.defaults
    }
    if (@(Compare-DriverVmBaseline $baseline $unrelated).Count -ne 0) { throw 'Installing the cable changed unrelated driver/device state.' }
    $defaultsChanged = (($baseline.defaults | Sort-Object) -join "`n") -cne (($installedSnapshot.defaults | Sort-Object) -join "`n")
    Record-Check 'A3' $true "Exact endpoints healthy; unrelated state unchanged; Windows default roles changed: $defaultsChanged (recorded, never changed by this script)."
} catch {
    $failure = $_.Exception.Message
    Record-Check 'failure' $false $failure
} finally {
    # Compensate even if root creation or enumeration failed. No pre-existing
    # package/device was allowed at A1; remove only exact matching hardware IDs.
    if ($baseline -and ($installed -or ($state -and (Test-Path -LiteralPath $state)))) {
        if ($KeepInstalled -and -not $failure) { Record-Check 'A14' $false 'Skipped by explicit KeepInstalled; checkpoint restore required.' }
        else {
            try {
                foreach ($root in @(Get-RootDevices)) {
                    $null = Invoke-DriverTool $pnputil @('/remove-device', $root.InstanceId) (Join-Path $Evidence 'remove-device.txt')
                }
                $null = Invoke-DriverTool $powershellExe @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $manage, '-Uninstall', '-AllowDriverInstall', '-Inf', $inf, '-State', $state) (Join-Path $Evidence 'uninstall.txt')
                $deadline = [DateTime]::UtcNow.AddSeconds(30)
                do {
                    $after = Snapshot 'after'
                    $differences = @(Compare-DriverVmBaseline $baseline $after)
                    if ($differences.Count -eq 0) { break }
                    Start-Sleep -Milliseconds 500
                } while ([DateTime]::UtcNow -lt $deadline)
                if ($differences.Count -ne 0) { throw "Baseline mismatch: $($differences -join ', '). Restore checkpoint." }
                Record-Check 'A14' $true 'Driver store, PnP identity/status/names and all default roles match baseline.'
            } catch { $failure = $_.Exception.Message; Record-Check 'A14' $false $failure }
        }
    }
    [ordered]@{ schema = 1; package = $metadata; checks = @($checks.ToArray()); stage = 'A'; installedRetained = [bool]($KeepInstalled -and -not $failure); lifecycleState = $state } |
        ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Evidence 'summary.json') -Encoding UTF8
}
if ($failure) { throw "$failure Evidence: $Evidence" }
Write-Output "VM smoke checks completed. Evidence: $Evidence"
