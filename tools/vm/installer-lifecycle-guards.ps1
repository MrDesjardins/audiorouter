function Assert-LifecycleSnapshot([string[]]$Lines, [string]$SnapshotId) {
    if (-not ($Lines -match ('^SnapshotUUID(?:-\d+)*="' + [regex]::Escape($SnapshotId) + '"$'))) {
        throw 'Snapshot UUID does not belong to this VM'
    }
}

function Assert-LifecycleGuestProfile([bool]$Administrator, [bool]$AppExists, [bool]$HiveLoaded) {
    if ($Administrator) { throw 'Guest account must not belong to Administrators' }
    if ($AppExists) { throw 'Guest profile is not fresh: AudioRouter directory already exists' }
    if (-not $HiveLoaded) { throw 'Guest user registry hive is not loaded; sign in interactively before running' }
}

function New-LifecycleStagingCommand([string]$Directory) {
    return "if exist $Directory (exit /b 3) else (mkdir $Directory)"
}

function Get-LifecycleWebViewVersions([object[]]$Entries) {
    @($Entries | Where-Object {
        $_.name -like '*WebView2 Runtime*' -and $_.pv -match '^[1-9]\d*(\.\d+){3}$'
    } | ForEach-Object { $_.pv } | Sort-Object -Unique)
}

function Assert-LifecycleWebViewBaseline([string]$Baseline, [string[]]$Versions) {
    if (($Baseline -eq 'Present' -and $Versions.Count -eq 0) -or
        ($Baseline -eq 'Absent' -and $Versions.Count -gt 0)) {
        throw "Guest did not match the expected WebView2 baseline: $Baseline"
    }
}

function Get-LifecycleOfflineOutcome([int]$ExitCode, [bool]$RuntimePresent, [bool]$RegistrationPresent,
    [bool]$FilesPresent, [bool]$DatabasePresent, [string]$ShellProbe, [bool]$BootstrapRefusalConfirmed) {
    if ($RuntimePresent -or $ShellProbe -eq 'usable') { return 'unexpected-offline-success' }
    if ($RegistrationPresent -or $FilesPresent -or $DatabasePresent) { return 'partial-install' }
    if ($ExitCode -eq 0) { return 'unexpected-success-exit' }
    if ($BootstrapRefusalConfirmed) { return 'confirmed-bootstrap-refusal' }
    return 'unattributed-installer-failure'
}

function Invoke-LifecycleOfflineAttempt([scriptblock]$Disconnect, [scriptblock]$Attempt, [scriptblock]$Reconnect, $Evidence) {
    $phase = 'disconnect'
    try {
        & $Disconnect
        $phase = 'setup'
        $Evidence.output = @(& $Attempt)
    } catch {
        $Evidence.primaryFailure = @{ phase = $phase; message = $_.Exception.Message }
    } finally {
        try { & $Reconnect } catch {
            $Evidence.reconnectFailure = @{ phase = 'reconnect'; message = $_.Exception.Message }
        }
    }
}

function Invoke-LifecycleOnlineRetry($OfflineReport, [scriptblock]$RetryAction) {
    if (-not $OfflineReport.retryEligible) { throw "Offline state cannot be safely retried: $($OfflineReport.status)" }
    & $RetryAction
}
