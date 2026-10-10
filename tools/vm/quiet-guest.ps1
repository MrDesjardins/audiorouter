<# Guest-only, reversible: reduce background maintenance in AR-DriverTest
   before long audio diagnostics. Traces showed every VM loss coinciding with
   Windows Update, Defender or background-task bursts that the VM (NEM)
   turns into late audio timers. Not for the host PC; never for release
   qualification. -Apply records previous values first; -Revert restores them. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, ParameterSetName = 'Status')][switch] $Status,
    [Parameter(Mandatory = $true, ParameterSetName = 'Apply')][switch] $Apply,
    [Parameter(Mandatory = $true, ParameterSetName = 'Revert')][switch] $Revert,
    [Parameter(ParameterSetName = 'Apply')][ValidateRange(0, 1800)][int] $SettleSeconds = 900
)
$ErrorActionPreference = 'Stop'
$StatePath = 'C:\ar\quiet-guest-state.json'
$UpdateKey = 'HKLM:\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings'
$UpdateValues = @('PauseUpdatesStartTime', 'PauseUpdatesExpiryTime', 'PauseFeatureUpdatesStartTime',
    'PauseFeatureUpdatesEndTime', 'PauseQualityUpdatesStartTime', 'PauseQualityUpdatesEndTime')
$MaintenanceKey = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Schedule\Maintenance'
$ExclusionPaths = @('C:\ar')
$ExclusionProcesses = @('m03_bridge_tone.exe', 'm03_direct_audio.exe', 'm00-probe.exe', 'audiorouter-driver-helper.exe')

function Get-RegistryValue([string] $Key, [string] $Name) {
    $item = Get-ItemProperty -Path $Key -Name $Name -ErrorAction SilentlyContinue
    if ($null -eq $item) { return $null }
    return $item.$Name
}

function Get-QuietSnapshot {
    $updates = [ordered]@{}
    foreach ($name in $UpdateValues) { $updates[$name] = Get-RegistryValue $UpdateKey $name }
    $preference = Get-MpPreference
    return [pscustomobject]@{
        Updates = [pscustomobject]$updates
        MaintenanceDisabled = Get-RegistryValue $MaintenanceKey 'MaintenanceDisabled'
        ExclusionPath = @($preference.ExclusionPath | Where-Object { $_ })
        ExclusionProcess = @($preference.ExclusionProcess | Where-Object { $_ })
    }
}

function Show-QuietSnapshot($Snapshot) {
    $expiry = $Snapshot.Updates.PauseUpdatesExpiryTime
    Write-Host ("Windows Update paused until: {0}" -f $(if ($expiry) { $expiry } else { 'not paused' }))
    Write-Host ("Automatic Maintenance disabled: {0}" -f ($Snapshot.MaintenanceDisabled -eq 1))
    $paths = @($ExclusionPaths | Where-Object { $Snapshot.ExclusionPath -contains $_ })
    $processes = @($ExclusionProcesses | Where-Object { $Snapshot.ExclusionProcess -contains $_ })
    Write-Host ("Defender exclusions for test files: {0}/{1} paths, {2}/{3} processes" -f $paths.Count, $ExclusionPaths.Count, $processes.Count, $ExclusionProcesses.Count)
}

