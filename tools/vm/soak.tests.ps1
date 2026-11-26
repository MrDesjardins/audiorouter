<#
  Host checks for the soak scripts. Uses a fake VBoxManage and a temporary
  folder: the real VM, the share and the driver are never touched. PS 5.1:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\soak.tests.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'soak-support.ps1')
$script:failures = 0
function Assert-That([string] $Name, [bool] $Condition, [string] $Detail = '') {
    if ($Condition) { Write-Host "PASS $Name" } else { Write-Host "FAIL $Name $Detail" -ForegroundColor Red; $script:failures++ }
}

# 1. Scripts parse, are ASCII, and call only commands that exist.
$functions = @()
$scripts = 'soak.ps1', 'soak-guest.ps1', 'soak-support.ps1', 'soak.tests.ps1', 'prepare-soak-bundle.ps1', 'run-cable-continuity.ps1'
$library = 'round2-support.ps1', 'round2-host-preflight.ps1', 'paired-trace-support.ps1', 'soak-support.ps1', '..\..\tests\acceptance\m03-driver-vm-support.ps1'
$trees = @{}
foreach ($name in $scripts + $library | Select-Object -Unique) {
    $errors = $null
    $trees[$name] = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $name), [ref]$null, [ref]$errors)
    Assert-That "parses: $name" (@($errors).Count -eq 0) (($errors | ForEach-Object { $_.Message }) -join '; ')
    if ($name -in $library) { $functions += $trees[$name].FindAll({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] }, $true) | ForEach-Object { $_.Name } }
}
foreach ($name in $scripts) {
    $bytes = [IO.File]::ReadAllBytes((Join-Path $PSScriptRoot $name))
    Assert-That "ASCII: $name" (@($bytes | Where-Object { $_ -gt 127 }).Count -eq 0)
    $own = @($trees[$name].FindAll({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] }, $true) | ForEach-Object { $_.Name })
    $unknown = @($trees[$name].FindAll({ param($n) $n -is [Management.Automation.Language.CommandAst] }, $true) | ForEach-Object { $_.GetCommandName() } |
        Where-Object { $_ -and $_ -notin ($own + $functions) -and -not (Get-Command $_ -ErrorAction SilentlyContinue) })
    Assert-That "commands exist: $name" ($unknown.Count -eq 0) ($unknown -join ', ')
}

# 2. Grading. Anything that is not proven clean adds no time.
function New-Result([string] $Verdict = 'clean', [double] $Emitted = 120000, [double] $Lost = 0, [double] $Corrupted = 0, [int] $Code = 0, [bool] $Passed = $true, [int] $Rows = 2, [double] $Dropped = 0) {
    $row = [pscustomobject]@{ Pair = 'A-to-B'; ProbeCode = $Code; RouteCode = 0; Emitted = $Emitted; Lost = $Lost; Corrupted = $Corrupted; CaptureDropped = $Dropped; RenderSteps = 0 }
    return [pscustomobject]@{ Passed = $Passed; Error = 'x'; Verdict = $Verdict; Results = @(1..$Rows | ForEach-Object { $row }) }
}
$cases = @(
    @('clean and idle', $false, (New-Result), 'Idle', 'valid', 1200), @('paused clean', $true, (New-Result), 'Idle', 'invalid', 0),
    @('paused but corrupted', $true, (New-Result -Corrupted 1), 'Idle', 'fail', 0), @('no result', $false, $null, 'Idle', 'review', 0),
    @('paused, no result', $true, $null, 'NoData', 'invalid', 0), @('clean, host unknown', $false, (New-Result), 'NoData', 'invalid', 0),
    @('clean, host busy', $false, (New-Result), 'Starved', 'invalid', 0), @('corrupted, host busy', $false, (New-Result -Corrupted 2), 'Starved', 'fail', 0),
    @('unexplained, idle', $false, (New-Result 'unexplained-events' -Lost 3), 'Idle', 'fail', 0), @('unexplained, host unknown', $false, (New-Result 'unexplained-events' -Lost 3), 'NoData', 'fail', 0),
    @('unexplained, host busy', $false, (New-Result 'unexplained-events' -Lost 3), 'Starved', 'invalid', 0), @('vm-wide', $false, (New-Result 'all-events-vm-wide' -Lost 3), 'Idle', 'invalid', 0),
    @('correlated', $false, (New-Result 'vm-correlated' -Lost 3), 'Idle', 'invalid', 0), @('probe error', $false, (New-Result -Code 3), 'Idle', 'fail', 0),
    @('harness stopped', $false, (New-Result -Passed $false), 'Idle', 'review', 0), @('a route missing', $false, (New-Result -Rows 1), 'Idle', 'review', 0),
    @('clean but a capture gap', $false, (New-Result -Dropped 480), 'Idle', 'review', 0),
    @('clean but lost', $false, (New-Result -Lost 1), 'Idle', 'fail', 0), @('70 percent of real time', $false, (New-Result -Emitted 84000), 'Idle', 'invalid', 0),
    @('96 percent counts delivered time', $false, (New-Result -Emitted 115200), 'Idle', 'valid', 1152), @('unknown verdict', $false, (New-Result 'other'), 'Idle', 'fail', 0))
