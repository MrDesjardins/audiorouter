[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Vm,
    [Parameter(Mandatory = $true)][string]$Username,
    [Parameter(Mandatory = $true)][string]$PasswordFile,
    [Parameter(Mandatory = $true)][string]$Candidate,
    [Parameter(Mandatory = $true)][string]$CandidateHash,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$Previous,
    [Parameter(Mandatory = $true)][string]$PreviousHash,
    [Parameter(Mandatory = $true)][string]$PreviousVersion,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$ProductionRecording,
    [ValidateSet('Any','Present','Absent')][string]$WebViewBaseline = 'Any',
    [switch]$OfflineBootstrapRetry
)
$ErrorActionPreference = 'Stop'
$vbox = Join-Path $env:ProgramFiles 'Oracle\VirtualBox\VBoxManage.exe'
. (Join-Path $PSScriptRoot 'installer-lifecycle-process.ps1')
. (Join-Path $PSScriptRoot 'installer-lifecycle-guards.ps1')
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw 'Evidence directory already exists' }
if (-not (Test-Path -LiteralPath (Split-Path -Parent $output) -PathType Container)) { throw 'Evidence parent does not exist' }
if ($Username -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,19}$') { throw 'Invalid guest username' }
foreach ($hash in @($CandidateHash, $PreviousHash)) { if ($hash -notmatch '^[a-fA-F0-9]{64}$') { throw 'Expected SHA-256 hashes' } }
foreach ($version in @($CandidateVersion, $PreviousVersion)) { if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected numeric product version' } }
foreach ($path in @($PasswordFile, $Candidate, $Previous, $vbox)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing input: $path" } }
if ((Get-FileHash $Candidate -Algorithm SHA256).Hash -ne $CandidateHash -or (Get-FileHash $Previous -Algorithm SHA256).Hash -ne $PreviousHash) { throw 'Host installer hash mismatch' }
New-Item -ItemType Directory -Path $output | Out-Null
$resultPath = Join-Path $output 'result.json'
$result = [ordered]@{ status = 'failed'; vm = $Vm; username = $Username; candidateHash = $CandidateHash; previousHash = $PreviousHash; candidateVersion = $CandidateVersion; previousVersion = $PreviousVersion; guest = $null; error = $null }
try {
    if ($OfflineBootstrapRetry -and ($WebViewBaseline -ne 'Absent' -or -not $ProductionRecording)) {
        throw 'Offline bootstrap retry requires the absent runtime baseline and installed shell verification'
    }
    $info = @(Invoke-LifecycleProcess $vbox 'guest-vm-preflight' @('showvminfo',$Vm,'--machinereadable') $output 30 -OutputFilter '^(VMState|audio|name)="')
    if ($info -notcontains 'VMState="running"' -or $info -notcontains 'audio="none"' -or $info -contains 'name="AR-DriverTest"') {
        throw 'Guest must be a running disposable VM with host audio disabled'
    }
    $users = @(Invoke-LifecycleProcess $vbox 'guest-login-preflight' @('guestproperty','get',$Vm,'/VirtualBox/GuestInfo/OS/LoggedInUsersList') $output 30 -OutputFilter '^Value:')
    if (($users | Out-String) -notmatch ('(?im)^Value:\s*(?:[^,\r\n]+,\s*)*' + [regex]::Escape($Username) + '(?:,|\r?$)')) {
        throw 'Guest user must already be signed into the desktop before guest control loads a profile'
    }
    $auth = @('--username', $Username, '--passwordfile', (Resolve-Path -LiteralPath $PasswordFile).Path)
    $argsRun = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '30000', '--exe', 'C:\Windows\System32\cmd.exe', '--wait-stdout', '--', '/c', 'echo %USERPROFILE%')
    $profileOutput = @(Invoke-LifecycleProcess $vbox 'guest-profile' $argsRun $output 40)
    $profile = ($profileOutput | Out-String).Trim()
    if ($profile -notmatch '^C:\\Users\\[A-Za-z0-9_.-]+$') { throw "Unexpected guest profile: $profile" }
    $guestDir = "$profile\ar-release-qualification"
    $mkdir = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '30000', '--exe', 'C:\Windows\System32\cmd.exe', '--wait-stdout', '--', '/c', (New-LifecycleStagingCommand $guestDir))
    $null = Invoke-LifecycleProcess $vbox 'guest-staging' $mkdir $output 40
    $copies = @(@($Candidate, "$guestDir\candidate.exe"), @($Previous, "$guestDir\previous.exe"), @((Join-Path $repo 'tests\fixtures\valid-session.json'), "$guestDir\session.json"), @((Join-Path $PSScriptRoot 'installer-lifecycle-guest.ps1'), "$guestDir\run.ps1"))
    $copies += ,@((Join-Path $PSScriptRoot 'installer-lifecycle-guards.ps1'), "$guestDir\guards.ps1")
    if ($ProductionRecording) {
        $copies += ,@((Join-Path $repo 'tests\fixtures\installer-recorder-session.json'), "$guestDir\recorder-session.json")
        $copies += ,@((Join-Path $PSScriptRoot 'installer-lifecycle-recording.ps1'), "$guestDir\recording.ps1")
    }
    foreach ($copy in $copies) {
        $copyArgs = @('guestcontrol', $Vm, 'copyto') + $auth + @($copy[0], $copy[1])
        $null = Invoke-LifecycleProcess $vbox 'guest-copy' $copyArgs $output 40
    }
    $run = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '400000', '--exe', 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe', '--wait-stdout', '--wait-stderr', '--', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "$guestDir\run.ps1", '-CandidateHash', $CandidateHash, '-PreviousHash', $PreviousHash, '-CandidateVersion', $CandidateVersion, '-PreviousVersion', $PreviousVersion)
    if ($ProductionRecording) { $run += '-ProductionRecording' }
    $run += @('-WebViewBaseline', $WebViewBaseline)
    if ($OfflineBootstrapRetry) {
        $network = @(Invoke-LifecycleProcess $vbox 'bootstrap-adapters' @('showvminfo',$Vm,'--machinereadable') $output 30 -OutputFilter '^(nic\d+|cableconnected1)="')
        if ($network -notcontains 'nic1="nat"' -or $network -notcontains 'cableconnected1="on"' -or
            @($network -match '^nic\d+="(?!none")').Count -ne 1) {
            throw 'Offline bootstrap requires exactly one enabled NAT adapter with its virtual cable connected'
        }
        $offlineRun = $run + '-ExpectOfflineBootstrapFailure'
        $result.offlineTransport = @{ primaryFailure = $null; reconnectFailure = $null }
        Invoke-LifecycleOfflineAttempt {
            $null = Invoke-LifecycleProcess $vbox 'bootstrap-disconnect-guest' @('controlvm',$Vm,'setlinkstate1','off') $output 30
            $link = @(Invoke-LifecycleProcess $vbox 'bootstrap-verify-disconnected' @('showvminfo',$Vm,'--machinereadable') $output 30 -OutputFilter '^cableconnected1=')
            if ($link -notcontains 'cableconnected1="off"') { throw 'Guest NAT link did not disconnect' }
        } {
            Invoke-LifecycleProcess $vbox 'bootstrap-offline-setup' $offlineRun $output 415
        } {
            $null = Invoke-LifecycleProcess $vbox 'bootstrap-reconnect-guest' @('controlvm',$Vm,'setlinkstate1','on') $output 30
            $link = @(Invoke-LifecycleProcess $vbox 'bootstrap-verify-reconnected' @('showvminfo',$Vm,'--machinereadable') $output 30 -OutputFilter '^cableconnected1=')
            if ($link -notcontains 'cableconnected1="on"') { throw 'Guest NAT link did not reconnect' }
        } $result.offlineTransport
        $offlineOutput = $result.offlineTransport.output
        $result.offlineTransport.Remove('output')
        if ($result.offlineTransport.primaryFailure -or $result.offlineTransport.reconnectFailure) {
            $failure = $result.offlineTransport.primaryFailure
            if (-not $failure) { $failure = $result.offlineTransport.reconnectFailure }
            throw $failure.message
        }
        $result.offlineBootstrap = ($offlineOutput -join [Environment]::NewLine) | ConvertFrom-Json
        $run = @(Invoke-LifecycleOnlineRetry $result.offlineBootstrap { $run + '-RetryAfterOfflineBootstrap' })
    }
    $guestExit = $null
    $guestOutput = @(Invoke-LifecycleProcess $vbox 'guest-lifecycle' $run $output 415 -AllowNonzero -ExitStatus ([ref]$guestExit))
    $guestText = $guestOutput | Out-String
    [IO.File]::WriteAllText((Join-Path $output 'guest-output.txt'), $guestText)
    try { $result.guest = $guestText | ConvertFrom-Json } catch { throw "Guest did not produce valid JSON (exit $guestExit)" }
    if ($guestExit -ne 0 -or $result.guest.status -ne 'passed') { throw "Guest lifecycle failed (exit $guestExit)" }
    if ($OfflineBootstrapRetry) {
        $result.onlineRetry = 'passed'
        if ($result.offlineBootstrap.status -ne 'confirmed-bootstrap-refusal') {
            throw "Online retry passed, but offline bootstrap attribution remains unverified: $($result.offlineBootstrap.status)"
        }
    }
    $result.status = 'passed'
} catch {
    $result.error = $_.Exception.Message
} finally {
    [IO.File]::WriteAllText($resultPath, ($result | ConvertTo-Json -Depth 12))
}
Write-Output "Lifecycle $($result.status): $resultPath"
if ($result.status -ne 'passed') { exit 1 }
