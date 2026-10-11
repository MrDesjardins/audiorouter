<# Host-safe checks of the repeated-start cable latency runner with fake process boundaries,
   plus the real static probe's steady-level self-test and the VM guard. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $workspace 'tools\vm\portable-tool-support.ps1')
$root = Join-Path $workspace ('target\cable-latency-starts-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$tokens = $null; $errors = $null
$runner = Join-Path $workspace 'tools\vm\run-cable-latency-starts.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
foreach ($function in @('Get-ProbeValue', 'Get-Median')) {
    $definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $function }, $true)
    . ([scriptblock]::Create($definition.Extent.Text))
}
$assignments = @($ast.FindAll({ param($node) $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -in @('$endpoints', '$modes') }, $true))
Assert ($assignments.Count -eq 2) 'endpoint and mode tables found'
$body = $ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $node.Extent.Text.Contains('Cable latency starts (') }, $true)
$execution = [scriptblock]::Create($body.Extent.Text)
Assert ((Get-Median @(3.0, 1.0, 2.0)) -eq 2.0 -and (Get-Median @(4.0, 1.0, 3.0, 2.0)) -eq 2.0 -and $null -eq (Get-Median @())) 'median of steady levels (lower middle)'

$cableRender = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$cableCapture = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
$vbRender = 'CABLE Input (VB-Audio Virtual Cable)'
$vbCapture = 'CABLE Output (VB-Audio Virtual Cable)'

function Get-FakeProbe([string] $Case, [int] $Call, [string] $Mode) {
    $level = if ($Mode -eq 'low-latency') { 12.5 + $Call } else { 25.25 + $Call }
    $shift = $Case -eq 'stall' -and $Call -eq 2
    return @"
cable_mode=$Mode cable_render_buffer_frames=$(if ($Mode -eq 'low-latency') { 280 } else { 1056 })
cable_render_frames_submitted=1 cable_capture_frames=1 cable_capture_dropped_frames=$(if ($shift) { 960 } else { 0 }) cable_capture_flagged_packets=1 cable_arrivals=400
cable_dating=in-run cable_render_clock_samples=500 cable_render_clock_overflow=0 cable_anchor_dated_impulses=0 cable_render_discontinuities=$(if ($shift) { 1 } else { 0 }) cable_render_discontinuity_ms=$(if ($shift) { 25.0 } else { 0.0 })
cable_impulses_emitted=400 cable_impulses_matched=$(if ($shift) { 396 } else { 400 }) cable_impulses_lost=$(if ($shift) { 4 } else { 0 }) cable_impulses_corrupted=0 cable_impulses_duplicate=0 cable_impulses_out_of_window=0
cable_latency_min_ms=$level cable_latency_p1_ms=$level cable_latency_p50_ms=$level cable_latency_p95_ms=$(if ($shift) { $level + 25 } else { $level }) cable_latency_p99_ms=$level cable_latency_max_ms=$level cable_latency_mean_ms=$level cable_jitter_p99_minus_p1_ms=0.003
cable_segments=$(if ($shift) { 2 } else { 1 }) cable_steady_level_ms=$level cable_steady_first_impulse=0 cable_steady_impulses=$(if ($shift) { 120 } else { 400 }) cable_steady_spread_ms=0.003
cable_end_anchor_p50_ms=$level cable_end_anchor_segments=1
"@
}

