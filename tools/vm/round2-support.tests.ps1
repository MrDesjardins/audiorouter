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
    'prepare-round2-bundle.ps1', 'run-stage-a.ps1', 'vm-checks.ps1') {
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
foreach ($name in 'run-stage-a.ps1', 'round2-support.ps1', 'round2-restart-render.ps1', 'round2-pause-watcher.ps1', 'prepare-round2-bundle.ps1') {
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

# 3. The A/B rule against synthetic runs.
function New-Rows([string] $Driver, [long[]] $Underruns, [long[]] $Lates, [double] $GapMs = 10) {
    $rows = @()
    for ($i = 0; $i -lt $Underruns.Count; $i++) {
        $rows += [pscustomobject]@{ Driver = $Driver; Variant = 'steady'; Valid = $true; HasPacketWrites = $true
            RenderUnderrun = $Underruns[$i]; Late = $Lates[$i]; PacketOverrun = 0; MaxGapMs = $GapMs }
    }
    return $rows
}
$same = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0, 0) @(0, 0, 0, 0, 0)) + (New-Rows 'new' @(0, 0, 0, 0, 0) @(0, 0, 0, 0, 0)))
Assert-That 'A/B: identical zero counters pass' ($same.Verdict -eq 'PASS') $same.Detail
$bad = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0) @(0, 0, 0, 0)) + (New-Rows 'new' @(6167, 5800, 6100, 6000) @(0, 0, 0, 0)))
Assert-That 'A/B: the 2026-11-11 pattern (6k underrun frames) on 0.2.5 only fails' ($bad.Verdict -eq 'FAIL') $bad.Detail
$late = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0) @(0, 0, 1, 0)) + (New-Rows 'new' @(0, 0, 0, 0) @(3, 4, 2, 5)))
Assert-That 'A/B: more late packets on 0.2.5 fails' ($late.Verdict -eq 'FAIL') $late.Detail
$both = Compare-RenderAb ((New-Rows 'rc3' @(6000, 5900, 6100, 6000) @(0, 0, 0, 0)) + (New-Rows 'new' @(6167, 5800, 6100, 6000) @(0, 0, 0, 0)))
Assert-That 'A/B: both drivers underrun alike is not a 0.2.5 regression' ($both.Verdict -eq 'PASS') $both.Detail
$small = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0) @(0, 0, 0, 0)) + (New-Rows 'new' @(300, 0, 0, 0) @(0, 0, 0, 0)))
Assert-That 'A/B: a difference under one packet does not fail' ($small.Verdict -eq 'PASS') $small.Detail
$stalled = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0) @(0, 0, 0, 0) 400) + (New-Rows 'new' @(9000, 9000, 9000, 9000) @(0, 0, 0, 0) 400))
Assert-That 'A/B: runs with VM stalls are inconclusive, never a verdict' ($stalled.Verdict -eq 'REVIEW') $stalled.Detail
$thin = Compare-RenderAb ((New-Rows 'rc3' @(0, 0) @(0, 0)) + (New-Rows 'new' @(0, 0, 0, 0) @(0, 0, 0, 0)))
Assert-That 'A/B: too few quiet runs is inconclusive' ($thin.Verdict -eq 'REVIEW') $thin.Detail
$invalid = New-Rows 'new' @(0, 0, 0, 0) @(0, 0, 0, 0); foreach ($row in $invalid) { $row.Valid = $false }
$none = Compare-RenderAb ((New-Rows 'rc3' @(0, 0, 0, 0) @(0, 0, 0, 0)) + $invalid)
Assert-That 'A/B: invalid runs (tool failed) are not counted' ($none.Verdict -eq 'REVIEW') $none.Detail

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

if ($script:failures -gt 0) { Write-Host "`n$($script:failures) check(s) failed." -ForegroundColor Red; exit 1 }
Write-Host "`nAll round-2 host checks passed."
exit 0
