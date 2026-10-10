# Definitions only. The caller must enforce VM identity before invocation.
function Invoke-DriverVmTraceCommand {
    param([string] $Recorder, [string[]] $Arguments, [string] $Directory, [string] $Name, [int] $TimeoutSeconds = 30)
    $stdout = Join-Path $Directory "$Name-stdout.txt"
    $stderr = Join-Path $Directory "$Name-stderr.txt"
    $result = Invoke-DriverVmProcess -Executable $Recorder -Arguments $Arguments -Stdout $stdout -Stderr $stderr -TimeoutSeconds $TimeoutSeconds
    $result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Directory "$Name-process.json") -Encoding UTF8
    return [pscustomobject]@{ Code = $result.Code; TimedOut = $result.TimedOut; Text = (Get-Content -LiteralPath $stdout -Raw) + (Get-Content -LiteralPath $stderr -Raw) }
}

function Invoke-DriverVmSchedulingRun {
    param([Parameter(Mandatory = $true)][string] $Recorder,
        [Parameter(Mandatory = $true)][string] $Directory,
        [Parameter(Mandatory = $true)][scriptblock] $Run)
    if (-not (Test-Path -LiteralPath $Recorder -PathType Leaf)) { throw 'WPR is unavailable; no traced tone was started.' }
    New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    $profiles = Invoke-DriverVmTraceCommand $Recorder @('-profiles') $Directory 'profiles'
    if ($profiles.Code -ne 0 -or $profiles.Text -notmatch '(?m)^\s*GeneralProfile\s') {
        throw 'WPR GeneralProfile is unavailable; no traced tone was started.'
    }
    $status = Invoke-DriverVmTraceCommand $Recorder @('-status') $Directory 'initial-status'
    # The test guest is en-US. Unknown/localized status is refused too.
    if ($status.Code -ne 0 -or $status.Text -notmatch 'WPR is not recording') {
        throw 'WPR is already recording or its status is unknown. Existing recordings were preserved; tone was not started.'
    }
    $instance = 'AudioRouterTone-' + [Guid]::NewGuid().ToString('N')
    $trace = Join-Path $Directory 'scheduling.etl'
    $temporary = Join-Path $Directory 'wpr-temporary'
    New-Item -ItemType Directory -Path $temporary -Force | Out-Null
    $started = $false
    $saved = $false
    $failure = $null
    $cleanupFailure = $null
    $startUtc = [DateTime]::UtcNow.ToString('o')
    try {
        $start = Invoke-DriverVmTraceCommand $Recorder @('-start', 'GeneralProfile.Light', '-filemode', '-recordtempto', $temporary, '-instancename', $instance) $Directory 'start'
        if ($start.Code -ne 0) { throw 'WPR startup failed; tone was not started. Preserve trace command logs.' }
        $started = $true
        # The callback must not print: console backpressure must never extend
        # this recording. Production runs only a bounded redirected child.
        & $Run
    } catch {
        $failure = $_
    } finally {
        try {
            if ($started) {
                # Capture lost-event diagnostics before stopping. This query
                # is useful evidence but must never prevent stop/save.
                try { $null = Invoke-DriverVmTraceCommand $Recorder @('-status', 'collectors', '-instancename', $instance) $Directory 'final-status' }
                catch { $_ | Out-String | Set-Content -LiteralPath (Join-Path $Directory 'status-error.txt') -Encoding UTF8 }
                $stop = Invoke-DriverVmTraceCommand $Recorder @('-stop', $trace, '-skipPdbGen', '-instancename', $instance) $Directory 'stop' 120
                $saved = $stop.Code -eq 0 -and (Test-Path -LiteralPath $trace -PathType Leaf) -and (Get-Item -LiteralPath $trace).Length -gt 0
                if (-not $saved) { throw 'Scheduling trace save failed; preserve command logs and temporary recordings.' }
            }
        } catch { $cleanupFailure = $_ }
        finally {
            if (-not $saved) {
                # A timed-out/failed start may have partially created a trace.
                # This unique instance name belongs only to this invocation.
                # Never issue a global cancel or touch another recorder.
                try {
                    $cancel = Invoke-DriverVmTraceCommand $Recorder @('-cancel', '-instancename', $instance) $Directory 'cancel'
                    if ($started -and $cancel.Code -ne 0) { throw "Could not stop owned recording $instance; inspect WPR status for that instance." }
                } catch { $cleanupFailure = $_ }
            }
            [pscustomobject]@{
                Instance = $instance; Started = $started; Saved = $saved
                StartUtc = $startUtc; EndUtc = [DateTime]::UtcNow.ToString('o')
                RunFailed = [bool]$failure; CleanupFailed = [bool]$cleanupFailure
            } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Directory 'trace-summary.json') -Encoding UTF8
        }
    }
    if ($cleanupFailure) { Write-Warning $cleanupFailure.ToString() }
    if ($failure) { throw $failure }
    if ($cleanupFailure) { throw $cleanupFailure }
    Write-Host "Scheduling trace saved: $trace"
}
