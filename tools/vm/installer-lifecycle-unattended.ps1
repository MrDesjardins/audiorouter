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
    [Parameter(Mandatory = $true)][switch]$RestoreDisposableSnapshot
)
$ErrorActionPreference = 'Stop'
$vbox = Join-Path $env:ProgramFiles 'Oracle\VirtualBox\VBoxManage.exe'
if (-not $RestoreDisposableSnapshot) { throw 'Explicit disposable snapshot restore acknowledgement is required' }
foreach ($id in @($VmId, $SnapshotId)) { if ($id -notmatch '^[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$') { throw 'VM and snapshot must be exact UUIDs' } }
if ($Username -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,19}$') { throw 'Invalid guest username' }
foreach ($version in @($CandidateVersion, $PreviousVersion)) { if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid product version' } }
foreach ($file in @($vbox, $PasswordFile, $Candidate, $Previous)) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Missing input: $file" } }
foreach ($pair in @(@($Candidate, $CandidateHash), @($Previous, $PreviousHash))) {
    if ($pair[1] -notmatch '^[a-fA-F0-9]{64}$' -or (Get-FileHash -LiteralPath $pair[0] -Algorithm SHA256).Hash -ne $pair[1]) { throw 'Host installer hash mismatch' }
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $output) -or -not (Test-Path -LiteralPath (Split-Path -Parent $output) -PathType Container)) { throw 'Evidence directory must be new and have an existing parent' }
$info = @(& $vbox showvminfo $VmId --machinereadable 2>&1)
if ($LASTEXITCODE -ne 0 -or $info -notcontains 'VMState="poweroff"') { throw 'Disposable VM must exist and be powered off' }
$snapshots = @(& $vbox snapshot $VmId list --machinereadable 2>&1)
if ($LASTEXITCODE -ne 0 -or -not ($snapshots | Where-Object { $_ -match ('^SnapshotUUID(?:-\d+)*="' + [regex]::Escape($SnapshotId) + '"$') })) { throw 'Snapshot UUID does not belong to this VM' }
New-Item -ItemType Directory -Path $output | Out-Null
$result = [ordered]@{ status = 'failed'; vmId = $VmId; snapshotId = $SnapshotId; username = $Username; steps = [Collections.Generic.List[object]]::new(); error = $null }
$started = $false
try {
    & $vbox snapshot $VmId restore $SnapshotId
    if ($LASTEXITCODE -ne 0) { throw 'Snapshot restore failed' }
    $result.steps.Add(@{ name = 'restore'; status = 'passed' })
    & $vbox startvm $VmId --type headless
    if ($LASTEXITCODE -ne 0) { throw 'VM start failed' }
    $started = $true
    $result.steps.Add(@{ name = 'start'; status = 'passed' })
    $deadline = [DateTime]::UtcNow.AddMinutes(3)
    $ready = $false
    while ([DateTime]::UtcNow -lt $deadline) {
        $users = @(& $vbox guestproperty get $VmId /VirtualBox/GuestInfo/OS/LoggedInUsersList 2>&1)
        if ($LASTEXITCODE -eq 0 -and (($users | Out-String) -match ('(?i)(?:^|[, :])' + [regex]::Escape($Username) + '(?:[,\r\n]|$)'))) { $ready = $true; break }
        Start-Sleep -Seconds 2
    }
    if (-not $ready) { throw 'Guest user did not sign in within three minutes' }
    $result.steps.Add(@{ name = 'guest-ready'; status = 'passed' })
    $runner = Join-Path $PSScriptRoot 'installer-lifecycle.ps1'
    $arguments = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner,'-Vm',$VmId,'-Username',$Username,'-PasswordFile',$PasswordFile,'-Candidate',$Candidate,'-CandidateHash',$CandidateHash,'-CandidateVersion',$CandidateVersion,'-Previous',$Previous,'-PreviousHash',$PreviousHash,'-PreviousVersion',$PreviousVersion,'-OutputDirectory',(Join-Path $output 'lifecycle'))
    # VBoxManage can hang before its guest-side timeout starts. Bound the
    # owned host process tree too, and retain both streams on failure.
    $quoted = @($arguments | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' })
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $quoted -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $output 'runner-output.txt') -RedirectStandardError (Join-Path $output 'runner-error.txt')
    $null = $process.Handle
    if (-not $process.WaitForExit(120000)) {
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        throw 'Lifecycle runner exceeded the host timeout; its process tree was stopped'
    }
    $runnerExit = $process.ExitCode
    if ($runnerExit -ne 0) { throw "Lifecycle runner failed ($runnerExit)" }
    $result.steps.Add(@{ name = 'lifecycle'; status = 'passed' })
    $result.status = 'passed'
} catch { $result.error = $_.Exception.Message }
finally {
    if ($started) {
        & $vbox controlvm $VmId acpipowerbutton
        $deadline = [DateTime]::UtcNow.AddSeconds(50)
        $stopped = $false
        while ([DateTime]::UtcNow -lt $deadline) {
            $state = @(& $vbox showvminfo $VmId --machinereadable 2>&1)
            if ($state -contains 'VMState="poweroff"') { $stopped = $true; break }
            Start-Sleep -Seconds 2
        }
        if ($stopped) { $result.steps.Add(@{ name = 'shutdown'; status = 'passed' }) }
        else { $result.status = 'failed'; $result.steps.Add(@{ name = 'shutdown'; status = 'failed' }); if (-not $result.error) { $result.error = 'Guest did not shut down; left running for inspection' } }
    }
    [IO.File]::WriteAllText((Join-Path $output 'result.json'), ($result | ConvertTo-Json -Depth 8))
}
Write-Output "Unattended lifecycle $($result.status): $output"
if ($result.status -ne 'passed') { exit 1 }