foreach ($case in $cases) {
    $grade = Get-SegmentOutcome -Interrupted $case[1] -Result $case[2] -HostVerdict $case[3] -Minutes 20 -Routes 2
    Assert-That "grade: $($case[0])" ($grade.Outcome -eq $case[4] -and $grade.ValidSeconds -eq $case[5]) "$($grade.Outcome) $($grade.ValidSeconds) $($grade.Reason)"
}

# 3. Score.
function New-Record($Outcome, $Reason, $Sha = 'AAA', $Version = '0.3.0.0', $Seconds = 0, $Routes = 2) {
    return [pscustomobject]@{ sysSha256 = $Sha; driverVersion = $Version; routes = $Routes; outcome = $Outcome; reason = $Reason; validSeconds = $Seconds }
}
$clean = @(1..900 | ForEach-Object { New-Record 'valid' 'clean' -Seconds 1200 })
$score = @(Get-SoakScore -Records $clean)
Assert-That 'score: 300 h gives under 1 per 100 h' ($score.Count -eq 1 -and $score[0].Hours -eq 300 -and $score[0].Claim -like 'fewer than 1.00*' -and -not $score[0].Rehearsal) $score[0].Claim
$mixed = @($clean) + (New-Record 'valid' 'clean' 'BBB' '0.2.5.0' 1200) + (New-Record 'valid' 'clean' -Seconds 1200 -Routes 4)
$score = @(Get-SoakScore -Records $mixed)
Assert-That 'score: drivers and loads never add up' ($score.Count -eq 3 -and @($score | Where-Object { $_.Hours -eq 300 }).Count -eq 1)
Assert-That 'score: 0.2.5 is a rehearsal' (@($score | Where-Object { $_.DriverVersion -eq '0.2.5.0' })[0].Rehearsal)
$score = @(Get-SoakScore -Records (@($clean) + (New-Record 'fail' 'corrupted audio')))
Assert-That 'score: one failure removes the claim' ($score[0].Failed -eq 1 -and $score[0].Claim -like 'none*')
$score = @(Get-SoakScore -Records (@($clean) + (New-Record 'fail' 'Windows crash dump: x' -Routes 0)))
Assert-That 'score: a crash dump fails its driver' ($score.Count -eq 1 -and $score[0].Failed -eq 1 -and $score[0].Claim -like 'none*')
$score = @(Get-SoakScore -Records @(New-Record 'fail' 'Windows crash dump: x' -Routes 0))
Assert-That 'score: a crash before any segment is shown' ($score.Count -eq 1 -and $score[0].Failed -eq 1)
$score = @(Get-SoakScore -Records $clean -Events @([pscustomobject]@{ kind = 'incident'; sysSha256 = 'AAA' }))
Assert-That 'score: a freeze removes the claim' ($score[0].Incidents -eq 1 -and $score[0].Claim -like 'none*')
$ten = @(1..7 | ForEach-Object { New-Record 'valid' 'clean' -Seconds 1200 }) + @(1..3 | ForEach-Object { New-Record 'invalid' 'PC busy' })
Assert-That 'score: 30 percent invalid is inconclusive' (@(Get-SoakScore -Records $ten)[0].Claim -like 'inconclusive*')
$pauses = @(1..9 | ForEach-Object { New-Record 'valid' 'clean' -Seconds 1200 }) + @(1..9 | ForEach-Object { New-Record 'invalid' 'paused' })
$score = @(Get-SoakScore -Records $pauses)
Assert-That 'score: pausing often is not held against the run' ($score[0].Paused -eq 9 -and $score[0].InvalidPercent -eq 0 -and $score[0].Hours -eq 3)
Assert-That 'score: nothing recorded' (@(Get-SoakScore -Records @()).Count -eq 0)

