<#
  Runs INSIDE the AR-DriverTest VM only, as administrator. Repeats the cable
  continuity test in segments while the PC says "run", and records one JSON
  file per segment on the share. Start it once; afterwards soak.cmd on the PC
  starts and pauses it (a saved VM resumes with this window still running).

      powershell -NoProfile -ExecutionPolicy Bypass -File Z:\<bundle>\soak-guest.ps1
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') { throw 'Run this only inside the AR-DriverTest VM.' }
$here = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
$name = Split-Path -Leaf $here
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
if ($here -notlike 'C:\ar\*') {
    # Started from the share: copy the bundle into the guest and continue there.
    $local = Join-Path 'C:\ar' $name
    & robocopy.exe $here $local /E /XF '*.zip' /NFL /NDL /NJH /NJS | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "Copy to $local failed (robocopy $LASTEXITCODE)." }
    & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $local 'soak-guest.ps1')
    exit $LASTEXITCODE
}
$bundle = $here
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'round2-support.ps1')
. (Join-Path $bundle 'soak-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
$share = 'Z:\soak'
if (-not (Test-Path -LiteralPath $share)) { throw 'Z:\soak is missing. Run "soak.cmd start" on the PC first.' }
$mutex = New-Object Threading.Mutex($false, 'Global\AudioRouterSoakGuest')
$owned = $false
try { $owned = $mutex.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned = $true }
if (-not $owned) { throw 'The soak runner is already running in this VM.' }
$work = 'C:\ar\soak'
New-Item -ItemType Directory -Force -Path $work, 'C:\ar\evidence', "$share\records", "$share\evidence" | Out-Null

function Invoke-Script([string] $Script, [string[]] $Arguments) {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $bundle $Script) @Arguments 2>&1 | ForEach-Object { Write-Host "$_" }
        return $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
}
function Get-LoadedDriver {
    $service = Get-CimInstance Win32_SystemDriver -Filter "Name='AudioRouterVirtual'" -ErrorAction SilentlyContinue
    if (-not $service -or $service.State -ne 'Running') { return $null }
    $path = ([string]$service.PathName) -replace '^\\\?\?\\', '' -replace '^\\SystemRoot', $env:SystemRoot
    if (-not [IO.Path]::IsPathRooted($path)) { $path = Join-Path $env:SystemRoot $path }
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    return (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
}
function Get-Dumps {
    return @(Get-ChildItem -Path "$env:SystemRoot\Minidump\*.dmp", "$env:SystemRoot\MEMORY.DMP" -ErrorAction SilentlyContinue |
        ForEach-Object { "$($_.Name) $($_.LastWriteTimeUtc.ToString('o'))" })
}
function Write-Heartbeat([string] $State, [int] $Session, [int] $Segment) {
    try {
        Write-SoakJson "$share\guest.json" @{ state = $State; session = $Session; segment = $Segment; sysSha256 = $script:sys
            driverVersion = $script:version; bundle = $name; boot = $script:boot }
    } catch { }
}
function Read-LoadRows([string] $Path) {
    for ($try = 0; $try -lt 5; $try++) {
        try { return @(Get-HostLoadRows ([IO.File]::ReadAllLines($Path))) } catch { Start-Sleep -Milliseconds 500 }
    }
    return @()
}

# --- one-time setup -----------------------------------------------------------
$expected = (Get-FileHash -LiteralPath (Join-Path $bundle 'driver\audioroutervirtual.sys') -Algorithm SHA256).Hash
$script:version = (Get-Content -LiteralPath (Join-Path $bundle 'driver\package.json') -Raw | ConvertFrom-Json).driverVersion
$script:boot = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('o')
$script:sys = Get-LoadedDriver
if (-not $script:sys) {
    Write-Host 'Installing the test driver in this VM...'
    if ((Invoke-Script 'vm-checks.ps1' @('-Step', 'preflight')) -ne 0) { throw 'VM preflight failed; see above.' }
    [void](Invoke-Script 'vm-checks.ps1' @('-Step', 'install'))
    $script:sys = Get-LoadedDriver
    if (-not $script:sys) { throw 'The driver is not running yet. Restart Windows in the VM, sign in and run this command again.' }
}
if ($script:sys -ine $expected) { throw "Another driver image is loaded ($($script:sys.Substring(0, 12))); expected $($expected.Substring(0, 12)). Remove it first." }
if (-not (Test-Path -LiteralPath 'C:\ar\quiet-guest-state.json')) { [void](Invoke-Script 'quiet-guest.ps1' @('-Apply', '-SettleSeconds', '600')) }

# A crash dump that appears after the first start is a failure of this driver.
$known = "$work\dumps.json"
if (-not (Test-Path -LiteralPath $known)) { Write-SoakJson $known @{ dumps = @(Get-Dumps) } }
function Test-NewDump {
    $saved = Read-SoakJson $known
    $before = if ($saved) { @($saved.dumps) } else { @() }
    $now = @(Get-Dumps)
    $new = @($now | Where-Object { $before -notcontains $_ })
    if ($new.Count -gt 0) {
        Write-SoakJson "$share\records\crash-$([Guid]::NewGuid().ToString('N')).json" @{ sysSha256 = $script:sys; driverVersion = $script:version
            routes = 0; outcome = 'fail'; reason = "Windows crash dump: $($new -join ', ')"; validSeconds = 0; minutes = 0 }
        Write-SoakJson $known @{ dumps = $now }
    }
}
Test-NewDump
# The bundle says how many cables to enable (cables.txt: 8 for 0.2.5, 16 for
# the 16-cable driver); without the file, 8.
$cableCount = 8
$cablesFile = Join-Path $bundle 'cables.txt'
if (Test-Path -LiteralPath $cablesFile) { $cableCount = [int]([IO.File]::ReadAllText($cablesFile).Trim()) }
if ($cableCount -lt 1 -or $cableCount -gt 16) { throw "cables.txt must hold 1-16, not $cableCount." }
function Invoke-SetCables {
    $inventory = & (Join-Path $bundle 'tools\m00-probe.exe') inventory | Out-String
    if ($LASTEXITCODE -ne 0) { throw "The audio inventory failed (exit $LASTEXITCODE)." }
    $active = @([regex]::Matches($inventory, '(?m)^render\[\d+\] name=AudioRouter Cable [A-P] Input \(AudioRouter Virtual Cable\) id=')).Count
    if ($active -eq $cableCount) { return }
    $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
    $process = Start-Process -FilePath (Join-Path $bundle 'tools\audiorouter-driver-helper.exe') -Wait -PassThru -WindowStyle Hidden `
        -ArgumentList "set-cables --count $cableCount --result `"$work\set-cables.json`"" -RedirectStandardOutput "$work\set-cables.txt" -RedirectStandardError "$work\set-cables-error.txt"
    if ($process.ExitCode -eq 5) { throw "Windows needs a restart to enable $cableCount cables. Restart the VM, sign in and run this command again." }
    if ($process.ExitCode -ne 0) { throw "Could not enable $cableCount cables (helper exit $($process.ExitCode)); see $work\set-cables.json." }
    Start-Sleep -Seconds 5
}
Invoke-SetCables

# --- segments -----------------------------------------------------------------
Write-Host 'Ready. Use soak.cmd start / pause / status on the PC. Leave this window open.' -ForegroundColor Green
$segment = 0
while ($true) {
    $control = Read-SoakJson "$share\control.json"
    if (-not $control -or $control.state -ne 'run') {
        $state = if ($control -and $control.state -eq 'stop') { 'stopped' } else { 'waiting' }
        $session = if ($control) { [int]$control.session } else { 0 }
        Write-Heartbeat $state $session $segment
        Start-Sleep -Seconds 3
        continue
    }
    $segment++
    $session = [int]$control.session
    $minutes = [int]$control.minutes
    $routes = [int]$control.routes
    $loadPath = Join-Path $share ([string]$control.loadFile)
    $fromRow = @(Read-LoadRows $loadPath).Count
    $id = '{0:D4}-{1:D4}-{2}' -f $session, $segment, [Guid]::NewGuid().ToString('N').Substring(0, 8)
    $existing = @(Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter 'continuity-*' | ForEach-Object { $_.Name })
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $log = Join-Path $work 'segment.txt'
    Write-Host "$(Get-Date -Format 'HH:mm') segment $segment (session $session): $routes routes, $minutes min"
    $child = Start-Process -FilePath $powershell -WindowStyle Hidden -PassThru -RedirectStandardOutput $log -RedirectStandardError "$log.err" `
        -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$bundle\run-cable-continuity.ps1`" -Minutes $minutes -RouteCount $routes"
    $null = $child.Handle
    $interrupted = $false
    $timedOut = $false
    $beat = -100.0
    while (-not $child.HasExited) {
        Start-Sleep -Seconds 2
        if ($clock.Elapsed.TotalSeconds - $beat -ge 10) { Write-Heartbeat 'running' $session $segment; $beat = $clock.Elapsed.TotalSeconds }
        $now = Read-SoakJson "$share\control.json"
        if ($now -and ($now.state -ne 'run' -or [int]$now.session -ne $session)) { $interrupted = $true }
        $timedOut = $clock.Elapsed.TotalMinutes -gt $minutes + 15
        if ($interrupted -or $timedOut) {
            & taskkill.exe /PID $child.Id /T /F | Out-Null
            Get-Process -Name 'm03_bridge_tone', 'm00-probe' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
            [void]$child.WaitForExit(15000)
            Start-Sleep -Seconds 5
            break
        }
    }
    # The PC may have paused and resumed the VM between two looks. An unreadable
    # control file counts as a change: never credit time that cannot be checked.
    $now = Read-SoakJson "$share\control.json"
    if (-not $now -or $now.state -ne 'run' -or [int]$now.session -ne $session) { $interrupted = $true }
    $dir = Get-ChildItem -LiteralPath 'C:\ar\evidence' -Directory -Filter 'continuity-*' |
        Where-Object { $existing -notcontains $_.Name } | Select-Object -Last 1
    $result = if ($dir -and -not $timedOut) { Read-SoakJson (Join-Path $dir.FullName 'result.json') } else { $null }
    $load = [pscustomobject]@{ Verdict = 'NoData'; Detail = 'the host-load file could not be read' }
    try { $load = Get-HostLoadVerdict (Get-HostLoadWindow -Rows @(Read-LoadRows $loadPath) -From $fromRow) } catch { }
    $grade = Get-SegmentOutcome -Interrupted $interrupted -Result $result -HostVerdict $load.Verdict -Minutes $minutes -Routes $routes
    $pool = $null
    try { $pool = Get-DriverPoolTagSample } catch { }
    $record = [ordered]@{ id = $id; session = $session; segment = $segment; sysSha256 = $script:sys; driverVersion = $script:version
        bundle = $name; boot = $script:boot; guestUtc = [DateTime]::UtcNow.ToString('o')
        routes = $routes; minutes = $minutes; outcome = $grade.Outcome; reason = $grade.Reason
        validSeconds = $grade.ValidSeconds; realTime = $grade.RealTime; hostLoad = $load.Detail
        verdict = $(if ($result) { $result.Verdict }); causes = $(if ($result) { $result.Causes }); pool = $pool
        results = $(if ($result) { @($result.Results | Select-Object * -ExcludeProperty Events) }) }
    if ((Get-LoadedDriver) -ine $script:sys) { $record.outcome = 'review'; $record.reason = 'the driver changed or stopped during the segment'; $record.validSeconds = 0 }
    for ($try = 0; $try -lt 20; $try++) { try { Write-SoakJson "$share\records\$id.json" $record; break } catch { Start-Sleep -Seconds 3 } }
    Write-Host "  $($record.outcome): $($record.reason)"
    Test-NewDump
    # Raw files (about 30 MB) are kept only for the first twenty segments that
    # someone must look at; every segment keeps its JSON record.
    if ($record.outcome -in @('fail', 'review') -and @(Get-ChildItem -LiteralPath "$share\evidence" -Filter '*.zip').Count -lt 20) {
        Copy-Item -LiteralPath $log -Destination "$share\evidence\$id.txt" -Force -ErrorAction SilentlyContinue
        Copy-Item -LiteralPath "$log.err" -Destination "$share\evidence\$id.err.txt" -Force -ErrorAction SilentlyContinue
        if ($dir) { Copy-Item -LiteralPath (Join-Path $bundle "$($dir.Name).zip") -Destination "$share\evidence\$id.zip" -ErrorAction SilentlyContinue }
    }
    if ($dir) {
        foreach ($path in (Join-Path $bundle "$($dir.Name).zip"), "Z:\$name\$($dir.Name).zip") { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue }
        Remove-Item -LiteralPath $dir.FullName -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($record.outcome -eq 'review') { Start-Sleep -Seconds 30 }
}
