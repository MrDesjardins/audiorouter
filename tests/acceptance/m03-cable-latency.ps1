<# Host-safe checks of the cable latency runner with fake process boundaries,
   plus the real static probe's pairing self-test and VM guards. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $workspace 'tools\vm\portable-tool-support.ps1')
$root = Join-Path $workspace ('target\cable-latency-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$tokens = $null; $errors = $null
$runner = Join-Path $workspace 'tools\vm\run-cable-latency.ps1'
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
$null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\prepare-cable-latency-update.ps1'), [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell package preparer syntax'
$helper = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-ProbeValue' }, $true)
. ([scriptblock]::Create($helper.Extent.Text))
$configurationAst = $ast.Find({ param($node) $node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -eq '$configurations' }, $true)
$body = $ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $node.Extent.Text.Contains('Cable latency diagnostic:') }, $true)
$execution = [scriptblock]::Create($body.Extent.Text)

$renderName = 'AudioRouter Cable A Input (AudioRouter Virtual Cable)'
$captureName = 'AudioRouter Cable B Output (AudioRouter Virtual Cable)'
$otherRenderName = 'AudioRouter Cable B Input (AudioRouter Virtual Cable)'
$quietCaptureName = 'AudioRouter Cable A Output (AudioRouter Virtual Cable)'
function Get-ProbeOutput([string] $Case, [string] $Mode) {
    $p95 = if ($Case -eq 'slow' -and $Mode -eq 'low-latency') { 26.5 } elseif ($Mode -eq 'low-latency') { 9.25 } else { 23.5 }
    $lost = if ($Case -eq 'lost') { 3 } else { 0 }
    return @"
cable_mode=$Mode cable_render_buffer_frames=480
cable_impulses_emitted=1000 cable_impulses_matched=$(1000 - $lost) cable_impulses_lost=$lost cable_impulses_corrupted=0 cable_impulses_duplicate=0 cable_impulses_out_of_window=0
cable_latency_min_ms=8.000 cable_latency_p1_ms=8.500 cable_latency_p50_ms=9.000 cable_latency_p95_ms=$p95 cable_latency_max_ms=30.000 cable_latency_mean_ms=9.100 cable_latency_p99_ms=9.900 cable_jitter_p99_minus_p1_ms=1.400
"@
}