function Invoke-QuietApply([datetime] $NowUtc) {
    if (Test-Path -LiteralPath $StatePath) { throw "Already applied (state file $StatePath exists). Run -Revert first." }
    $before = Get-QuietSnapshot
    # Record first: a failure part-way must still be revertible.
    $before | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $StatePath -Encoding UTF8
    $start = $NowUtc.ToString('yyyy-MM-ddTHH:mm:ssZ')
    $end = $NowUtc.AddDays(7).ToString('yyyy-MM-ddTHH:mm:ssZ')
    if (-not (Test-Path -LiteralPath $UpdateKey)) { New-Item -Path $UpdateKey -Force | Out-Null }
    foreach ($name in $UpdateValues) {
        $value = if ($name -like '*Start*') { $start } else { $end }
        Set-ItemProperty -Path $UpdateKey -Name $name -Value $value -Type String
    }
    if (-not (Test-Path -LiteralPath $MaintenanceKey)) { New-Item -Path $MaintenanceKey -Force | Out-Null }
    Set-ItemProperty -Path $MaintenanceKey -Name 'MaintenanceDisabled' -Value 1 -Type DWord
    $addPaths = @($ExclusionPaths | Where-Object { $before.ExclusionPath -notcontains $_ })
    $addProcesses = @($ExclusionProcesses | Where-Object { $before.ExclusionProcess -notcontains $_ })
    if ($addPaths.Count) { Add-MpPreference -ExclusionPath $addPaths }
    if ($addProcesses.Count) { Add-MpPreference -ExclusionProcess $addProcesses }
    # Fetch signatures now rather than during a measurement.
    try { Update-MpSignature } catch { Write-Warning "Defender signature update failed: $($_.Exception.Message)" }
}

function Invoke-QuietRevert {
    if (-not (Test-Path -LiteralPath $StatePath)) { throw "Nothing to revert: $StatePath does not exist." }
    $before = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
    foreach ($name in $UpdateValues) {
        $value = $before.Updates.$name
        if ($null -eq $value) { Remove-ItemProperty -Path $UpdateKey -Name $name -ErrorAction SilentlyContinue }
        else { Set-ItemProperty -Path $UpdateKey -Name $name -Value $value -Type String }
    }
    if ($null -eq $before.MaintenanceDisabled) { Remove-ItemProperty -Path $MaintenanceKey -Name 'MaintenanceDisabled' -ErrorAction SilentlyContinue }
    else { Set-ItemProperty -Path $MaintenanceKey -Name 'MaintenanceDisabled' -Value ([int]$before.MaintenanceDisabled) -Type DWord }
    # Remove only exclusions this script added.
    $removePaths = @($ExclusionPaths | Where-Object { @($before.ExclusionPath) -notcontains $_ })
    $removeProcesses = @($ExclusionProcesses | Where-Object { @($before.ExclusionProcess) -notcontains $_ })
    if ($removePaths.Count) { Remove-MpPreference -ExclusionPath $removePaths }
    if ($removeProcesses.Count) { Remove-MpPreference -ExclusionProcess $removeProcesses }
    Remove-Item -LiteralPath $StatePath
}

function Wait-QuietCpu([int] $Seconds) {
    # Wait for 60 consecutive seconds below 10 % total CPU, bounded.
    if ($Seconds -le 0) { return $true }
    $calm = 0
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $Seconds) {
        $sample = (Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 5 -MaxSamples 1).CounterSamples[0].CookedValue
        $calm = if ($sample -lt 10) { $calm + 5 } else { 0 }
        Write-Host ("  CPU {0,5:N1} %  quiet for {1,2} s" -f $sample, $calm)
        if ($calm -ge 60) { return $true }
    }
    return $false
}

if ($MyInvocation.InvocationName -ne '.') {
    . (Join-Path $PSScriptRoot 'repo\tests\acceptance\m03-driver-vm-support.ps1')
    Assert-DriverTestVmIdentity $env:COMPUTERNAME (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run in Administrator PowerShell inside the VM.' }
    if ($Status) { Show-QuietSnapshot (Get-QuietSnapshot) }
    if ($Apply) {
        Invoke-QuietApply ([DateTime]::UtcNow)
        Show-QuietSnapshot (Get-QuietSnapshot)
        Write-Host 'Waiting for background work to settle (up to the -SettleSeconds limit)...'
        if (Wait-QuietCpu $SettleSeconds) { Write-Host 'Guest is quiet. Undo later with -Revert.' -ForegroundColor Green }
        else { Write-Warning 'CPU did not settle within the limit; tests may still be disturbed. Undo later with -Revert.' }
    }
    if ($Revert) {
        Invoke-QuietRevert
        Show-QuietSnapshot (Get-QuietSnapshot)
        Write-Host 'Restored the recorded settings.' -ForegroundColor Green
    }
}
