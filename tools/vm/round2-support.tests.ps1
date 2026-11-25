<#
  Host checks for round2-support.ps1 and the round-2 scripts. Read-only: parses
  scripts, grades synthetic tone reports, and reads the host's own pool-tag
  table (no driver is touched). Run with Windows PowerShell 5.1:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\round2-support.tests.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'round2-support.ps1')
$script:failures = 0
function Assert-That([string] $Name, [bool] $Condition, [string] $Detail = '') {
    if ($Condition) { Write-Host "PASS $Name" } else { Write-Host "FAIL $Name $Detail" -ForegroundColor Red; $script:failures++ }
}

# 1. Every script parses and is ASCII-only (PowerShell 5.1 reads BOM-less files as Windows-1252).
foreach ($name in 'round2-support.ps1', 'round2-support.tests.ps1', 'round2-restart-render.ps1', 'round2-pause-watcher.ps1',
    'prepare-round2-bundle.ps1', 'run-stage-a.ps1', 'vm-checks.ps1', 'host-load.ps1', 'round2-host-preflight.ps1') {
    $path = Join-Path $PSScriptRoot $name
    $errors = $null
    [void][Management.Automation.Language.Parser]::ParseFile($path, [ref]$null, [ref]$errors)
    Assert-That "parses: $name" (@($errors).Count -eq 0) (($errors | ForEach-Object { $_.Message }) -join '; ')
    $bytes = [IO.File]::ReadAllBytes($path)
    # vm-checks.ps1 predates this rule and has section signs in comments; its edits are checked below.
    if ($name -ne 'vm-checks.ps1') { Assert-That "ASCII only: $name" (@($bytes | Where-Object { $_ -gt 127 }).Count -eq 0) }
}

# 1b. Every command the guest runner calls is a defined function or a real cmdlet/native name (no typos).
function Get-ScriptAst([string] $FileName) {
    return [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $FileName), [ref]$null, [ref]$null)
}
$functionFilter = { param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] }
$commandFilter = { param($n) $n -is [Management.Automation.Language.CommandAst] }
$supportFunctions = @('round2-support.ps1', 'paired-trace-support.ps1' | ForEach-Object { (Get-ScriptAst $_).FindAll($functionFilter, $true) } | ForEach-Object { $_.Name })
foreach ($name in 'run-stage-a.ps1', 'round2-support.ps1', 'round2-restart-render.ps1', 'round2-pause-watcher.ps1', 'prepare-round2-bundle.ps1', 'host-load.ps1', 'round2-host-preflight.ps1') {
    $ast = Get-ScriptAst $name
    $defined = @($ast.FindAll($functionFilter, $true) | ForEach-Object { $_.Name }) + $supportFunctions
    $unknown = @()
    foreach ($call in $ast.FindAll($commandFilter, $true)) {
        $command = $call.GetCommandName()
        if (-not $command -or $command -in $defined) { continue }
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { $unknown += $command }
    }
    Assert-That "no undefined commands: $name" (@($unknown).Count -eq 0) (($unknown | Sort-Object -Unique) -join ', ')
}

# 2. Tone report parsing, with the exact line shapes the tool prints.
function New-ToneText([long] $RenderUnderrun, [long] $Late, [long] $GapUs = 9000, [bool] $Writes = $true) {
    $lines = @(
        'capture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 0, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 0, last_qpc_time: 0 }',
        "render-source counters (cable-a): NativeBridgeStreamCounters { underrun_frames: $RenderUnderrun, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 0, last_qpc_time: 0 }")
    if ($Writes) { $lines += "render-source packet writes (cable-a): NativeBridgePacketCounters { accepted: 2990, late: $Late, overrun: 0 }" }
    $lines += 'maximum control heartbeat: 386 us; progress snapshot: 122 us; lease close: 42 us'
    $lines += "leases deactivated at 30000 ms; maximum control loop gap: $GapUs us"
    $lines += "maximum pump gaps: capture=$GapUs us render=$GapUs us; WAV append=0 us finish=526 us"
    return ($lines -join "`r`n")
}
$parsed = Get-ToneAbCounters (New-ToneText 6167 4 18900)
Assert-That 'parse: complete report' ($parsed.Complete -and $parsed.HasPacketWrites)
Assert-That 'parse: render underrun' ($parsed.RenderUnderrun -eq 6167)
Assert-That 'parse: late packets' ($parsed.Late -eq 4 -and $parsed.Accepted -eq 2990)
Assert-That 'parse: max gap in ms' ($parsed.MaxGapMs -eq 18.9) "$($parsed.MaxGapMs)"
$old = Get-ToneAbCounters (New-ToneText 0 0 9000 $false)
Assert-That 'parse: a driver without packet counters is complete but has none' ($old.Complete -and -not $old.HasPacketWrites)
$broken = Get-ToneAbCounters "error: Windows(Error { code: HRESULT(0x80070057) })"
Assert-That 'parse: an error report is incomplete' (-not $broken.Complete)

