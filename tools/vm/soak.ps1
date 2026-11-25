<#
  PC side of the driver soak campaign. It drives only the AR-DriverTest VM and
  the folder C:\VMs\ar-share\soak. It changes no host driver, audio device,
  power setting or VM snapshot.

    soak.cmd start [-Until 15:00]   start or resume; -Until pauses by itself
    soak.cmd pause                  give the PC back (VM frozen at once, then saved, memory freed)
    soak.cmd stop                   end the campaign and shut the VM down
    soak.cmd status                 hours so far
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)][ValidateSet('start', 'pause', 'stop', 'status', 'monitor')][string] $Action = 'status',
    [ValidatePattern('^(([01]\d|2[0-3]):[0-5]\d)?$')][string] $Until = '',
    [ValidateRange(5, 60)][int] $Minutes = 20,
    [ValidateSet(2, 4)][int] $Routes = 2,
    [switch] $AllowWsl,
    # Tests only. Segments on a busy PC are graded invalid anyway.
    [switch] $SkipIdleCheck,
    # Internal: the monitor's grace after a fresh boot, and its freeze limit.
    [switch] $FreshBoot,
    [ValidateRange(5, 3600)][int] $StaleSeconds = 300,
    [string] $Root = 'C:\VMs\ar-share\soak',
    [string] $Bundle = 'rc-0.2.5-soak',
    [string] $Box = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe',
    [string] $VmName = 'AR-DriverTest',
    [int] $TargetHours = 300
)
$ErrorActionPreference = 'Stop'
$scriptPath = $PSCommandPath
. (Join-Path $PSScriptRoot 'soak-support.ps1')
. (Join-Path $PSScriptRoot 'round2-host-preflight.ps1') -LibraryOnly
$controlPath = Join-Path $Root 'control.json'
$monitorPath = Join-Path $Root 'monitor.json'
$eventsPath = Join-Path $Root 'events.jsonl'
$guestPath = Join-Path $Root 'guest.json'
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'