function Test-Latency([string] $Case) {
    $evidence = Join-Path $root $Case
    $bundle = Join-Path $evidence 'bundle'
    $peer = Join-Path $evidence 'peer'
    New-Item -ItemType Directory -Path $evidence,$bundle,$peer | Out-Null
    $run = $Case
    $Impulses = 1000
    $powershell = 'fake-powershell'; $probe = 'fake-probe'; $tone = 'fake-tone'
    . ([scriptblock]::Create($configurationAst.Extent.Text))
    $results = @(); $bitExact = $null; $isolation = @(); $failure = $null; $active = $null
    $script:started = @(); $script:probeCalls = @(); $script:killed = 0; $script:disposed = 0
    function Start-Sleep { param($Milliseconds) }
    function Start-Process {
        param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru,$RedirectStandardOutput,$RedirectStandardError)
        Assert ($FilePath -eq $tone -and $WindowStyle -eq 'Hidden') 'only the owned pass-through tool is started, hidden'
        $script:started += [string]$ArgumentList
        $ready = $Case -ne 'early-exit'
        Set-Content -LiteralPath $RedirectStandardOutput -Value $(if ($ready) { "driver: info`nrender worker polling before lease activation`ncapture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 0 }`nrender-source counters (cable-a): NativeBridgeStreamCounters { underrun_frames: 0 }`nrender-source packet writes (cable-a): NativeBridgePacketCounters { accepted: 1200, late: 0, overrun: 0 }`npass-through relay: forwarded=1500 silence=40 dropped=0" } else { 'driver: info' })
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
            $lines = @('render_endpoint_count=3', "render[0] name=Speakers (High Definition Audio Device) id=x", "render[2] name=$renderName id={0.0.0}.{a}",
                'capture_endpoint_count=2', "capture[1] name=$captureName id={0.0.1}.{b}",
                "render[4] name=$otherRenderName id={0.0.0}.{d}", "capture[0] name=$quietCaptureName id={0.0.1}.{e}")
            if ($Case -eq 'duplicate') { $lines += "render[3] name=$renderName id={0.0.0}.{c}" }
            Set-Content -LiteralPath $Stdout -Value ($lines -join "`n")
            return @{ Code = 0 }
        }
        if ($Arguments[0] -eq 'cable-isolation') {
            Assert ($Arguments[1] -eq '5' -and $Arguments[2] -in @('2','4') -and $Arguments[3] -eq '0' -and $Arguments[4] -eq '1' -and $TimeoutSeconds -ge 20) 'isolation records Cable A Output (and drains Cable B Output) while noise plays into Cable A or B Input'
            $script:probeCalls += "isolation-$($Arguments[2])"
            $leak = $Case -eq 'leak' -and $Arguments[2] -eq '4'
            $idleB = $Case -eq 'idle-b'
            Set-Content -LiteralPath $Stdout -Value ("isolation_rendered_frames=240000 isolation_capture_overflow_packets=0
isolation_drain_frames=240000 isolation_drain_active_frames=$(if ($idleB) { 0 } else { 239000 })
isolation_frames=240000 isolation_nonzero_samples=$(if ($leak) { 7 } else { 0 }) isolation_peak=$(if ($leak) { 0.001 } else { 0 }) isolation_peak_dbfs=$(if ($leak) { -60 } else { -1000 })
isolation_pass=$(if ($leak) { 0 } else { 1 })")
            return @{ Code = 0 }
        }
        if ($Arguments[0] -eq 'cable-bitexact') {
            Assert ($Arguments[1] -eq '10' -and $Arguments[2] -eq '2' -and $Arguments[3] -eq '1' -and $TimeoutSeconds -ge 30) 'bit-exact probe on the exact endpoints with a watchdog'
            $script:probeCalls += 'bitexact'
            $exact = $Case -ne 'not-exact'
            Set-Content -LiteralPath $Stdout -Value ("bitexact_aligned=1 bitexact_align_frame=4410 bitexact_noise_frames=480000 bitexact_compared_frames=480000 bitexact_mismatched_samples=$(if ($exact) { 0 } else { 12 }) bitexact_first_mismatch_frame=0 bitexact_max_abs_diff=$(if ($exact) { 0 } else { 3e-8 })
bitexact_silence_frames_expected=24000 bitexact_silence_frames_checked=24000 bitexact_silence_nonzero_samples=0
bitexact_pass=$(if ($exact) { 1 } else { 0 })")
            return @{ Code = 0 }
        }
        Assert ($Arguments[0] -eq 'cable-impulse' -and $Arguments[1] -eq '1000' -and $Arguments[2] -eq '2' -and $Arguments[3] -eq '1') 'probe uses the exact Cable A Input / Cable B Output indices'
        Assert ($TimeoutSeconds -ge 30) 'bounded probe watchdog'
        $mode = if ($Arguments -contains 'low-latency') { 'low-latency' } else { 'default' }
        $script:probeCalls += $mode
        Set-Content -LiteralPath $Stdout -Value (Get-ProbeOutput $Case $mode)
        return @{ Code = $(if ($Case -eq 'probe-failure') { 1 } else { 0 }) }
    }
    function Compress-Archive {
        param($LiteralPath,$DestinationPath)
        Assert ($LiteralPath -eq $evidence) 'archive contains only this run'
        Set-Content -LiteralPath $DestinationPath -Value 'fake archive'
    }
    . $execution
    Assert (Test-Path -LiteralPath (Join-Path $peer "$run.zip")) "$Case evidence copied"
    $saved = Read-PairedTraceJson (Join-Path $evidence 'result.json')
    Assert ($saved.Qualification -eq $false) "$Case never claims qualification"
    switch ($Case) {
        { $_ -in 'pass','slow','lost','not-exact','leak','idle-b' } {
            Assert (-not $failure) "$Case completes"
            Assert ($script:started.Count -eq 4 -and $script:started[3] -notmatch '"--passthrough"' -and $script:started[0] -match '"--frames" "480"' -and $script:started[1] -match '"--frames" "128"' -and $script:started[2] -match '"--frames" "480"') 'latency at 480 and 128 frames, then bit-exactness at 480'
            Assert (@($script:started[0..2] | Where-Object { $_ -notmatch '"--passthrough"' }).Count -eq 0) 'pass-through for latency and bit-exactness; tone mode for isolation'
            $seconds = [int]([regex]::Match($script:started[0], '"--seconds" "(\d+)"').Groups[1].Value)
            Assert ($seconds -ge 18) 'pass-through outlives the probe'
            Assert (($script:probeCalls -join ',') -eq 'default,low-latency,bitexact,isolation-2,isolation-4') 'latency, bit-exact, then both isolation paths'
            Assert (@($saved.Isolation).Count -eq 2) 'isolation results recorded'
            Assert ($saved.BitExact.Passed -eq ($Case -ne 'not-exact')) 'bit-exact verdict recorded from the probe'
            Assert ($bitExact.Relay -match 'forwarded=') 'bit-exact relay statistics recorded'
            Assert ($results.Count -eq 2 -and $results[0].Relay -match 'forwarded=1500' -and @($results[0].Counters).Count -eq 3) 'relay statistics and driver counters recorded'
            Assert ($results[0].P95Ms -eq 23.5 -and $results[0].JitterMs -eq 1.4) 'probe values parsed'
        }
        'pass' { Assert ($results[0].MeetsTargets -and $results[1].MeetsTargets -and @($isolation | Where-Object { $_.Passed }).Count -eq 2) 'targets met and both isolation paths silent' }
        'idle-b' { Assert (-not $isolation[0].Passed -and -not $isolation[1].Passed -and $isolation[0].NonzeroSamples -eq 0) 'silence with an idle Cable B is inconclusive, not a pass' }
        'leak' { Assert ($isolation[0].Passed -and -not $isolation[1].Passed -and $isolation[1].NonzeroSamples -eq 7) 'a leak into Cable A Output is reported, not hidden' }
        'not-exact' { Assert (-not $bitExact.Passed -and $bitExact.MismatchedSamples -eq 12) 'a mismatch is reported, not hidden' }
        'slow' { Assert ($results[0].MeetsTargets -and -not $results[1].MeetsTargets) 'a missed low-latency target is reported, not hidden' }
        'lost' { Assert (-not $results[0].MeetsTargets -and -not $results[1].MeetsTargets) 'lost impulses fail the targets' }
        { $_ -in 'status-failure','duplicate' } {
            Assert ([bool]$failure -and $script:started.Count -eq 0 -and $script:probeCalls.Count -eq 0) "$Case starts no audio"
        }
        'early-exit' { Assert ([bool]$failure -and $script:probeCalls.Count -eq 0 -and [string]$failure -match 'before its leases') 'early pass-through exit stops before the probe' }
        'probe-failure' { Assert ([bool]$failure -and $results.Count -eq 2 -and $null -ne $bitExact) 'probe failure reported after every measurement' }
        'tone-hang' { Assert ([bool]$failure -and $script:killed -eq 1 -and $script:disposed -ge 1) 'hung pass-through is killed and disposed' }
    }
}
foreach ($case in @('pass','slow','lost','not-exact','leak','idle-b','status-failure','duplicate','early-exit','probe-failure','tone-hang')) { Test-Latency $case }