# 2b. The driver version the tone tool prints is parsed, so a run against another driver is never counted.
$withVersion = Get-ToneAbCounters ((New-ToneText 0 0) + "`r`ndriver: NativeBridgeDriverInfo { protocol_major: 1, protocol_minor: 1, driver_version: [0, 2, 5, 0], flags: 0 }")
Assert-That 'parse: driver version' ($withVersion.DriverVersion -eq '0.2.5.0') "$($withVersion.DriverVersion)"
Assert-That 'parse: no driver line leaves the version empty' ($null -eq $parsed.DriverVersion)

# 3. The A/B rule against synthetic runs (medians per render shape).
function New-Rows([string] $Driver, [long[]] $Underruns, [long[]] $Lates, [double] $GapMs = 10, [string] $Variant = 'steady') {
    $rows = @()
    for ($i = 0; $i -lt $Underruns.Count; $i++) {
        $rows += [pscustomobject]@{ Driver = $Driver; Variant = $Variant; Valid = $true; HasPacketWrites = $true
            RenderUnderrun = $Underruns[$i]; Late = $Lates[$i]; PacketOverrun = 0; MaxGapMs = $GapMs }
    }
    return $rows
}
# Both shapes, four runs per driver unless told otherwise.
function New-Both([string] $Driver, [long[]] $Underruns, [long[]] $Lates, [double] $GapMs = 10) {
    return @(New-Rows $Driver $Underruns $Lates $GapMs 'steady') + @(New-Rows $Driver $Underruns $Lates $GapMs 'restarts')
}
$z = @(0, 0, 0, 0)
$same = Compare-RenderAb ((New-Both 'rc3' $z $z) + (New-Both 'new' $z $z))
Assert-That 'A/B: identical zero counters pass' ($same.Verdict -eq 'PASS') $same.Detail
$bad = Compare-RenderAb ((New-Both 'rc3' $z $z) + (New-Both 'new' @(6167, 5800, 6100, 6000) $z))
Assert-That 'A/B: the 2026-11-11 pattern (6k underrun frames) on 0.2.5 only fails' ($bad.Verdict -eq 'FAIL') $bad.Detail
$oneShape = Compare-RenderAb ((New-Both 'rc3' $z $z) + (New-Rows 'new' $z $z 10 'steady') + (New-Rows 'new' @(6167, 5800, 6100, 6000) $z 10 'restarts'))
Assert-That 'A/B: a regression in one shape alone fails' ($oneShape.Verdict -eq 'FAIL') $oneShape.Detail
$late = Compare-RenderAb ((New-Both 'rc3' $z @(0, 0, 1, 0)) + (New-Both 'new' $z @(3, 4, 2, 5)))
Assert-That 'A/B: more late packets on 0.2.5 fails' ($late.Verdict -eq 'FAIL') $late.Detail
$both = Compare-RenderAb ((New-Both 'rc3' @(6000, 5900, 6100, 6000) $z) + (New-Both 'new' @(6167, 5800, 6100, 6000) $z))
Assert-That 'A/B: both drivers underrun alike is not a 0.2.5 regression' ($both.Verdict -eq 'PASS') $both.Detail
$small = Compare-RenderAb ((New-Both 'rc3' $z $z) + (New-Both 'new' @(300, 300, 300, 300) $z))
Assert-That 'A/B: a median difference under one packet does not fail' ($small.Verdict -eq 'PASS') $small.Detail
# A stalled minority cannot move the median: one VM-stall run in four on 0.2.5.
$minority = Compare-RenderAb ((New-Both 'rc3' $z $z) + (New-Both 'new' @(0, 0, 0, 9000) $z))
Assert-That 'A/B: one stalled run in four does not move the median' ($minority.Verdict -eq 'PASS') $minority.Detail
# A stall in most runs of a driver cannot yield a verdict.
$stalled = Compare-RenderAb ((New-Both 'rc3' $z $z 600) + (New-Both 'new' @(9000, 9000, 9000, 9000) $z 600))
Assert-That 'A/B: stall-dominated runs (median gap above 500 ms) are inconclusive' ($stalled.Verdict -eq 'REVIEW') $stalled.Detail
$typical = Compare-RenderAb ((New-Both 'rc3' $z $z 154) + (New-Both 'new' $z $z 248))
Assert-That 'A/B: typical VM gaps of 150-250 ms are still judged' ($typical.Verdict -eq 'PASS') $typical.Detail
$thin = Compare-RenderAb ((New-Both 'rc3' @(0, 0, 0) @(0, 0, 0)) + (New-Both 'new' $z $z))
Assert-That 'A/B: fewer than 4 valid runs on a driver is inconclusive' ($thin.Verdict -eq 'REVIEW') $thin.Detail
$onlySteady = Compare-RenderAb ((New-Rows 'rc3' $z $z) + (New-Rows 'new' $z $z))
Assert-That 'A/B: a missing render shape can never pass' ($onlySteady.Verdict -eq 'REVIEW') $onlySteady.Detail
$invalid = New-Both 'new' $z $z; foreach ($row in $invalid) { $row.Valid = $false }
$none = Compare-RenderAb ((New-Both 'rc3' $z $z) + $invalid)
Assert-That 'A/B: invalid runs (tool failed or wrong driver version) are not counted' ($none.Verdict -eq 'REVIEW') $none.Detail
$noWrites = New-Both 'rc3' $z $z; foreach ($row in $noWrites) { $row.HasPacketWrites = $false }
$lateSkipped = Compare-RenderAb ($noWrites + (New-Both 'new' $z @(9, 9, 9, 9)))
Assert-That 'A/B: late packets are not compared when one driver has no counters' ($lateSkipped.Verdict -eq 'PASS' -and $lateSkipped.Detail -match 'not compared') $lateSkipped.Detail

