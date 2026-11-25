[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$VmId,
    [Parameter(Mandatory = $true)][string]$SnapshotId,
    [Parameter(Mandatory = $true)][string]$Username,
    [Parameter(Mandatory = $true)][string]$PasswordFile,
    [Parameter(Mandatory = $true)][string]$Candidate,
    [Parameter(Mandatory = $true)][string]$CandidateHash,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$Previous,
    [Parameter(Mandatory = $true)][string]$PreviousHash,
    [Parameter(Mandatory = $true)][string]$PreviousVersion,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][switch]$RestoreDisposableSnapshot,
    [ValidateRange(1, 300)][int]$CommandTimeoutSeconds = 30,
    [switch]$InjectGuestControlTimeout,
    [switch]$ProductionRecording,
    [ValidateSet('Any','Present','Absent')][string]$WebViewBaseline = 'Any',
    [switch]$OfflineBootstrapRetry
)
$ErrorActionPreference = 'Stop'
$vbox = Join-Path $env:ProgramFiles 'Oracle\VirtualBox\VBoxManage.exe'
. (Join-Path $PSScriptRoot 'installer-lifecycle-process.ps1')
. (Join-Path $PSScriptRoot 'installer-lifecycle-guards.ps1')
function Invoke-VBox([string]$Name, [string[]]$Arguments, [switch]$AllowNonzero) {
    Invoke-LifecycleProcess $vbox $Name $Arguments $output $CommandTimeoutSeconds -AllowNonzero:$AllowNonzero -OutputFilter '^(UUID|VMState|audio|SnapshotUUID(?:-\d+)*)="|^Value:'
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $output) -or -not (Test-Path -LiteralPath (Split-Path -Parent $output) -PathType Container)) { throw 'Evidence directory must be new and have an existing parent' }
New-Item -ItemType Directory -Path $output | Out-Null
$result = [ordered]@{ status = 'failed'; vmId = $VmId; snapshotId = $SnapshotId; username = $Username; timeoutMarkerReached = $null; steps = [Collections.Generic.List[object]]::new(); error = $null }
$started = $false
try {
    if (-not $RestoreDisposableSnapshot) { throw 'Explicit disposable snapshot restore acknowledgement is required' }
    if ($OfflineBootstrapRetry -and ($WebViewBaseline -ne 'Absent' -or -not $ProductionRecording)) {
        throw 'Offline bootstrap retry requires the absent runtime baseline and installed shell verification'
    }
    foreach ($id in @($VmId, $SnapshotId)) { if ($id -notmatch '^[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$') { throw 'VM and snapshot must be exact UUIDs' } }
    if ($VmId -eq 'c889da39-b7d2-4800-b656-35319aeba2f5') { throw 'The driver test VM is excluded from installer qualification' }
    if ($Username -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,19}$') { throw 'Invalid guest username' }
    foreach ($version in @($CandidateVersion, $PreviousVersion)) { if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid product version' } }
    foreach ($file in @($vbox, $PasswordFile, $Candidate, $Previous)) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw 'A required executable, installer or credential file is missing' } }
    foreach ($pair in @(@($Candidate, $CandidateHash), @($Previous, $PreviousHash))) {
        if ($pair[1] -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash -LiteralPath $pair[0] -Algorithm SHA256).Hash -ne $pair[1]) { throw 'Host installer hash mismatch' }
    }
    $info = @(Invoke-VBox 'preflight-vm' @('showvminfo',$VmId,'--machinereadable'))
    if ($info -notcontains 'VMState="poweroff"' -or $info -notcontains 'audio="none"' -or $info -notcontains ('UUID="' + $VmId + '"')) { throw 'Disposable VM must match UUID, be powered off and have audio disabled' }
    $snapshots = @(Invoke-VBox 'preflight-snapshot' @('snapshot',$VmId,'list','--machinereadable'))
    Assert-LifecycleSnapshot $snapshots $SnapshotId
    $null = Invoke-VBox 'restore' @('snapshot',$VmId,'restore',$SnapshotId)
    $result.steps.Add(@{ name = 'restore'; status = 'passed' })
    $started = $true
    $null = Invoke-VBox 'start' @('startvm',$VmId,'--type','headless')
    $result.steps.Add(@{ name = 'start'; status = 'passed' })
    # An offline baseline cold-boots to the sign-in screen (a saved-memory
    # baseline left guest control stale after restore). Sign the standard
    # user in at the console, with the password read from the credential file
    # and never logged, so guest control finds the profile already loaded.
    $deadline = [DateTime]::UtcNow.AddMinutes(5)
    $ready = $false
    $signInAttempts = 0
    $lastSignIn = [DateTime]::MinValue
    $userPattern = '(?im)^Value:\s*(?:[^,\r\n]+,\s*)*' + [regex]::Escape($Username) + '(?:,|\r?$)'
    while ([DateTime]::UtcNow -lt $deadline) {
        $users = @(Invoke-VBox 'readiness-poll' @('guestproperty','get',$VmId,'/VirtualBox/GuestInfo/OS/LoggedInUsersList') -AllowNonzero)
        if (($users | Out-String) -match $userPattern) { $ready = $true; break }
        if ($signInAttempts -lt 3 -and ([DateTime]::UtcNow - $lastSignIn).TotalSeconds -ge 45) {
            $alive = @(Invoke-LifecycleProcess $vbox 'guest-control-alive' @('guestcontrol',$VmId,'run','--username',$Username,'--passwordfile',(Resolve-Path -LiteralPath $PasswordFile).Path,'--timeout','15000','--exe','C:\Windows\System32\cmd.exe','--wait-stdout','--','/c','echo AR_QUAL_ALIVE') $output $CommandTimeoutSeconds -AllowNonzero -OutputFilter '^AR_QUAL_ALIVE\s*$')
            if (($alive | Out-String) -match 'AR_QUAL_ALIVE') {
                $signInAttempts++
                # Let the sign-in screen finish drawing after guest services start.
                Start-Sleep -Seconds 20
                $lastSignIn = [DateTime]::UtcNow
                # Wake the lock screen, then sign in. Do not press Tab, Enter or Escape first:
                # on the account picker they move focus to the account list, so the password
                # went to whichever account headed it instead of the default user.
                $null = Invoke-VBox 'sign-in-wake' @('controlvm',$VmId,'keyboardputscancode','39','b9')
                Start-Sleep -Seconds 3
                $secret = ([IO.File]::ReadAllText((Resolve-Path -LiteralPath $PasswordFile).Path)).Trim()
                $null = Invoke-VBox 'sign-in-password' @('controlvm',$VmId,'keyboardputstring',$secret)
                $secret = $null
                $null = Invoke-VBox 'sign-in-enter' @('controlvm',$VmId,'keyboardputscancode','1c','9c')
                Start-Sleep -Seconds 10
                # The masked sign-in screen is retained so a failed attempt can be diagnosed.
                $null = Invoke-VBox 'sign-in-screenshot' @('controlvm',$VmId,'screenshotpng',(Join-Path $output ('sign-in-attempt-{0}.png' -f $signInAttempts)))
                $result.steps.Add(@{ name = 'console-sign-in'; attempt = $signInAttempts })
            }
        }
        Start-Sleep -Seconds 3
    }
    if (-not $ready) {
        # Keep the console as evidence: one failed attempt stalled at the VirtualBox boot logo.
        try { $null = Invoke-VBox 'readiness-screenshot' @('controlvm',$VmId,'screenshotpng',(Join-Path $output 'guest-not-ready.png')) } catch { }
        $kind = if ($signInAttempts -eq 0) { 'guest control never became ready (boot stall)' } else { 'console sign-in did not complete' }
        throw "Guest user did not sign in within five minutes: $kind"
    }
    $result.steps.Add(@{ name = 'guest-ready'; status = 'passed' })
    if ($InjectGuestControlTimeout) {
        $guestExe = 'C:\Windows\System32\cmd.exe'
        $marker = '%TEMP%\ar-timeout-' + [guid]::NewGuid().ToString('N') + '.txt'
        $probeBase = @('guestcontrol',$VmId,'run','--username',$Username,'--passwordfile',
            (Resolve-Path -LiteralPath $PasswordFile).Path,'--profile','--timeout','40000','--exe',
            $guestExe,'--wait-stdout','--wait-stderr','--','/c')
        # The first profile session after sign-in is slow; warm it up so the bounded
        # probe measures a hung command, not session creation.
        $null = Invoke-LifecycleProcess $vbox 'guest-warm-up' ($probeBase + 'echo AR_QUAL_WARM') $output $CommandTimeoutSeconds -SuppressOutput
        $probe = $probeBase + ('echo AR_QUAL_TIMEOUT_REACHED>{0} & ping -n 20 127.0.0.1 >nul' -f $marker)
        try {
            $null = Invoke-LifecycleProcess $vbox 'injected-guest-timeout' $probe $output 12 -SuppressOutput
            throw 'Guest-control timeout injection returned unexpectedly'
        } finally {
            try {
                $verify = $probeBase + ('type {0}' -f $marker)
                $markerOutput = @(Invoke-LifecycleProcess $vbox 'verify-timeout-marker' $verify $output 10 -OutputFilter '^AR_QUAL_TIMEOUT_REACHED\s*$')
                $result.timeoutMarkerReached = (($markerOutput | Out-String) -match '(?m)^AR_QUAL_TIMEOUT_REACHED\s*$')
                $result.steps.Add(@{ name = 'timeout-marker'; reached = $result.timeoutMarkerReached })
            } catch {
                $result.timeoutMarkerReached = $false
                $result.steps.Add(@{ name = 'timeout-marker'; reached = $false; error = $_.Exception.Message })
            }
        }
    }
    $runner = Join-Path $PSScriptRoot 'installer-lifecycle.ps1'
    $arguments = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner,'-Vm',$VmId,'-Username',$Username,'-PasswordFile',$PasswordFile,'-Candidate',$Candidate,'-CandidateHash',$CandidateHash,'-CandidateVersion',$CandidateVersion,'-Previous',$Previous,'-PreviousHash',$PreviousHash,'-PreviousVersion',$PreviousVersion,'-OutputDirectory',(Join-Path $output 'lifecycle'))
    if ($ProductionRecording) { $arguments += '-ProductionRecording' }
    $arguments += @('-WebViewBaseline', $WebViewBaseline)
    if ($OfflineBootstrapRetry) { $arguments += '-OfflineBootstrapRetry' }
    # VBoxManage can hang before its guest-side timeout starts. Bound the
    # owned host runner tree as a backstop for the inner command deadlines.
    # Leave the inner runner enough time to reconnect after its own command deadline.
    $runnerTimeout = if ($OfflineBootstrapRetry) { 1500 } else { 900 }
    $null = Invoke-LifecycleProcess (Join-Path $PSHOME 'powershell.exe') 'lifecycle' $arguments $output $runnerTimeout -SuppressOutput -KillTree
    $result.steps.Add(@{ name = 'lifecycle'; status = 'passed' })
    $result.status = 'passed'
} catch {
    $result.error = $_.Exception.Message
    # RQ-02 requires a reached marker, not only a bounded timeout. The injected
    # run fails by design; say which half failed instead of leaving it in steps.
    if ($InjectGuestControlTimeout -and $result.timeoutMarkerReached -eq $false) {
        $result.error = "Guest timeout marker was not verified: $($result.error)"
    }
}
finally {
    try {
      if ($started) {
        $state = @(Invoke-VBox 'shutdown-preflight' @('showvminfo',$VmId,'--machinereadable'))
        if ($state -contains 'VMState="running"') {
            $null = Invoke-VBox 'shutdown-request' @('controlvm',$VmId,'acpipowerbutton')
        }
        $deadline = [DateTime]::UtcNow.AddSeconds(300)
        $stopped = $false
        while ([DateTime]::UtcNow -lt $deadline) {
            $state = @(Invoke-VBox 'shutdown-poll' @('showvminfo',$VmId,'--machinereadable'))
            if ($state -contains 'VMState="poweroff"') { $stopped = $true; break }
            Start-Sleep -Seconds 2
        }
        if ($stopped) { $result.steps.Add(@{ name = 'shutdown'; status = 'passed' }) }
        else { $result.status = 'failed'; $result.steps.Add(@{ name = 'shutdown'; status = 'failed' }); if (-not $result.error) { $result.error = 'Guest did not shut down; left running for inspection' } }
      }
    } catch {
        $result.status = 'failed'
        $result.steps.Add(@{ name = 'shutdown'; status = 'failed'; error = $_.Exception.Message })
        if (-not $result.error) { $result.error = $_.Exception.Message }
    } finally {
        [IO.File]::WriteAllText((Join-Path $output 'result.json'), ($result | ConvertTo-Json -Depth 8))
    }
}
Write-Output "Unattended lifecycle $($result.status): $output"
if ($result.status -ne 'passed') { exit 1 }