function Invoke-Box([string[]] $Arguments) {
    # VBoxManage prints progress on stderr, which is fatal under "Stop" in 5.1.
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $text = & $Box @Arguments 2>&1 | ForEach-Object { "$_" } | Out-String
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    return [pscustomobject]@{ Code = $code; Text = $text }
}
function Get-VmState {
    $info = Invoke-Box @('showvminfo', $VmName, '--machinereadable')
    $match = [regex]::Match($info.Text, '(?m)^VMState="([^"]+)"')
    if ($info.Code -ne 0 -or -not $match.Success) { throw "VirtualBox does not know $VmName." }
    return $match.Groups[1].Value
}
function Wait-VmState([string[]] $States, [int] $Seconds) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    do {
        if ($States -contains (Get-VmState)) { return $true }
        Start-Sleep -Seconds 1
    } while ($watch.Elapsed.TotalSeconds -lt $Seconds)
    return $false
}
# The driver last heard from, also while no heartbeat exists (a resume, a freeze).
function Get-LastSha {
    $last = Read-SoakJson $monitorPath
    if ($last -and $last.sysSha256) { return [string]$last.sysSha256 }
    $newest = Get-ChildItem -Path (Join-Path $Root 'records\*.json') -ErrorAction SilentlyContinue | Sort-Object LastWriteTime | Select-Object -Last 1
    if ($newest) { return [string](Read-SoakJson $newest.FullName).sysSha256 }
    return ''
}
function Add-SoakEvent([string] $Kind, [string] $Detail) {
    $guest = Read-SoakJson $guestPath
    $line = [ordered]@{ utc = [DateTime]::UtcNow.ToString('o'); kind = $Kind; detail = $Detail
        sysSha256 = $(if ($guest) { $guest.sysSha256 } else { Get-LastSha }) } | ConvertTo-Json -Compress
    [IO.File]::AppendAllText($eventsPath, "$line`r`n", (New-Object Text.UTF8Encoding($false)))
}
function Read-SoakEvents {
    if (-not (Test-Path -LiteralPath $eventsPath)) { return @() }
    return @([IO.File]::ReadAllLines($eventsPath) | Where-Object { $_.Trim() } | ForEach-Object { try { $_ | ConvertFrom-Json } catch { } })
}
function Set-Control([string] $State, $Change = @{}) {
    $old = Read-SoakJson $controlPath
    $new = [ordered]@{ state = $State; session = 0; minutes = $Minutes; routes = $Routes; loadFile = '' }
    if ($old) { foreach ($key in 'session', 'minutes', 'routes', 'loadFile') { $new[$key] = $old.$key } }
    foreach ($key in $Change.Keys) { $new[$key] = $Change[$key] }
    Write-SoakJson $controlPath $new
}
# True only for a process this campaign started (the command line guards against PID reuse).
function Test-Owned($ProcessId, [string] $Marker) {
    if (-not $ProcessId) { return $false }
    $process = Get-CimInstance Win32_Process -Filter "ProcessId=$([int]$ProcessId)" -ErrorAction SilentlyContinue
    return [bool]($process -and $process.Name -eq 'powershell.exe' -and [string]$process.CommandLine -like "*$Marker*")
}
function Stop-Helpers([switch] $KeepMonitor) {
    $info = Read-SoakJson $monitorPath
    if (-not $info) { return }
    if (-not $KeepMonitor -and (Test-Owned $info.monitorPid 'soak.ps1')) { Stop-Process -Id ([int]$info.monitorPid) -Force -ErrorAction SilentlyContinue }
    if (Test-Owned $info.samplerPid 'host-load.ps1') { Stop-Process -Id ([int]$info.samplerPid) -Force -ErrorAction SilentlyContinue }
}
function Invoke-Pause([string] $Why, [switch] $KeepMonitor) {
    # Tell the guest first, so the segment in progress cannot count. Freezing the
    # VM must happen even if that write fails; start changes the session anyway.
    $controlError = $null
    try { if (Test-Path -LiteralPath $controlPath) { Set-Control 'pause' } } catch { $controlError = $_ }
    $state = Get-VmState
    if ($state -eq 'running') {
        [void](Invoke-Box @('controlvm', $VmName, 'pause'))
        $state = Get-VmState
        if ($state -eq 'paused') { Write-Host 'The VM is frozen; your CPU is free. Saving it to free the memory (up to a few minutes)...' }
    }
    Stop-Helpers -KeepMonitor:$KeepMonitor
    if ($state -in @('paused', 'running')) {
        $saved = Invoke-Box @('controlvm', $VmName, 'savestate')
        if (-not (Wait-VmState @('saved') 300)) { throw "The VM could not be saved (state $(Get-VmState)): $($saved.Text)" }
        Write-Host 'Paused. The PC is yours. Run "soak.cmd start" to continue.' -ForegroundColor Green
    } elseif ($state -eq 'saved') { Write-Host 'Already paused.' }
    else { Write-Host "The VM is $state; nothing to pause." }
    if (Test-Path -LiteralPath $Root) { Add-SoakEvent 'pause' $Why }
    if ($controlError) { throw $controlError }
}
function Start-Campaign {
    $vm = Get-VmState
    if ($vm -notin @('poweroff', 'aborted', 'saved', 'aborted-saved', 'paused', 'running')) { throw "The VM is $vm; try again in a moment." }
    $control = Read-SoakJson $controlPath
    $monitor = Read-SoakJson $monitorPath
    if ($vm -eq 'running' -and $control -and $control.state -eq 'run' -and $monitor -and (Test-Owned $monitor.monitorPid 'soak.ps1')) {
        Write-Host 'Already running. To change -Until, pause and start again.'
        return
    }
    $share = Split-Path -Parent $Root
    if (-not (Test-Path -LiteralPath (Join-Path $share "$Bundle\soak-guest.ps1"))) { throw "The bundle $share\$Bundle is missing; run tools\vm\prepare-soak-bundle.ps1 once." }
    if (-not $SkipIdleCheck) {
        Write-Host 'Checking that the PC is idle (10 s)...'
        # The VM needs 8 GB when it is not loaded, and 6 GB must remain for an hour to count.
        $needMb = if ($vm -in @('running', 'paused')) { 6000 } else { 14500 }
        $verdict = Get-HostLoadVerdict (Measure-HostIdle -Seconds 10) -MinAvailableMb $needMb
        $reasons = @()
        if ($verdict.Verdict -ne 'Idle') { $reasons += $verdict.Detail }
        if (@(Get-RunningWsl).Count -gt 0 -and -not $AllowWsl) { $reasons += 'WSL is running: run "wsl --shutdown", or start with -AllowWsl' }
        if ($reasons.Count -gt 0) {
            Write-Host 'NOT started: the PC is too busy for the hours to count.' -ForegroundColor Yellow
            foreach ($reason in $reasons) { Write-Host "  - $reason" }
            $top = @(Get-Process | Sort-Object WorkingSet64 -Descending | Select-Object -First 6 | ForEach-Object { '{0} {1:N1} GB' -f $_.ProcessName, ($_.WorkingSet64 / 1GB) })
            Write-Host "  Largest programs: $($top -join ', '). Free memory needed: $needMb MB."
            exit 2
        }
    }
    New-Item -ItemType Directory -Force -Path $Root, (Join-Path $Root 'records'), (Join-Path $Root 'evidence') | Out-Null
    Stop-Helpers
    $session = 1 + $(if ($control) { [int]$control.session } else { 0 })
    $load = 'host-load-{0:D4}.csv' -f $session
    $samplerPid = Start-Sampler -Path (Join-Path $Root $load) -Minutes 2880
    try {
        # Written while the VM is still stopped or frozen: whatever segment it
        # resumes in sees the new session number and drops itself.
        $heard = Read-SoakJson $guestPath
        $sha = if ($heard) { [string]$heard.sysSha256 } else { Get-LastSha }
        Remove-Item -LiteralPath $guestPath -Force -ErrorAction SilentlyContinue
        Set-Control 'run' @{ session = $session; minutes = $Minutes; routes = $Routes; loadFile = $load }
        $run = [pscustomobject]@{ Code = 0; Text = '' }
        if ($vm -eq 'paused') { $run = Invoke-Box @('controlvm', $VmName, 'resume') }
        elseif ($vm -ne 'running') { $run = Invoke-Box @('startvm', $VmName, '--type', 'gui') }
        if ($run.Code -ne 0 -or -not (Wait-VmState @('running') 180)) { throw "The VM did not start: $($run.Text)" }
        $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" monitor -Root `"$Root`" -Box `"$Box`" -VmName `"$VmName`" -StaleSeconds $StaleSeconds"
        if ($vm -in @('poweroff', 'aborted')) { $arguments += ' -FreshBoot' }
        if ($Until) { $arguments += " -Until $Until" }
        $process = Start-Process -FilePath $powershell -ArgumentList $arguments -WindowStyle Hidden -PassThru
        Write-SoakJson $monitorPath @{ monitorPid = $process.Id; samplerPid = $samplerPid; session = $session; loadFile = $load; sysSha256 = $sha }
    } catch {
        try { Set-Control 'pause' } catch { }
        Stop-Process -Id $samplerPid -Force -ErrorAction SilentlyContinue
        throw
    }
    Add-SoakEvent 'start' "session $session, $Routes routes, $Minutes min segments, until $Until"
    Write-Host "Running (session $session)." -ForegroundColor Green
    if ($Until) { Write-Host "It pauses by itself at $((Get-SoakDeadline $Until (Get-Date)).ToString('ddd HH:mm'))." }
    if ($vm -in @('poweroff', 'aborted')) {
        Write-Host @"
The VM was off, so do this once inside it (later starts need nothing):
  1. Sign in and open PowerShell with "Run as administrator".
  2. powershell -NoProfile -ExecutionPolicy Bypass -File Z:\$Bundle\soak-guest.ps1
  3. Wait for "Ready". Leave that window open.
"@ -ForegroundColor Cyan
    }
}
function Show-Status {
    $control = Read-SoakJson $controlPath
    $guest = Read-SoakJson $guestPath
    Write-Host "VM: $(Get-VmState)   campaign: $(if ($control) { "$($control.state), session $($control.session)" } else { 'not started' })"
    if ($guest) {
        $age = [int]((Get-Date) - (Get-Item -LiteralPath $guestPath).LastWriteTime).TotalSeconds
        Write-Host "VM runner: $($guest.state), segment $($guest.segment), heard $age s ago"
    } else { Write-Host 'VM runner: not heard from in this session' }
    $records = @(Get-ChildItem -Path (Join-Path $Root 'records\*.json') -ErrorAction SilentlyContinue | ForEach-Object { Read-SoakJson $_.FullName })
    $events = @(Read-SoakEvents)
    $scores = @(Get-SoakScore -Records $records -Events $events -TargetHours $TargetHours)
    if ($scores.Count -eq 0) { Write-Host 'No finished segments yet.' }
    foreach ($score in $scores) {
        Write-Host ''
        Write-Host ('Driver {0} ({1}), {2} routes{3}' -f $score.DriverVersion, $score.SysSha256.Substring(0, [Math]::Min(12, $score.SysSha256.Length)), $score.Routes,
            $(if ($score.Rehearsal) { '  [REHEARSAL: does not count for the frozen 0.3.0]' } else { '' }))
        Write-Host ('  clean hours: {0:N2} of {1}' -f $score.Hours, $score.TargetHours)
        Write-Host ('  segments: {0} valid, {1} invalid ({2} %), {3} paused, {4} failed, {5} to review; {6} incidents' -f `
            $score.Valid, $score.Invalid, $score.InvalidPercent, $score.Paused, $score.Failed, $score.Review, $score.Incidents)
        Write-Host "  claim: $($score.Claim)"
        if ($score.Reasons) { Write-Host "  not counted: $($score.Reasons)" }
    }
    foreach ($item in @($events | Where-Object { $_.kind -in @('incident', 'monitor-error') } | Select-Object -Last 3)) {
        Write-Host "  $($item.kind) $($item.utc): $($item.detail)" -ForegroundColor Red
    }
}
function Start-Monitor {
    # Keeps the PC awake only while this process lives; no power setting changes.
    Add-Type -Namespace AudioRouter -Name Awake -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);'
    [void][AudioRouter.Awake]::SetThreadExecutionState([uint32]2147483649)
    $deadline = Get-SoakDeadline $Until (Get-Date)
    $started = Get-Date
    $bad = 0
    $reported = $false
    while ($true) {
        Start-Sleep -Seconds 15
        try {
            $control = Read-SoakJson $controlPath
            if ($control -and $control.state -ne 'run') { return }
            if ($deadline -and (Get-Date) -ge $deadline) { Invoke-Pause "automatic at $Until" -KeepMonitor; return }
            $vm = Get-VmState
            $age = if (Test-Path -LiteralPath $guestPath) { ((Get-Date) - (Get-Item -LiteralPath $guestPath).LastWriteTime).TotalSeconds } else { $null }
            $health = Get-SoakHealth $vm $age ((Get-Date) - $started).TotalSeconds $StaleSeconds ([bool]$FreshBoot)
            # The sampler ends after 48 h; a weekend run needs it to continue.
            $info = Read-SoakJson $monitorPath
            if ($info -and [int]$info.monitorPid -eq $PID -and $info.loadFile -and -not (Test-Owned $info.samplerPid 'host-load.ps1')) {
                $again = Read-SoakJson $controlPath
                if ($again -and $again.state -ne 'run') { return }
                $info.samplerPid = Start-Sampler -Path (Join-Path $Root $info.loadFile) -Minutes 2880
                Write-SoakJson $monitorPath $info
            }
            if ($health -eq 'ok') { $bad = 0; $reported = $false } else { $bad++ }
            # Two looks in a row, so a pause in progress is not an incident.
            if ($bad -ge 2 -and -not $reported) {
                $again = Read-SoakJson $controlPath
                if ($again -and $again.state -ne 'run') { return }
                Add-SoakEvent 'incident' "guest $health while running (VM $vm, last heard $age s ago). Nothing was reset; see VBox.log."
                $reported = $true
            }
        } catch {
            try { Add-SoakEvent 'monitor-error' "$_" } catch { }
        }
    }
}

switch ($Action) {
    'start' { Start-Campaign }
    'pause' { Invoke-Pause 'owner' }
    'status' { Show-Status }
    'monitor' { Start-Monitor }
    'stop' {
        if (Test-Path -LiteralPath $controlPath) { Set-Control 'stop' }
        if ((Get-VmState) -eq 'running') {
            # Let the guest end its processes before Windows shuts down.
            $watch = [Diagnostics.Stopwatch]::StartNew()
            do { Start-Sleep -Seconds 2; $guest = Read-SoakJson $guestPath } while ($guest -and $guest.state -ne 'stopped' -and $watch.Elapsed.TotalSeconds -lt 60)
            Stop-Helpers
            [void](Invoke-Box @('controlvm', $VmName, 'acpipowerbutton'))
            if (-not (Wait-VmState @('poweroff') 180)) {
                [void](Invoke-Box @('controlvm', $VmName, 'savestate'))
                [void](Wait-VmState @('saved') 300)
                Write-Host "The VM did not shut down in time; it is now $(Get-VmState)."
            }
        } else {
            Stop-Helpers
            Write-Host "The VM was left $(Get-VmState)."
        }
        if (Test-Path -LiteralPath $Root) { Add-SoakEvent 'stop' 'owner' }
        Show-Status
    }
}