# 3b. Guest auto-logon is reversible (tested on a fake key, never the real Winlogon key).
$fakeKey = 'HKCU:\Software\AudioRouterRound2Test'
Remove-Item -LiteralPath $fakeKey -Recurse -Force -ErrorAction SilentlyContinue
New-Item -Path $fakeKey -Force | Out-Null
Set-ItemProperty -LiteralPath $fakeKey -Name DefaultUserName -Value 'someone' -Type String
$secret = ConvertTo-SecureString 'not-a-real-password' -AsPlainText -Force
$before = Set-GuestAutoLogon -UserName 'tester' -Domain 'BOX' -Password $secret -KeyPath $fakeKey
$on = Get-ItemProperty -LiteralPath $fakeKey
Assert-That 'autologon: set writes the four values' ($on.AutoAdminLogon -eq '1' -and $on.DefaultUserName -eq 'tester' -and $on.DefaultDomainName -eq 'BOX' -and $on.DefaultPassword -eq 'not-a-real-password')
Assert-That 'autologon: the previous values are returned' ($before.DefaultUserName -eq 'someone' -and $null -eq $before.DefaultPassword)
Clear-GuestAutoLogon -Before ($before | ConvertTo-Json | ConvertFrom-Json) -KeyPath $fakeKey
$off = Get-ItemProperty -LiteralPath $fakeKey
Assert-That 'autologon: clear removes the password and restores the earlier values' ($null -eq $off.DefaultPassword -and $off.AutoAdminLogon -eq '0' -and $off.DefaultUserName -eq 'someone')
Remove-Item -LiteralPath $fakeKey -Recurse -Force -ErrorAction SilentlyContinue