function Test-Starts([string] $Case, [string] $Route) {
    $evidence = Join-Path $root $Case
    $bundle = Join-Path $evidence 'bundle'
    $peer = Join-Path $evidence 'peer'
    New-Item -ItemType Directory -Path $evidence,$bundle,$peer | Out-Null
    $run = $Case
    $Starts = 3; $Impulses = 400
    $powershell = 'fake-powershell'; $probe = 'fake-probe'; $tone = 'fake-tone'
    foreach ($assignment in $assignments) { . ([scriptblock]::Create($assignment.Extent.Text)) }
    $results = @(); $summary = @(); $failure = $null; $active = $null
    $script:started = @(); $script:probeCalls = @(); $script:killed = 0; $script:disposed = 0
    function Start-Sleep { param($Milliseconds) }
    function Start-Process {
        param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru,$RedirectStandardOutput,$RedirectStandardError)
        Assert ($FilePath -eq $tone -and $WindowStyle -eq 'Hidden') 'only the owned route tool is started, hidden'
        $script:started += [string]$ArgumentList
        $ready = $Case -ne 'early-exit'
        Set-Content -LiteralPath $RedirectStandardOutput -Value $(if ($ready) { "driver: info`nrender worker polling before lease activation`ncapture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 0 }`nrender-source counters (cable-a): NativeBridgeStreamCounters { underrun_frames: 0 }`nrender-source packet writes (cable-a): NativeBridgePacketCounters { accepted: 600, late: 0, overrun: 0 }`nengine route quanta: processed=2200 silent=0" } else { 'driver: info' })
        Set-Content -LiteralPath $RedirectStandardError -Value ''
        $fake = [pscustomobject]@{ Handle = 1; HasExited = (-not $ready); ExitCode = $(if ($ready) { 0 } else { 3 }) }
        $fake | Add-Member ScriptMethod WaitForExit { param($Milliseconds) if ($Case -eq 'tone-hang') { return $false }; $this.HasExited = $true; return $true }
        $fake | Add-Member ScriptMethod Dispose { $script:disposed++ }
        $fake | Add-Member ScriptMethod Kill { $this.HasExited = $true; $script:killed++ }
        return $fake
    }
    function Invoke-DriverVmProcess {
        param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
        Set-Content -LiteralPath $Stderr -Value ''
        if ($Arguments -contains 'status') {
            Set-Content -LiteralPath $Stdout -Value 'status'
            return @{ Code = $(if ($Case -eq 'status-failure') { 1 } else { 0 }) }
        }
        if ($Arguments[0] -eq 'inventory') {
            $lines = @('render_endpoint_count=3', 'render[0] name=Speakers (High Definition Audio Device) id=x', "render[2] name=$cableRender id={0.0.0}.{a}",
                "capture[1] name=$cableCapture id={0.0.1}.{b}")
            if ($Case -ne 'vbcable-missing') { $lines += @("render[5] name=$vbRender id={0.0.0}.{v}", "capture[3] name=$vbCapture id={0.0.1}.{w}") }
            Set-Content -LiteralPath $Stdout -Value ($lines -join "`n")
            return @{ Code = 0 }
        }
        Assert ($Arguments[0] -eq 'cable-impulse' -and $Arguments[1] -eq '400' -and $TimeoutSeconds -ge 30) 'bounded impulse probe'
        $expected = if ($Route -eq 'vbcable') { @('5', '3') } else { @('2', '1') }
        Assert ($Arguments[2] -eq $expected[0] -and $Arguments[3] -eq $expected[1]) "$Route probe uses its exact endpoint indices"
        $mode = if ($Arguments -contains 'low-latency') { 'low-latency' } else { 'default' }
        $script:probeCalls += $mode
        Set-Content -LiteralPath $Stdout -Value (Get-FakeProbe $Case $script:probeCalls.Count $mode)
        return @{ Code = $(if ($Case -eq 'probe-failure' -and $script:probeCalls.Count -eq 3) { 1 } else { 0 }) }
    }
    function Compress-Archive {
        param($LiteralPath,$DestinationPath)
        Assert ($LiteralPath -eq $evidence) 'archive contains only this run'
        Set-Content -LiteralPath $DestinationPath -Value 'fake archive'
    }
    . $execution
    Assert (Test-Path -LiteralPath (Join-Path $peer "$run.zip")) "$Case evidence copied"
    $saved = Read-PairedTraceJson (Join-Path $evidence 'result.json')
    Assert ($saved.Qualification -eq $false -and $saved.Route -eq $Route) "$Case records its route and never claims qualification"
    switch ($Case) {
        { $_ -in 'engine-pass', 'vbcable-pass', 'stall' } {
            Assert (-not $failure) "$Case completes"
            Assert (($script:probeCalls -join ',') -eq 'default,low-latency,default,low-latency,default,low-latency') 'each start measures both period modes'
            Assert ($results.Count -eq 6 -and $summary.Count -eq 2) 'six measurements and two mode summaries'
            $default = $summary | Where-Object { $_.Mode -eq 'default' }
            Assert ((@($default.SteadyLevelsMs) -join ',') -eq '26.25,28.25,30.25' -and $default.MedianSteadyMs -eq 28.25 -and $default.MaxSteadyMs -eq 30.25) 'steady level per start, median and maximum'
            Assert ($default.CleanStarts -eq 3 -and (@($default.CleanP50sMs) -join ',') -eq '26.25,28.25,30.25' -and $default.MedianCleanP50Ms -eq 28.25) 'clean starts enter the comparison median'
            Assert ($results[0].EndAnchorP50Ms -eq 26.25 -and $results[0].RenderDiscontinuities -eq 0) 'in-run dating fields recorded'
        }
        'engine-pass' {
            Assert ($script:started.Count -eq 6 -and @($script:started | Where-Object { $_ -notmatch '"--engine"' -or $_ -match '"--passthrough"' }).Count -eq 0) 'a fresh engine route for every measurement'
            Assert ($script:started[0] -match '"--frames" "480"' -and $script:started[1] -match '"--frames" "128"') 'bridge block follows the period mode'
            Assert ($results[0].Engine -match 'silent=0' -and @($results[0].Counters).Count -eq 3) 'engine quanta and driver counters recorded'
            Assert ($saved.Scope -match 'AudioRouter engine') 'engine scope recorded'
        }
        'vbcable-pass' {
            Assert ($script:started.Count -eq 0) 'VB-Cable needs no AudioRouter process'
            Assert ($saved.Scope -match 'VB-Cable' -and $saved.Scope -match 'no route') 'VB-Cable scope recorded'
        }
        'stall' {
            $low = $summary | Where-Object { $_.Mode -eq 'low-latency' }
            Assert ($low.StartsWithShift -eq 1 -and $low.LostTotal -eq 4) 'a mid-run level shift is counted, not pooled'
            Assert ($low.CleanStarts -eq 2 -and -not $results[1].Clean -and $results[1].RenderDiscontinuityMs -eq 25.0) 'a start with a render stall or loss is excluded from the comparison'
        }
        'vbcable-missing' { Assert ([bool]$failure -and [string]$failure -match 'Install VB-Cable' -and $script:probeCalls.Count -eq 0) 'missing VB-Cable starts no audio' }
        'status-failure' { Assert ([bool]$failure -and $script:started.Count -eq 0 -and $script:probeCalls.Count -eq 0) 'driver status failure starts no audio' }
        'early-exit' { Assert ([bool]$failure -and $script:probeCalls.Count -eq 0 -and [string]$failure -match 'before its leases') 'early route exit stops before the probe' }
        'probe-failure' { Assert ([bool]$failure -and $results.Count -eq 6) 'probe failure reported after every measurement' }
        'tone-hang' { Assert ([bool]$failure -and $script:killed -eq 1 -and $script:disposed -ge 1) 'hung route is killed and disposed' }
    }
}
Test-Starts 'engine-pass' 'engine'
Test-Starts 'vbcable-pass' 'vbcable'
Test-Starts 'stall' 'vbcable'
Test-Starts 'vbcable-missing' 'vbcable'
Test-Starts 'status-failure' 'engine'
Test-Starts 'early-exit' 'engine'
Test-Starts 'probe-failure' 'vbcable'
Test-Starts 'tone-hang' 'engine'

$probeBinary = Join-Path $workspace 'target\m00-probe-cable\m00-probe.exe'
$imports = @(Assert-PortableVmTool $probeBinary)
Assert (@($imports | Where-Object { $_ -match 'vcruntime|msvcp' }).Count -eq 0) 'static probe has no redistributable imports'
$selfTest = Invoke-DriverVmProcess -Executable $probeBinary -Arguments @('cable-impulse-selftest') -Stdout (Join-Path $root 'selftest.txt') -Stderr (Join-Path $root 'selftest-stderr.txt') -TimeoutSeconds 20
Assert ($selfTest.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'selftest.txt') -Raw) -match '\d+ cable impulse pairing checks pass') 'real probe pairing and steady-level self-test (offline, no endpoint)'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner,'-Route','engine') `
        -Stdout (Join-Path $root 'wrapper-guard.txt') -Stderr (Join-Path $root 'wrapper-guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'wrapper-guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks repeated-start latency orchestration checks passed. No audio stream or driver tool was opened. Evidence: $root"