$probeBinary = Join-Path $workspace 'target\m00-probe-cable\m00-probe.exe'
$imports = @(Assert-PortableVmTool $probeBinary)
Assert (@($imports | Where-Object { $_ -match 'vcruntime|msvcp' }).Count -eq 0) 'static probe has no redistributable imports'
$selfTest = Invoke-DriverVmProcess -Executable $probeBinary -Arguments @('cable-impulse-selftest') -Stdout (Join-Path $root 'selftest.txt') -Stderr (Join-Path $root 'selftest-stderr.txt') -TimeoutSeconds 20
Assert ($selfTest.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'selftest.txt') -Raw) -match '\d+ cable impulse pairing checks pass') 'real probe pairing self-test (offline, no endpoint)'
$selfTest = Invoke-DriverVmProcess -Executable $probeBinary -Arguments @('cable-bitexact-selftest') -Stdout (Join-Path $root 'bitexact-selftest.txt') -Stderr (Join-Path $root 'bitexact-selftest-stderr.txt') -TimeoutSeconds 20
Assert ($selfTest.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'bitexact-selftest.txt') -Raw) -match '\d+ cable bit-exact checks pass') 'real probe bit-exact self-test (offline, no endpoint)'
$selfTest = Invoke-DriverVmProcess -Executable $probeBinary -Arguments @('cable-isolation-selftest') -Stdout (Join-Path $root 'isolation-selftest.txt') -Stderr (Join-Path $root 'isolation-selftest-stderr.txt') -TimeoutSeconds 20
Assert ($selfTest.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'isolation-selftest.txt') -Raw) -match '\d+ cable isolation checks pass') 'real probe isolation self-test (offline, no endpoint)'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'wrapper-guard.txt') -Stderr (Join-Path $root 'wrapper-guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'wrapper-guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks cable-latency orchestration checks passed. No audio stream or driver tool was opened. Evidence: $root"