# 3c. The route-state check used before the live update/remove, extracted from the runner and run on fake output.
$runnerAst = Get-ScriptAst 'run-stage-a.ps1'
foreach ($fn in $runnerAst.FindAll($functionFilter, $true) | Where-Object { $_.Name -in 'Read-SharedText', 'Test-RouteRunning' }) { . ([scriptblock]::Create($fn.Extent.Text)) }
$routeFile = Join-Path ([IO.Path]::GetTempPath()) 'round2-route-test.txt'
$fake = [pscustomobject]@{ HasExited = $false; ExitCode = 0; OutFile = $routeFile }
Remove-Item -LiteralPath $routeFile -Force -ErrorAction SilentlyContinue
Assert-That 'route check: no output yet is not running' (-not (Test-RouteRunning $fake).Ok)
$writer = [IO.File]::Open($routeFile, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::ReadWrite)
$bytes = [Text.Encoding]::ASCII.GetBytes("progress 1000 ms: cableRoute={`"state`":`"starting`"} max_service_gap_us=1`nprogress 2000 ms: cableRoute={`"state`":`"running`"} max_service_gap_us=1`n")
$writer.Write($bytes, 0, $bytes.Length); $writer.Flush()
Assert-That 'route check: reads the file while the route still holds it open; newest state wins' ((Test-RouteRunning $fake).Ok)
$bytes = [Text.Encoding]::ASCII.GetBytes("progress 3000 ms: cableRoute={`"state`":`"recovering`"} max_service_gap_us=1`n")
$writer.Write($bytes, 0, $bytes.Length); $writer.Flush(); $writer.Dispose()
Assert-That 'route check: a newest state other than running is not ready' (-not (Test-RouteRunning $fake).Ok)
$fake.HasExited = $true
Assert-That 'route check: an exited route is not ready' (-not (Test-RouteRunning $fake).Ok)
Remove-Item -LiteralPath $routeFile -Force -ErrorAction SilentlyContinue

# 4. Growth fit.
$flat = 0..29 | ForEach-Object { [pscustomobject]@{ Minutes = [double]$_; Bytes = 134MB } }
Assert-That 'growth: flat series is 0 MiB/h' ((Get-SeriesGrowthMiBPerHour $flat) -eq 0)
$rising = 0..29 | ForEach-Object { [pscustomobject]@{ Minutes = [double]$_; Bytes = 134MB + $_ * 1MB } }
Assert-That 'growth: 1 MiB per minute is 60 MiB/h' ([Math]::Abs((Get-SeriesGrowthMiBPerHour $rising) - 60) -lt 0.01) "$(Get-SeriesGrowthMiBPerHour $rising)"
Assert-That 'growth: too few points gives null' ($null -eq (Get-SeriesGrowthMiBPerHour @($flat[0..3])))

# 5. Pool tag reader against the host's own table (read-only system query).
$sample = Get-DriverPoolTagSample
$whole = [double](Get-CimInstance Win32_PerfRawData_PerfOS_Memory).PoolNonpagedBytes
Assert-That 'pool tags: the table has many tags' ($sample.TagsSeen -gt 100) "$($sample.TagsSeen)"
$ratio = $sample.TotalNonPagedBytes / $whole
Assert-That 'pool tags: tag sum matches the non-paged pool counter (0.5x to 1.5x)' ($ratio -gt 0.5 -and $ratio -lt 1.5) "ratio $ratio"
# Another SysVAD-derived driver on the host already uses MNWR (7 KiB here), so the tag alone is
# not proof of AudioRouter's pool; the guest has no other such driver, and only growth is judged.
Assert-That 'pool tags: driver tag columns are present and numeric' ($sample.MNWR -ge 0 -and $sample.'MNWR-allocs' -ge 0 -and $sample.AUDR -ge 0)
Initialize-PoolTagReader
$known = @([AudioRouterPoolTags]::Read() | Where-Object { $_.Tag -in 'Ntfn', 'File', 'Thre', 'Proc', 'MmCa', 'Even' })
Assert-That 'pool tags: well-known kernel tags decode as text' ($known.Count -ge 2) "found $($known.Count)"

# 6. Host-load rows: the PC sampler's CSV, judged by row position, not clock.
#    CPU has three columns so the guest's own vCPUs are not host starvation.
function New-HostLoadCsv([double[]] $Total, [double[]] $Other, [double[]] $Vm, [double[]] $Avail) {
    $lines = @('utc,cpu_pct,cpu_other_pct,vm_cpu_pct,avail_mb,pages_sec')
    $base = [DateTime]::UtcNow.Date.AddHours(22)
    for ($i = 0; $i -lt $Total.Count; $i++) {
        $utc = $base.AddSeconds(20 * $i).ToString('yyyy-MM-ddTHH:mm:ssZ', [Globalization.CultureInfo]::InvariantCulture)
        $lines += '{0},{1},{2},{3},{4},1' -f $utc, [int]$Total[$i], [int]$Other[$i], [int]$Vm[$i], [int]$Avail[$i]
    }
    return $lines
}
$csv = @(
    'utc,cpu_pct,cpu_other_pct,vm_cpu_pct,avail_mb,pages_sec',
    '2026-11-24T22:00:00Z,17,0,17,12000,100',
    '2026-11-24T22:00:20Z,90,89,1,332,90000',
    '2026-11-24T22:00:40Z,18,2,16,11000,50',
    '2026-11-24T22:01:00Z,17,1,16,11500,40',
    '2026-11-24T22:01:20Z,7,3',
    '2026-11-24T22:01:40Z,7,-1,0,9000,3',
    '2026-11-24T22:02:00Z,5,11200,40',
    'garbage line',
    ''
)
$rows = @(Get-HostLoadRows $csv)
Assert-That 'host load: short, negative and garbage rows are ignored' ($rows.Count -eq 5) "$($rows.Count)"
Assert-That 'host load: total, non-VM and VM CPU are parsed' ($rows[0].Utc.Hour -eq 22 -and $rows[0].CpuPercent -eq 17 -and $rows[0].OtherCpuPercent -eq 0 -and $rows[0].VmCpuPercent -eq 17 -and $rows[0].AvailableMb -eq 12000 -and $rows[0].PagesPerSec -eq 100 -and $rows[0].HasBreakdown)
Assert-That 'host load: an old four-column row is read as total-only' (-not $rows[4].HasBreakdown -and $rows[4].OtherCpuPercent -eq 5 -and $rows[4].VmCpuPercent -eq 0)
$two = Get-HostLoadWindow -Rows $rows -From 0 -To 2
Assert-That 'host load: a window by row position takes the right rows' ($two.Count -eq 2 -and $two.MeanOtherCpuPercent -eq 44.5 -and $two.MaxOtherCpuPercent -eq 89 -and $two.WorstCpuPercent -eq 90 -and $two.MinAvailableMb -eq 332 -and $two.MedianAvailableMb -eq 6166 -and $two.LastUtc.Minute -eq 0)
$tail = Get-HostLoadWindow -Rows $rows -From 2
Assert-That 'host load: From without To reads to the end and flags legacy rows' ($tail.Count -eq 3 -and -not $tail.HasBreakdown) "$($tail.Count)"

# Owner-rule cases the manager asked for. Guest-only load is not starvation.
$guestOnly = @(Get-HostLoadRows (New-HostLoadCsv (@(0..9 | ForEach-Object { 17 })) (@(0..9 | ForEach-Object { 0 })) (@(0..9 | ForEach-Object { 17 })) (@(0..9 | ForEach-Object { 8000 }))))
$guestVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $guestOnly)
Assert-That 'host load: a busy guest alone (17 % total, 0 % other) is Idle' ($guestVerdict.Verdict -eq 'Idle') $guestVerdict.Detail
$sustained = @(Get-HostLoadRows (New-HostLoadCsv (@(0..9 | ForEach-Object { 40 })) (@(0..9 | ForEach-Object { 24 })) (@(0..9 | ForEach-Object { 16 })) (@(0..9 | ForEach-Object { 8000 }))))
$sustainedVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $sustained)
Assert-That 'host load: sustained non-VM CPU above 20 % is Starved' ($sustainedVerdict.Verdict -eq 'Starved' -and $sustainedVerdict.Detail -match 'sustained') $sustainedVerdict.Detail
# One spike in ten 20-second samples does not move a mean or a 50 % fraction.
$oneSpike = @(3, 3, 3, 3, 3, 3, 3, 3, 3, 89)
$spikeRows = @(Get-HostLoadRows (New-HostLoadCsv @(0..9 | ForEach-Object { 40 }) $oneSpike (@(0..9 | ForEach-Object { 0 })) (@(0..9 | ForEach-Object { 8000 }))))
$spikeVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $spikeRows)
Assert-That 'host load: one CPU spike in a block is not starvation' ($spikeVerdict.Verdict -eq 'Idle') $spikeVerdict.Detail
# Even a three-sample window with one 95 % sample is not sustained.
$shortSpike = @(Get-HostLoadRows (New-HostLoadCsv @(5, 95, 5) @(5, 95, 5) @(0, 0, 0) @(8000, 8000, 8000)))
$shortVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $shortSpike)
Assert-That 'host load: one spike in a short window is still not sustained' ($shortVerdict.Verdict -eq 'Idle') $shortVerdict.Detail
$single = @(Get-HostLoadRows (New-HostLoadCsv @(95) @(95) @(0) @(8000)))
Assert-That 'host load: a single sample is NoData, never Starved or Idle' ((Get-HostLoadVerdict (Get-HostLoadWindow -Rows $single)).Verdict -eq 'NoData')
# Memory: the floor is judged on the median, so one dip is not starvation.
$dip = @(3000, 10000, 10000, 10000, 10000, 10000, 10000, 10000, 10000, 10000)
$dipRows = @(Get-HostLoadRows (New-HostLoadCsv (@(0..9 | ForEach-Object { 5 })) (@(0..9 | ForEach-Object { 2 })) (@(0..9 | ForEach-Object { 3 })) $dip))
$dipVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $dipRows)
Assert-That 'host load: one memory dip does not deny an idle host' ($dipVerdict.Verdict -eq 'Idle') $dipVerdict.Detail
$lowMemory = @(Get-HostLoadRows (New-HostLoadCsv (@(0..9 | ForEach-Object { 5 })) (@(0..9 | ForEach-Object { 2 })) (@(0..9 | ForEach-Object { 3 })) (@(0..9 | ForEach-Object { 4000 }))))
Assert-That 'host load: memory held below 6000 MB is Starved' ((Get-HostLoadVerdict (Get-HostLoadWindow -Rows $lowMemory)).Verdict -eq 'Starved')
$limits = @(Get-HostLoadRows (New-HostLoadCsv (@(0..9 | ForEach-Object { 20 })) (@(0..9 | ForEach-Object { 20 })) (@(0..9 | ForEach-Object { 0 })) (@(0..9 | ForEach-Object { 6000 }))))
Assert-That 'host load: the rule limits themselves are still Idle' ((Get-HostLoadVerdict (Get-HostLoadWindow -Rows $limits)).Verdict -eq 'Idle')
$legacyLines = @('utc,cpu_pct,avail_mb,pages_sec') + @(0..9 | ForEach-Object { '2026-11-24T23:00:{0:D2}Z,30,8000,1' -f $_ })
$legacy = @(Get-HostLoadRows $legacyLines)
$legacyVerdict = Get-HostLoadVerdict (Get-HostLoadWindow -Rows $legacy)
Assert-That 'host load: old total-only rows still apply the 20 % rule' ($legacyVerdict.Verdict -eq 'Starved' -and $legacyVerdict.Detail -match 'legacy rows') $legacyVerdict.Detail
Assert-That 'host load: an empty window is NoData, never PASS' ((Get-HostLoadVerdict (Get-HostLoadWindow -Rows $rows -From 9)).Verdict -eq 'NoData')

# The guest must require the sampler and record host load per block, not rely
# on a night's operator memory.
$runnerText = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'run-stage-a.ps1') -Raw
Assert-That 'runner: round2 stops without a host-load sampler' ($runnerText -match 'host-load sampler' -and $runnerText -match "'SKIP' 'the host-load sampler check did not pass")
Assert-That 'runner: each round2 block records host load' ($runnerText -match 'Add-HostLoadResult "\$block block" \$from')
Assert-That 'runner: an unexpected restart is classified against host load' ($runnerText -match 'until the restart')
# A 24 h pool run: allowed by the parameter, its state survives the JSON
# round-trip, its samples are copied to the share, and the sampler outlasts it.
$longSeries = 0..2879 | ForEach-Object { [pscustomobject]@{ Minutes = $_ / 2.0; Bytes = 134MB } }
Assert-That 'pool: a 24 h tagged series (2,880 samples) grades flat' ((Get-SeriesGrowthMiBPerHour $longSeries) -eq 0)
$stateBack = ([ordered]@{ session = 'round2'; done = @('x'); results = @(); current = $null; crashed = $false; startedUtc = '2026-11-24T00:00:00Z'; hostRows = 1234 } | ConvertTo-Json -Depth 5 | ConvertFrom-Json)
Assert-That 'pool: the run state keeps hostRows and its session across a restart' ($stateBack.hostRows -eq 1234 -and $stateBack.session -eq 'round2')
Assert-That 'pool: -PoolMinutes allows 1440 and defaults to 120' ($runnerText -match '\[ValidateRange\(5, 1440\)\]\[int\] \$PoolMinutes = 120')
Assert-That 'pool: the samples are copied to the share while the block runs' ($runnerText -match 'Copy-Item -LiteralPath \$csv -Destination \(Join-Path \$peer .pool-tags\.csv.\)')
$samplerText = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'host-load.ps1') -Raw
Assert-That 'sampler: up to 48 h and the six-column format' ($samplerText -match 'ValidateRange\(1, 2880\)' -and $samplerText -match 'cpu_other_pct')
$preflightText = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'round2-host-preflight.ps1') -Raw
Assert-That 'preflight: the sampler window outlasts a 24 h run' ($preflightText -match '\$SamplerMinutes = 1560')

# 7. Host preflight: the idle gate and the sampler check (library mode only).
. (Join-Path $PSScriptRoot 'round2-host-preflight.ps1') -LibraryOnly
$idle = [pscustomobject]@{ Verdict = 'Idle'; Detail = 'ok' }
$busy = [pscustomobject]@{ Verdict = 'Starved'; Detail = 'cpu 91' }
Assert-That 'preflight: an idle PC has no reasons' (@(Get-PreflightReasons -Verdict $idle -Wsl @()).Count -eq 0)
Assert-That 'preflight: a busy PC is refused with its reason' (((@(Get-PreflightReasons -Verdict $busy -Wsl @())) -join '; ') -match 'cpu 91')
Assert-That 'preflight: a running WSL distro is refused' (((@(Get-PreflightReasons -Verdict $idle -Wsl @('Ubuntu Running'))) -join '; ') -match 'wsl --shutdown')
$live = Measure-HostIdle -Seconds 2 -IntervalSeconds 1
Assert-That 'preflight: a live measurement returns a usable window' ($live.Count -ge 1 -and $live.MeanOtherCpuPercent -ge 0 -and $live.MedianAvailableMb -ge 0) "count $($live.Count)"
Assert-That 'preflight: a missing sampler file is not running' (-not (Test-SamplerRunning -Path (Join-Path ([IO.Path]::GetTempPath()) 'no-such-host-load.csv')))
$tempCsv = Join-Path ([IO.Path]::GetTempPath()) 'round2-host-load-test.csv'
'utc,cpu_pct,cpu_other_pct,vm_cpu_pct,avail_mb,pages_sec' | Set-Content -LiteralPath $tempCsv -Encoding ASCII
(Get-Item -LiteralPath $tempCsv).LastWriteTime = (Get-Date).AddMinutes(-10)
Assert-That 'preflight: a stale sampler file is not running' (-not (Test-SamplerRunning -Path $tempCsv))
(Get-Item -LiteralPath $tempCsv).LastWriteTime = (Get-Date)
Assert-That 'preflight: a fresh sampler file is running' (Test-SamplerRunning -Path $tempCsv)
Remove-Item -LiteralPath $tempCsv -Force -ErrorAction SilentlyContinue
Assert-That 'preflight: heavy-process and WSL probes return arrays' ($null -ne @(Get-HeavyProcesses) -and $null -ne @(Get-RunningWsl))

# 7b. The real sampler writes the six-column format (short run, then stopped).
$sampleCsv = Join-Path ([IO.Path]::GetTempPath()) 'round2-sampler-format.csv'
Remove-Item -LiteralPath $sampleCsv -Force -ErrorAction SilentlyContinue
$sampler = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
    -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'host-load.ps1'),
        '-OutputPath', $sampleCsv, '-Minutes', '1', '-IntervalSeconds', '5') -WindowStyle Hidden -PassThru
try {
    $deadline = (Get-Date).AddSeconds(20)
    $samplerRows = @()
    do {
        Start-Sleep -Seconds 1
        if (Test-Path -LiteralPath $sampleCsv) { $samplerRows = @(Get-HostLoadRows @(Get-Content -LiteralPath $sampleCsv)) }
    } while ($samplerRows.Count -lt 2 -and (Get-Date) -lt $deadline)
    Assert-That 'sampler: writes two or more six-column rows' ($samplerRows.Count -ge 2 -and $samplerRows[0].HasBreakdown -and $samplerRows[0].VmCpuPercent -ge 0) "$($samplerRows.Count) rows"
} finally {
    if (-not $sampler.HasExited) { Stop-Process -Id $sampler.Id -Force -ErrorAction SilentlyContinue }
    Remove-Item -LiteralPath $sampleCsv -Force -ErrorAction SilentlyContinue
}

if ($script:failures -gt 0) { Write-Host "`n$($script:failures) check(s) failed." -ForegroundColor Red; exit 1 }
Write-Host "`nAll round-2 host checks passed."
exit 0