# 4. Clock and health rules.
$night = [DateTime]'2026-11-25T22:30:00'
Assert-That 'until: 15:00 from 22:30 is tomorrow' ((Get-SoakDeadline '15:00' $night) -eq [DateTime]'2026-11-26T15:00:00')
Assert-That 'until: 23:00 from 22:30 is today' ((Get-SoakDeadline '23:00' $night) -eq [DateTime]'2026-11-25T23:00:00')
Assert-That 'until: empty is none' ($null -eq (Get-SoakDeadline '' $night))
Assert-That 'health' ((Get-SoakHealth 'running' 10 900) -eq 'ok' -and (Get-SoakHealth 'running' 900 900) -eq 'stale' -and (Get-SoakHealth 'running' 900 60) -eq 'ok' -and (Get-SoakHealth 'aborted' 1 900) -eq 'down' -and
    (Get-SoakHealth 'running' $null 900) -eq 'stale' -and (Get-SoakHealth 'running' $null 900 300 $true) -eq 'ok')

# 5. The real soak.ps1, through a fake VirtualBox.
$temp = Join-Path ([IO.Path]::GetTempPath()) ('soak-test-' + [Guid]::NewGuid().ToString('N'))
$root = Join-Path $temp 'soak'
New-Item -ItemType Directory -Path (Join-Path $temp 'rc-0.2.5-soak') -Force | Out-Null
Set-Content -LiteralPath (Join-Path $temp 'rc-0.2.5-soak\soak-guest.ps1') -Value '#' -Encoding ASCII
$state = Join-Path $temp 'state.txt'
$calls = Join-Path $temp 'calls.txt'
$box = Join-Path $temp 'box.cmd'
@"
@echo off
if not "%1"=="showvminfo" echo %1 %3>>"$calls"
if "%1"=="showvminfo" for /f %%s in ($state) do echo VMState="%%s"
if "%1"=="startvm" echo running>"$state"
if "%3"=="pause" echo paused>"$state"
if "%3"=="resume" echo running>"$state"
if "%3"=="savestate" echo saved>"$state"
if "%3"=="acpipowerbutton" echo poweroff>"$state"
exit /b 0
"@ | Set-Content -LiteralPath $box -Encoding ASCII
Set-Content -LiteralPath $state -Value 'poweroff' -Encoding ASCII
function Invoke-Soak([string[]] $Arguments) {
    # A file, not a pipe: the hidden monitor would inherit and hold a pipe open.
    $out = Join-Path $temp 'out.txt'
    $process = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -PassThru -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError "$out.err" `
        -ArgumentList ("-NoProfile -ExecutionPolicy Bypass -File `"$PSScriptRoot\soak.ps1`" $($Arguments -join ' ') -Root `"$root`" -Box `"$box`"")
    $null = $process.Handle
    if (-not $process.WaitForExit(90000)) { $process.Kill(); throw "soak.ps1 $Arguments did not end." }
    return [pscustomobject]@{ Code = $process.ExitCode; Text = [IO.File]::ReadAllText($out) + [IO.File]::ReadAllText("$out.err") }
}
function Test-Running($ProcessId) { return [bool](Get-Process -Id ([int]$ProcessId) -ErrorAction SilentlyContinue) }
try {
    $run = Invoke-Soak @('start', '-Until', '99:00')
    Assert-That 'cli: a wrong time is refused' ($run.Code -ne 0 -and -not (Test-Path -LiteralPath $calls))
    $run = Invoke-Soak @('pause')
    Assert-That 'cli: pause before the first start is harmless' ($run.Code -eq 0 -and $run.Text -like '*nothing to pause*') $run.Text
    $run = Invoke-Soak @('start', '-SkipIdleCheck', '-Until', (Get-Date).AddHours(2).ToString('HH:mm'))
    $control = Read-SoakJson "$root\control.json"
    $helpers = Read-SoakJson "$root\monitor.json"
    Assert-That 'cli: start' ($run.Code -eq 0 -and $control.state -eq 'run' -and $control.session -eq 1 -and $control.routes -eq 2 -and $run.Text -like '*soak-guest.ps1*') $run.Text
    Assert-That 'cli: monitor and sampler run' ((Test-Running $helpers.monitorPid) -and (Test-Running $helpers.samplerPid))
    $run = Invoke-Soak @('start', '-SkipIdleCheck')
    Assert-That 'cli: a second start changes nothing' ($run.Text -like '*Already running*' -and (Read-SoakJson "$root\control.json").session -eq 1) $run.Text
    $run = Invoke-Soak @('pause')
    Start-Sleep -Seconds 1
    Assert-That 'cli: pause freezes, then saves' ($run.Code -eq 0 -and (Get-Content $state).Trim() -eq 'saved' -and (Read-SoakJson "$root\control.json").state -eq 'pause' -and
        ((Get-Content $calls) -join '|') -like '*controlvm pause|controlvm savestate') "$($run.Text) $((Get-Content $calls) -join '|')"
    Assert-That 'cli: pause ends the helpers' (-not (Test-Running $helpers.monitorPid) -and -not (Test-Running $helpers.samplerPid))
    $run = Invoke-Soak @('pause')
    Assert-That 'cli: pausing twice is harmless' ($run.Code -eq 0 -and $run.Text -like '*Already paused*') $run.Text
    $run = Invoke-Soak @('start', '-SkipIdleCheck', '-Routes', '4')
    $control = Read-SoakJson "$root\control.json"
    Assert-That 'cli: resume opens a new session without a sign-in note' ($control.session -eq 2 -and $control.routes -eq 4 -and $control.loadFile -eq 'host-load-0002.csv' -and $run.Text -notlike '*Sign in*') $run.Text
    Write-SoakJson "$root\records\a.json" (New-Record 'valid' 'clean' -Seconds 1200)
    $run = Invoke-Soak @('status')
    Assert-That 'cli: status shows hours' ($run.Code -eq 0 -and $run.Text -match 'clean hours: 0[.,]33 of 300') $run.Text
    # The sampler must come back by itself (weekend runs outlive its 48 h).
    $helpers = Read-SoakJson "$root\monitor.json"
    Stop-Process -Id ([int]$helpers.samplerPid) -Force
    $watch = [Diagnostics.Stopwatch]::StartNew()
    do { Start-Sleep -Seconds 2; $helpers2 = Read-SoakJson "$root\monitor.json" } while ($helpers2.samplerPid -eq $helpers.samplerPid -and $watch.Elapsed.TotalSeconds -lt 60)
    Assert-That 'cli: the sampler is restarted' ($helpers2.samplerPid -ne $helpers.samplerPid -and (Test-Running $helpers2.samplerPid))
    # A guest that freezes while resuming: no heartbeat ever arrives.
    [void](Invoke-Soak @('pause'))
    $run = Invoke-Soak @('start', '-SkipIdleCheck', '-StaleSeconds', '5')
    $watch = [Diagnostics.Stopwatch]::StartNew()
    do { Start-Sleep -Seconds 2; $log = [IO.File]::ReadAllText("$root\events.jsonl") } while ($log -notlike '*incident*' -and $watch.Elapsed.TotalSeconds -lt 90)
    Assert-That 'cli: a silent resume is an incident of the last driver' ($log -match '"kind":"incident".*"sysSha256":"AAA"') $log
    $run = Invoke-Soak @('status')
    Assert-That 'cli: the incident removes the claim' ($run.Text -match '1 incidents' -and $run.Text -like '*claim: none*') $run.Text
    $helpers = Read-SoakJson "$root\monitor.json"
    $run = Invoke-Soak @('stop')
    Start-Sleep -Seconds 1
    Assert-That 'cli: stop shuts down' ($run.Code -eq 0 -and (Get-Content $state).Trim() -eq 'poweroff' -and (Read-SoakJson "$root\control.json").state -eq 'stop' -and -not (Test-Running $helpers.monitorPid)) $run.Text

    # Automatic pause: a monitor whose time arrives within the next two minutes.
    Set-Content -LiteralPath $state -Value 'saved' -Encoding ASCII
    $run = Invoke-Soak @('start', '-SkipIdleCheck', '-Until', (Get-Date).AddMinutes(1).ToString('HH:mm'))
    $helpers = Read-SoakJson "$root\monitor.json"
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ((Get-Content $state).Trim() -ne 'saved' -and $watch.Elapsed.TotalSeconds -lt 120) { Start-Sleep -Seconds 2 }
    Start-Sleep -Seconds 8
    Assert-That 'cli: -Until pauses by itself and ends the helpers' ((Get-Content $state).Trim() -eq 'saved' -and (Read-SoakJson "$root\control.json").state -eq 'pause' -and
        -not (Test-Running $helpers.monitorPid) -and -not (Test-Running $helpers.samplerPid))
} finally {
    $helpers = Read-SoakJson "$root\monitor.json"
    if ($helpers) { foreach ($id in $helpers.monitorPid, $helpers.samplerPid) { Stop-Process -Id ([int]$id) -Force -ErrorAction SilentlyContinue } }
    Start-Sleep -Seconds 1
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
if ($script:failures -gt 0) { Write-Host "$($script:failures) check(s) failed." -ForegroundColor Red; exit 1 }
Write-Host 'All soak checks passed.' -ForegroundColor Green
