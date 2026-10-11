<# Host-safe checks of the eight-cable continuity runner: its progress parsing and VM-wide
   stall attribution, then its orchestration with fake process boundaries (no driver, helper,
   probe or audio stream is started), plus the real probe self-test and the VM guard. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $workspace 'tools\vm\portable-tool-support.ps1')
$root = Join-Path $workspace ('target\cable-continuity-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$runner = Join-Path $workspace 'tools\vm\run-cable-continuity.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
. $runner  # dot-sourcing defines the analysis functions and returns before any guest action

# Progress parsing on a real line from VM run r11.
$line = 'progress 4016 ms: written=232 read=213 ack=231 interval_capture_gap_us=3256 interval_render_gap_us=3055 interval_control_gap_us=7306 capture=NativeBridgeStreamCounters { underrun_frames: 0, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 110064, last_qpc_time: 12733712074 } render=NativeBridgeStreamCounters { underrun_frames: 480, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 102288, last_qpc_time: 12733713589 } render_packets=Some(NativeBridgePacketCounters { accepted: 214, late: 0, overrun: 0 })'
$entry = ConvertFrom-ProgressLine $line
Assert ($entry.Ms -eq 4016 -and $entry.MaxGapMs -eq 7.306 -and $entry.Errors -eq 480) 'progress line: time, largest gap, error total (accepted packets and positions excluded)'
Assert ($null -eq (ConvertFrom-ProgressLine 'driver: info')) 'non-progress lines ignored'
$late = $line.Replace('late: 0', 'late: 2').Replace('interval_render_gap_us=3055', 'interval_render_gap_us=52000')
Assert ((ConvertFrom-ProgressLine $late).Errors -eq 482 -and (ConvertFrom-ProgressLine $late).MaxGapMs -eq 52) 'late packet writes count; render gap can be the largest'

function New-ProgressLine([int] $Ms, [int] $GapUs, [int] $Underrun) {
    return "progress $Ms ms: written=1 read=1 ack=1 interval_capture_gap_us=3000 interval_render_gap_us=$GapUs interval_control_gap_us=7000 capture=NativeBridgeStreamCounters { underrun_frames: 0, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 1, last_qpc_time: 1 } render=NativeBridgeStreamCounters { underrun_frames: $Underrun, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0, last_device_position: 1, last_qpc_time: 1 } render_packets=Some(NativeBridgePacketCounters { accepted: 9, late: 0, overrun: 0 })"
}
$intervals = @(Get-ProgressIntervals @((New-ProgressLine 1000 3000 480), (New-ProgressLine 2000 3000 480), (New-ProgressLine 3000 50000 1920), (New-ProgressLine 4000 3000 1920)) 100.0)
Assert ($intervals.Count -eq 4 -and $intervals[0].From -eq 100.0 -and $intervals[0].To -eq 101.0 -and $intervals[0].Added -eq 480) 'first interval starts at the route start'
Assert ($intervals[1].Added -eq 0 -and $intervals[2].Added -eq 1440 -and $intervals[2].MaxGapMs -eq 50 -and $intervals[2].From -eq 102.0 -and $intervals[2].To -eq 103.0) 'counter increases are per interval'
$stalled = $intervals
$quiet = @(Get-ProgressIntervals @((New-ProgressLine 1000 3000 0), (New-ProgressLine 2000 3000 0), (New-ProgressLine 3000 3000 0), (New-ProgressLine 4000 3000 0)) 100.0)
Assert (Test-VmWideStall 102.5 @(,$stalled) ) 'one route stalled at the event'
Assert (Test-VmWideStall 102.5 @($stalled, $stalled, $stalled, $stalled)) 'all four routes stalled: VM-wide'
Assert (-not (Test-VmWideStall 102.5 @($stalled, $stalled, $stalled, $quiet))) 'one route without a gap: not VM-wide'
Assert ((Test-VmWideStall 104.4 @($stalled, $stalled)) -and -not (Test-VmWideStall 104.6 @($stalled, $stalled))) 'tolerance of 1.5 s around the stall interval'
Assert (-not (Test-VmWideStall 102.5 @($stalled, @()))) 'a route with no progress cannot confirm a stall'

# Orchestration with fake process boundaries.
$body = $ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $node.Extent.Text.Contains('Cable continuity: 8 cables') }, $true)
$helpers = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -in @('Invoke-Helper','Get-Inventory','Get-EndpointIndex','Get-CableCount') }, $true))
$pairsAst = $ast.Find({ param($node) $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$pairs' }, $true)
Assert ($null -ne $body -and $helpers.Count -eq 4 -and $null -ne $pairsAst) 'runner body, helpers and pair table found'
$execution = [scriptblock]::Create($body.Extent.Text)

function New-Inventory([int] $Cables) {
    $lines = @('render[0] name=Speakers (High Definition Audio Device) id=x')
    $letters = @('A','B','C','D','E','F','G','H')
    for ($index = 0; $index -lt $Cables; $index++) {
        $lines += "render[$($index + 1)] name=AudioRouter Cable $($letters[$index]) Input (AudioRouter Virtual Cable) id={r$index}"
        $lines += "capture[$($index + 1)] name=AudioRouter Cable $($letters[$index]) Output (AudioRouter Virtual Cable) id={c$index}"
    }
    return $lines -join "`n"
}

function Test-Continuity([string] $Case) {
    $evidence = Join-Path $root $Case
    $bundle = Join-Path $evidence 'bundle'
    $peer = Join-Path $evidence 'peer'
    New-Item -ItemType Directory -Path $evidence,$bundle,$peer | Out-Null
    $run = $Case; $Minutes = 1
    $probe = 'fake-probe'; $tone = 'fake-tone'; $helper = 'fake-helper'
    foreach ($definition in $helpers) { . ([scriptblock]::Create($definition.Extent.Text)) }
    . ([scriptblock]::Create($pairsAst.Extent.Text))
    $routes = @{}; $probes = @{}; $results = @(); $failure = $null; $cablesBefore = $null; $restored = $null; $verdict = $null
    $script:cables = if ($Case -eq 'already-8') { 8 } else { 2 }
    $script:helperCalls = @(); $script:tones = @(); $script:probeRuns = @(); $script:killed = 0
    $script:clock = 1000.0
    function Get-Seconds { $script:clock += 0.001; return $script:clock }
    function Start-Sleep { param($Seconds, $Milliseconds) }
    function Invoke-DriverVmProcess {
        param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
        Set-Content -LiteralPath $Stderr -Value ''
        if ($Executable -eq $helper) {
            Assert ($env:AUDIOROUTER_ALLOW_TEST_DRIVER -eq '1' -and $Arguments[0] -eq 'set-cables' -and $Arguments -contains '--result') 'helper runs set-cables with a result file and the test-driver opt-in'
            $script:helperCalls += $Arguments[2]
            Set-Content -LiteralPath $Stdout -Value 'helper'
            if ($Case -eq 'set-cables-failure' -and $Arguments[2] -eq '8') { return @{ Code = 1 } }
            $script:cables = [int]$Arguments[2]
            return @{ Code = 0 }
        }
        Assert ($Executable -eq $probe -and $Arguments[0] -eq 'inventory') 'only inventory runs synchronously'
        Set-Content -LiteralPath $Stdout -Value (New-Inventory $script:cables)
        return @{ Code = 0 }
    }
    function Start-Process {
        param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru,$RedirectStandardOutput,$RedirectStandardError)
        Assert ($WindowStyle -eq 'Hidden') 'hidden processes only'
        Set-Content -LiteralPath $RedirectStandardError -Value ''
        $name = [IO.Path]::GetFileName($RedirectStandardOutput) -replace '-(tone|probe)\.txt$', ''
        if ($FilePath -eq $tone) {
            $script:tones += [string]$ArgumentList
            $ready = -not ($Case -eq 'route-early-exit' -and $name -eq 'C-to-D')
            Set-Content -LiteralPath $RedirectStandardOutput -Value $(if ($ready) { "render worker polling before lease activation`nengine route quanta: processed=28000 silent=0`npass-through relay: forwarded=6000 silence=20 dropped=0`ncapture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 0 }`nrender-source counters (cable-a): NativeBridgeStreamCounters { underrun_frames: 480 }`nrender-source packet writes (cable-a): NativeBridgePacketCounters { accepted: 6000, late: 0, overrun: 0 }" } else { 'driver: info' })
            # Progress (tool-relative ms): a 50 ms gap at 3-4 s on every route
            # for the VM-wide case, on A-to-B only for the unexplained case.
            $stall = ($Case -eq 'vm-wide') -or ($Case -eq 'unexplained' -and $name -eq 'A-to-B')
            $progressLines = for ($second = 1; $second -le 62; $second++) {
                $underrun = if ($second -ge 4 -and ($Case -in 'vm-wide','unexplained') -and $name -eq 'A-to-B') { 1920 } else { 480 }
                New-ProgressLine ($second * 1000) $(if ($stall -and $second -eq 4) { 50000 } else { 3000 }) $underrun
            }
            Set-Content -LiteralPath (Join-Path $evidence "$name.progress.txt") -Value $progressLines
            $exit = if ($ready) { 0 } else { 3 }
            $fake = [pscustomobject]@{ Handle = 1; HasExited = (-not $ready); ExitCode = $exit; Kind = 'tone' }
        } else {
            $script:probeRuns += [string]$ArgumentList
            $lost = if ($Case -in 'vm-wide','unexplained' -and $name -eq 'A-to-B') { "cable_lost_runs=1`ncable_lost_run first_impulse=300 count=3" } else { 'cable_lost_runs=0' }
            $lostCount = if ($Case -in 'vm-wide','unexplained' -and $name -eq 'A-to-B') { 3 } else { 0 }
            $corrupted = if ($Case -eq 'corrupted' -and $name -eq 'G-to-H') { 2 } else { 0 }
            Set-Content -LiteralPath $RedirectStandardOutput -Value "cable_render_frames_submitted=1 cable_capture_frames=1 cable_capture_dropped_frames=0 cable_capture_flagged_packets=1 cable_arrivals=6000`ncable_dating=in-run-median cable_render_clock_samples=6000 cable_render_clock_overflow=0 cable_anchor_dated_impulses=0 cable_render_discontinuities=0 cable_render_discontinuity_ms=0.000`ncable_impulses_emitted=6000 cable_impulses_matched=$(6000 - $lostCount) cable_impulses_lost=$lostCount cable_impulses_corrupted=$corrupted cable_impulses_duplicate=0 cable_impulses_out_of_window=0`ncable_latency_p50_ms=38.5`ncable_segments=1 cable_steady_level_ms=38.5`n$lost"
            $fake = [pscustomobject]@{ Handle = 1; HasExited = $false; ExitCode = 0; Kind = 'probe' }
        }
        $fake | Add-Member ScriptMethod WaitForExit {
            param($Milliseconds)
            if ($this.Kind -eq 'probe' -and $Case -eq 'probe-hang') { return $false }
            if ($this.Kind -eq 'probe') { $script:clock += 60.0 }
            $this.HasExited = $true; return $true
        }
        $fake | Add-Member ScriptMethod Kill { $this.HasExited = $true; $script:killed++ }
        return $fake
    }
    function Compress-Archive {
        param($LiteralPath,$DestinationPath)
        Assert ($LiteralPath -eq $evidence) 'archive contains only this run'
        Set-Content -LiteralPath $DestinationPath -Value 'fake archive'
    }
    . ([scriptblock]::Create($ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $node.Extent.Text.Contains('Cable continuity: 8 cables') }, $true).Extent.Text))
    Assert (Test-Path -LiteralPath (Join-Path $peer "$run.zip")) "$Case evidence copied"
    $saved = Read-PairedTraceJson (Join-Path $evidence 'result.json')
    Assert ($saved.Qualification -eq $false -and $saved.Scope -match 'deviation run') "$Case never claims qualification"
    if ($Case -ne 'already-8') {
        Assert ($script:helperCalls[0] -eq '8' -and $script:helperCalls[-1] -eq '2' -and $saved.CableCountRestored -eq $true) "$Case enables eight cables and restores two"
    }
    switch ($Case) {
        { $_ -in 'clean','vm-wide','unexplained','already-8' } {
            Assert (-not $failure) "$Case completes"
            Assert ($script:tones.Count -eq 4 -and $script:probeRuns.Count -eq 4) 'four routes and four impulse trains'
            Assert (@($script:tones | Where-Object { $_ -notmatch '"--engine"' -or $_ -notmatch '"--discard"' -or $_ -notmatch '"--seconds" "100"' }).Count -eq 0) 'engine routes keep no WAV and outlive the impulses'
            Assert ($script:tones[0] -match '"--render-bus" "cable-a"' -and $script:tones[0] -match '"--capture-bus" "cable-b"' -and $script:tones[3] -match '"--render-bus" "cable-g"' -and $script:tones[3] -match '"--capture-bus" "cable-h"') 'A->B ... G->H'
            Assert ($script:probeRuns[0] -match '^cable-impulse 6000 1 2 raw=' -and $script:probeRuns[3] -match '^cable-impulse 6000 7 8 raw=') 'each impulse train uses its pair''s Input and Output'
        }
        'clean' { Assert ($verdict -eq 'clean' -and @($results | ForEach-Object { $_.Events }).Count -eq 0) 'no event: clean' }
        'already-8' { Assert ($script:helperCalls.Count -eq 0 -and $null -eq $saved.CableCountRestored) 'eight cables already: no helper call, nothing to restore' }
        'vm-wide' {
            $events = @($results[0].Events)
            Assert ($events.Count -eq 2 -and @($events | Where-Object { $_.VmWide }).Count -eq 2) 'the loss and the counter increase both coincide with the four-route stall'
            Assert ($verdict -eq 'all-events-vm-wide') 'documented deviation verdict'
        }
        'unexplained' {
            Assert (@($results[0].Events | Where-Object { -not $_.VmWide }).Count -eq 2 -and $verdict -eq 'unexplained-events') 'a stall on one route only does not explain its loss'
        }
        'set-cables-failure' { Assert ([bool]$failure -and $script:tones.Count -eq 0 -and $script:probeRuns.Count -eq 0) 'no audio when eight cables cannot be enabled' }
        'route-early-exit' { Assert ([bool]$failure -and [string]$failure -match 'C-to-D route exited' -and $script:probeRuns.Count -eq 0) 'a route that fails to start stops the run before any impulse' }
        'probe-hang' { Assert ([bool]$failure -and $script:killed -ge 1) 'a hung impulse train is killed' }
        'corrupted' { Assert ([bool]$failure -and [string]$failure -match 'corrupted') 'corrupted impulses fail the run' }
    }
}
foreach ($case in @('clean','vm-wide','unexplained','already-8','set-cables-failure','route-early-exit','probe-hang','corrupted')) { Test-Continuity $case }

$probeBinary = Join-Path $workspace 'target\m00-probe-cable\m00-probe.exe'
$imports = @(Assert-PortableVmTool $probeBinary)
Assert (@($imports | Where-Object { $_ -match 'vcruntime|msvcp' }).Count -eq 0) 'static probe has no redistributable imports'
$selfTest = Invoke-DriverVmProcess -Executable $probeBinary -Arguments @('cable-impulse-selftest') -Stdout (Join-Path $root 'selftest.txt') -Stderr (Join-Path $root 'selftest-stderr.txt') -TimeoutSeconds 20
Assert ($selfTest.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'selftest.txt') -Raw) -match '\d+ cable impulse pairing checks pass') 'real probe self-test, including lost-run reporting'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'wrapper-guard.txt') -Stderr (Join-Path $root 'wrapper-guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'wrapper-guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks cable-continuity checks passed. No audio stream, helper or driver tool was opened. Evidence: $root"
