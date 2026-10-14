<# Guest-only sustained continuity with eight cables (VCAB-24 step 3, VM deviation run).
   Enables eight cables (restored afterwards), runs four AudioRouter engine routes at once
   (Cable A->B, C->D, E->F, G->H) and one identity-coded impulse train per route for -Minutes.
   Every lost impulse and every driver counter increase is checked against VM-wide stalls:
   seconds in which all four route processes saw a scheduling gap. Under VirtualBox NEM this
   is a documented deviation run, never VCAB-24 qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 60)][int] $Minutes = 60)
$ErrorActionPreference = 'Stop'

# A progress line from m03_bridge_tone: tool-relative time, the largest worker
# or control gap of that interval, and the error counter total (driver stream
# counters plus late/overrun packet writes).
function ConvertFrom-ProgressLine([string] $Line) {
    $head = [regex]::Match($Line, '^progress (\d+) ms: .*?interval_capture_gap_us=(\d+) interval_render_gap_us=(\d+) interval_control_gap_us=(\d+)')
    if (-not $head.Success) { return $null }
    $total = 0
    foreach ($counter in [regex]::Matches($Line, '\b(underrun_frames|overrun_frames|sequence_gaps|non_finite_samples|format_mismatches|late|overrun): (\d+)')) {
        $total += [long]$counter.Groups[2].Value
    }
    $gapUs = [Math]::Max([Math]::Max([long]$head.Groups[2].Value, [long]$head.Groups[3].Value), [long]$head.Groups[4].Value)
    $qpc = [regex]::Match($Line, '\bsnapshot_qpc_100ns=(\d+)')
    return [pscustomobject]@{ Ms = [long]$head.Groups[1].Value; MaxGapMs = $gapUs / 1000.0; Errors = $total
        QpcSeconds = $(if ($qpc.Success) { [double]$qpc.Groups[1].Value / 1.0e7 } else { $null }) }
}

# Intervals (absolute seconds) of one route process: start, end, largest gap,
# and the error counters added during the interval.
function Get-ProgressIntervals([string[]] $Lines, [double] $StartSeconds) {
    $intervals = @()
    $previous = $null
    foreach ($line in $Lines) {
        $entry = ConvertFrom-ProgressLine $line
        if (-not $entry) { continue }
        $from = if ($previous) { $previous.Ms } else { 0 }
        $added = if ($previous) { $entry.Errors - $previous.Errors } else { $entry.Errors }
        $hasQpc = $null -ne $entry.QpcSeconds
        if ($previous -and ($null -ne $previous.QpcSeconds) -ne $hasQpc) { throw 'Mixed QPC/nominal progress clocks.' }
        $to = if ($hasQpc) { $entry.QpcSeconds } else { $StartSeconds + $entry.Ms / 1000.0 }
        $begin = if ($hasQpc) {
            if ($previous) { $previous.QpcSeconds } else { $to - $entry.Ms / 1000.0 }
        } else { $StartSeconds + $from / 1000.0 }
        if ($to -lt $begin -or $added -lt 0) { throw 'Progress clock/counters regressed.' }
        $intervals += [pscustomobject]@{ From = $begin; To = $to
            MaxGapMs = $entry.MaxGapMs; Added = $added; Timing = $(if ($hasQpc) { 'qpc' } else { 'nominal' }) }
        $previous = $entry
    }
    return $intervals
}

# VM-wide: every route process had a gap of at least StallMs within
# ToleranceSeconds of the event. Four independent processes stalling together
# establishes four-route gap correlation. It does not prove the cause of a loss.
function Test-VmWideStall([double] $At, [object[]] $IntervalsPerRoute, [double] $StallMs = 20.0, [double] $ToleranceSeconds = 1.5) {
    foreach ($intervals in $IntervalsPerRoute) {
        $hit = @($intervals | Where-Object { $_.MaxGapMs -ge $StallMs -and $_.From -le ($At + $ToleranceSeconds) -and $_.To -ge ($At - $ToleranceSeconds) })
        if ($hit.Count -eq 0) { return $false }
    }
    return $true
}

# Gives every event a Cause once all routes are collected:
# - vm-wide: Test-VmWideStall (every route process stalled at that moment);
# - cross-route: every other route has an event within CrossSeconds, so the
#   cause is shared by four independent routes (r9: counter increases on all
#   four within 40 ms while no process showed a 20 ms gap);
# - relay-trim: after a stall starves the pass-through relay it inserts
#   silence, then trims one block to win the delay back, losing one impulse
#   (r9: per route the unexplained single losses equalled `trimmed`). Single
#   losses count as trims only up to the relay's trimmed count, and only when
#   trims do not exceed silence insertions; a counter increase within
#   TrimSeconds of such a loss on the same route belongs to that trim;
# - unexplained: anything else.
# Returns the counts per cause.
function Resolve-EventCauses([object[]] $Results, [double] $CrossSeconds = 1.0, [double] $TrimSeconds = 1.5) {
    foreach ($route in $Results) {
        foreach ($item in @($route.Events)) {
            $cause = if ($item.VmWide) { 'vm-wide' } else { 'unexplained' }
            $item | Add-Member NoteProperty Cause $cause -Force
        }
    }
    foreach ($route in $Results) {
        $others = @($Results | Where-Object { $_ -ne $route })
        if ($others.Count -eq 0) { continue }
        foreach ($item in @($route.Events | Where-Object { $_.Cause -eq 'unexplained' })) {
            $shared = $true
            foreach ($other in $others) {
                if (@($other.Events | Where-Object { [Math]::Abs($_.At - $item.At) -le $CrossSeconds }).Count -eq 0) { $shared = $false; break }
            }
            if ($shared) { $item.Cause = 'cross-route' }
        }
    }
    foreach ($route in $Results) {
        $relay = [string]$route.Relay
        $trimmed = [regex]::Match($relay, '\btrimmed=(\d+)')
        $silence = [regex]::Match($relay, '\bsilence=(\d+)')
        if (-not $trimmed.Success -or -not $silence.Success) { continue }
        $trims = [long]$trimmed.Groups[1].Value
        if ($trims -gt [long]$silence.Groups[1].Value) { continue }
        $singles = @($route.Events | Where-Object { $_.Cause -eq 'unexplained' -and $_.Kind -eq 'lost-impulses' -and $_.Size -eq 1 })
        if ($singles.Count -eq 0 -or $singles.Count -gt $trims) { continue }
        foreach ($item in $singles) { $item.Cause = 'relay-trim' }
        foreach ($item in @($route.Events | Where-Object { $_.Cause -eq 'unexplained' -and $_.Kind -eq 'driver-counters' })) {
            $near = @($singles | Where-Object { [Math]::Abs($_.At - $item.At) -le $TrimSeconds }).Count
            if ($near -gt 0) { $item.Cause = 'relay-trim' }
        }
    }
    $counts = [ordered]@{ 'vm-wide' = 0; 'cross-route' = 0; 'relay-trim' = 0; unexplained = 0 }
    foreach ($route in $Results) { foreach ($item in @($route.Events)) { $counts[$item.Cause]++ } }
    return [pscustomobject]$counts
}

function Get-ContinuityVerdict($Causes) {
    $total = $Causes.'vm-wide' + $Causes.'cross-route' + $Causes.'relay-trim' + $Causes.unexplained
    if ($total -eq 0) { return 'clean' }
    if ($Causes.unexplained -gt 0) { return 'unexplained-events' }
    if ($Causes.'cross-route' + $Causes.'relay-trim' -eq 0) { return 'all-events-vm-wide' }
    return 'vm-correlated'
}

function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}

if ($MyInvocation.InvocationName -eq '.') { return }

$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Requires 2 GB free on guest C:.' }
if (Get-Process -Name 'm03_bridge_tone','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$run = 'continuity-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$tone = Join-Path $bundle 'tools\m03_bridge_tone.exe'
$helper = Join-Path $bundle 'tools\audiorouter-driver-helper.exe'
$pairs = @(
    @{ Name = 'A-to-B'; Render = 'A'; Capture = 'B' }, @{ Name = 'C-to-D'; Render = 'C'; Capture = 'D' },
    @{ Name = 'E-to-F'; Render = 'E'; Capture = 'F' }, @{ Name = 'G-to-H'; Render = 'G'; Capture = 'H' }
)
$routes = @{}
$probes = @{}
$results = @()
$failure = $null
$cablesBefore = $null
$restored = $null
$verdict = $null
$causes = $null
function Invoke-Helper([string[]] $Arguments, [string] $Name) {
    $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
    return Invoke-DriverVmProcess -Executable $helper -Arguments ($Arguments + @('--result', (Join-Path $evidence "$Name.json"))) `
        -Stdout (Join-Path $evidence "$Name.txt") -Stderr (Join-Path $evidence "$Name-stderr.txt") -TimeoutSeconds 240
}
function Get-Inventory([string] $Name) {
    $result = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence "$Name.txt") `
        -Stderr (Join-Path $evidence "$Name-stderr.txt") -TimeoutSeconds 20
    if ($result.Code -ne 0) { throw "Probe inventory failed (exit $($result.Code))." }
    return Get-Content -LiteralPath (Join-Path $evidence "$Name.txt") -Raw
}
function Get-EndpointIndex([string] $Inventory, [string] $Direction, [string] $Name) {
    $found = [regex]::Matches($Inventory, "(?m)^$Direction\[(\d+)\] name=" + [regex]::Escape($Name) + ' id=')
    if ($found.Count -ne 1) { return $null }
    return $found[0].Groups[1].Value
}
function Get-CableCount([string] $Inventory) {
    return @('A','B','C','D','E','F','G','H' | Where-Object {
        $null -ne (Get-EndpointIndex $Inventory 'render' "AudioRouter Cable $_ Input (AudioRouter Virtual Cable)") }).Count
}
function Get-Seconds { return [DateTime]::UtcNow.Ticks / 1.0e7 }
try {
    Write-Host "Cable continuity: 8 cables, 4 engine routes, $Minutes minute(s) of impulses per route. No manual playback needed."
    $inventory = Get-Inventory 'inventory-before'
    $cablesBefore = Get-CableCount $inventory
    if ($cablesBefore -lt 1) { throw 'No AudioRouter cable is active; install the driver first. No audio test started.' }
    if ($cablesBefore -ne 8) {
        Write-Host "Enabling 8 cables (was $cablesBefore; restored at the end)..."
        $set = Invoke-Helper @('set-cables', '--count', '8') 'set-cables-8'
        if ($set.Code -ne 0) { throw "set-cables 8 failed (exit $($set.Code)); see set-cables-8.txt. No audio test started." }
        Start-Sleep -Seconds 3
        $inventory = Get-Inventory 'inventory-8'
    }
    if ((Get-CableCount $inventory) -ne 8) { throw 'Eight active cables expected after set-cables 8; no audio test started.' }
    foreach ($pair in $pairs) {
        $pair.RenderIndex = Get-EndpointIndex $inventory 'render' "AudioRouter Cable $($pair.Render) Input (AudioRouter Virtual Cable)"
        $pair.CaptureIndex = Get-EndpointIndex $inventory 'capture' "AudioRouter Cable $($pair.Capture) Output (AudioRouter Virtual Cable)"
        if ($null -eq $pair.RenderIndex -or $null -eq $pair.CaptureIndex) { throw "Endpoints for $($pair.Name) not found exactly once; no audio test started." }
    }
    $impulses = $Minutes * 6000
    $toneSeconds = $Minutes * 60 + 40
    foreach ($pair in $pairs) {
        $out = Join-Path $evidence "$($pair.Name)-tone.txt"
        $arguments = @('--engine','--discard','--frames','480','--seconds',[string]$toneSeconds,
            '--render-bus',"cable-$($pair.Render.ToLower())",'--capture-bus',"cable-$($pair.Capture.ToLower())",
            '--out',(Join-Path $evidence "$($pair.Name).wav")) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }
        $routes[$pair.Name] = Start-Process -FilePath $tone -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput $out -RedirectStandardError (Join-Path $evidence "$($pair.Name)-tone-stderr.txt")
        $null = $routes[$pair.Name].Handle
        $watch = [Diagnostics.Stopwatch]::StartNew()
        while (-not ((Test-Path -LiteralPath $out) -and ((Get-Content -LiteralPath $out -Raw) -match 'render worker polling before lease activation'))) {
            if ($routes[$pair.Name].HasExited) { throw "$($pair.Name) route exited before its leases were active (exit $($routes[$pair.Name].ExitCode))." }
            if ($watch.Elapsed.TotalSeconds -ge 15) { throw "$($pair.Name) route readiness timed out." }
            Start-Sleep -Milliseconds 50
        }
        # The tool prints this line right after starting its progress clock.
        $pair.ToneStart = Get-Seconds
    }
    Start-Sleep -Milliseconds 1500
    foreach ($pair in $pairs) {
        if ($routes[$pair.Name].HasExited) { throw "$($pair.Name) route ended before the impulses started (exit $($routes[$pair.Name].ExitCode))." }
        $arguments = @('cable-impulse',[string]$impulses,$pair.RenderIndex,$pair.CaptureIndex,('raw=' + (Join-Path $evidence "$($pair.Name)-raw.csv")))
        $pair.ProbeStart = Get-Seconds
        $probes[$pair.Name] = Start-Process -FilePath $probe -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru `
            -RedirectStandardOutput (Join-Path $evidence "$($pair.Name)-probe.txt") -RedirectStandardError (Join-Path $evidence "$($pair.Name)-probe-stderr.txt")
        $null = $probes[$pair.Name].Handle
    }
    Write-Host "All four routes and impulse trains running; results in about $Minutes minute(s)."
    $deadline = [Diagnostics.Stopwatch]::StartNew()
    foreach ($pair in $pairs) {
        $remaining = [Math]::Max(1000, ($Minutes * 60 + 120) * 1000 - [int]$deadline.ElapsedMilliseconds)
        if (-not $probes[$pair.Name].WaitForExit($remaining)) { throw "$($pair.Name) impulse train did not finish within its watchdog." }
        $pair.ProbeEnd = Get-Seconds
    }
    foreach ($pair in $pairs) {
        if (-not $routes[$pair.Name].WaitForExit(120000)) { throw "$($pair.Name) route did not finish within its watchdog." }
    }
    $intervalsPerRoute = @()
    foreach ($pair in $pairs) {
        $progress = Join-Path $evidence "$($pair.Name).progress.txt"
        $lines = if (Test-Path -LiteralPath $progress) { @(Get-Content -LiteralPath $progress) } else { @() }
        $pair.Intervals = @(Get-ProgressIntervals $lines $pair.ToneStart)
        $intervalsPerRoute += ,$pair.Intervals
    }
    foreach ($pair in $pairs) {
        $probeText = Get-Content -LiteralPath (Join-Path $evidence "$($pair.Name)-probe.txt") -Raw
        $toneText = Get-Content -LiteralPath (Join-Path $evidence "$($pair.Name)-tone.txt") -Raw
        $events = @()
        $qpcDating = @($pair.Intervals | Where-Object { $_.Timing -eq 'qpc' }).Count -gt 0
        $windowStart = $pair.ProbeStart + 1.0
        $windowEnd = $pair.ProbeEnd - 1.0
        foreach ($lostRun in [regex]::Matches($probeText, '(?m)^cable_lost_run first_impulse=(\d+) count=(\d+)(?: first_qpc_100ns=([0-9.]+) last_qpc_100ns=([0-9.]+))?')) {
            if ($qpcDating -ne $lostRun.Groups[3].Success) { throw 'Probe and route timing bases differ; cannot attribute losses.' }
            $at = if ($qpcDating) { [double]$lostRun.Groups[3].Value / 1.0e7 } else { $pair.ProbeStart + 0.15 + [double]$lostRun.Groups[1].Value * 0.01 }
            $events += [pscustomobject]@{ Kind = 'lost-impulses'; At = $at; Size = [int]$lostRun.Groups[2].Value }
        }
        if ($qpcDating) {
            $anchor = Get-ProbeValue $probeText 'cable_emission_first_qpc_100ns'
            $end = Get-ProbeValue $probeText 'cable_emission_last_qpc_100ns'
            if ($null -eq $anchor -or $null -eq $end) { throw 'QPC emission window missing.' }
            $windowStart = $anchor / 1.0e7 + 1.0
            $windowEnd = $end / 1.0e7 - 1.0
        }
        $lostTotal = Get-ProbeValue $probeText 'cable_impulses_lost'
        $reportedLosses = [long](($events | Measure-Object -Property Size -Sum).Sum)
        if ($reportedLosses -ne $lostTotal) {
            $runsTotal = Get-ProbeValue $probeText 'cable_lost_runs'
            throw "Lost-run report is incomplete ($($events.Count) of $runsTotal runs listed, $reportedLosses of $lostTotal impulses); cannot attribute every loss."
        }
        # Driver counter increases inside the measurement window (the route's
        # start-up and its idle tail after the impulses stop are excluded).
        foreach ($interval in $pair.Intervals | Where-Object { $_.Added -gt 0 -and $_.From -ge $windowStart -and $_.To -le $windowEnd }) {
            $events += [pscustomobject]@{ Kind = 'driver-counters'; At = ($interval.From + $interval.To) / 2; Size = $interval.Added }
        }
        foreach ($item in $events) {
            $item | Add-Member NoteProperty VmWide (Test-VmWideStall $item.At $intervalsPerRoute)
        }
        $results += [pscustomobject][ordered]@{
            Pair = $pair.Name; ProbeCode = $probes[$pair.Name].ExitCode; RouteCode = $routes[$pair.Name].ExitCode; Timing = $(if ($qpcDating) { 'render-qpc-estimate' } else { 'nominal' })
            Emitted = Get-ProbeValue $probeText 'cable_impulses_emitted'; Lost = Get-ProbeValue $probeText 'cable_impulses_lost'
            Corrupted = Get-ProbeValue $probeText 'cable_impulses_corrupted'; LostRuns = Get-ProbeValue $probeText 'cable_lost_runs'
            CaptureDropped = Get-ProbeValue $probeText 'cable_capture_dropped_frames'; RenderSteps = Get-ProbeValue $probeText 'cable_render_discontinuities'
            P50Ms = Get-ProbeValue $probeText 'cable_latency_p50_ms'; Segments = Get-ProbeValue $probeText 'cable_segments'
            Engine = ([regex]::Match($toneText, '(?m)^engine route quanta: .*$')).Value
            Relay = ([regex]::Match($toneText, '(?m)^pass-through relay: .*$')).Value
            Counters = @([regex]::Matches($toneText, '(?m)^(capture-sink|render-source) (counters|packet writes) \(.*$') | ForEach-Object { $_.Value.TrimEnd() })
            StallIntervals = @($pair.Intervals | Where-Object { $_.MaxGapMs -ge 20 }).Count
            Events = $events
        }
        Write-Host ("{0}: lost {1} of {2} impulses in {3} run(s), corrupted {4}, capture dropped {5}, render steps {6}, p50 {7} ms; events {8} (VM-wide {9}); probe exit {10}, route exit {11}" -f `
            $pair.Name, $results[-1].Lost, $results[-1].Emitted, $results[-1].LostRuns, $results[-1].Corrupted, $results[-1].CaptureDropped,
            $results[-1].RenderSteps, $results[-1].P50Ms, $events.Count, @($events | Where-Object { $_.VmWide }).Count, $results[-1].ProbeCode, $results[-1].RouteCode)
        if ($results[-1].Engine) { Write-Host "  $($results[-1].Engine)" }
        $results[-1].Counters | ForEach-Object { Write-Host "  $_" }
    }
    $causes = Resolve-EventCauses $results
    $verdict = Get-ContinuityVerdict $causes
    Write-Host ("Event causes: VM-wide {0}, cross-route {1}, relay trim {2}, unexplained {3}" -f $causes.'vm-wide', $causes.'cross-route', $causes.'relay-trim', $causes.unexplained)
    if (@($results | Where-Object { $_.ProbeCode -ne 0 -or $_.RouteCode -ne 0 -or $_.Corrupted -gt 0 }).Count -gt 0) {
        throw 'A probe or route failed, or impulses arrived corrupted; see the reports.'
    }
} catch { $failure = $_ } finally {
    foreach ($process in @($probes.Values) + @($routes.Values)) {
        try {
            if ($process -and -not $process.HasExited) {
                if (-not $process.WaitForExit(30000)) { $process.Kill(); [void]$process.WaitForExit(5000) }
            }
        } catch { }
    }
    if ($null -ne $cablesBefore -and $cablesBefore -ge 1 -and $cablesBefore -ne 8) {
        $restore = Invoke-Helper @('set-cables', '--count', [string]$cablesBefore) "set-cables-$cablesBefore"
        $restored = ($restore.Code -eq 0)
        Write-Host ("Cable count restored to {0}: {1}" -f $cablesBefore, $(if ($restored) { 'yes' } else { "NO (exit $($restore.Code)); run the helper set-cables again" }))
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Minutes = $Minutes; CablesBefore = $cablesBefore; CableCountRestored = $restored; Verdict = $verdict; Causes = $causes; Results = $results; Qualification = $false
        Scope = 'Eight cables, four AudioRouter engine routes (A->B, C->D, E->F, G->H), identity-coded impulses every 10 ms; NEM VM deviation run, not VCAB-24 qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
switch ($verdict) {
    'clean' { Write-Host "No lost impulse and no driver counter increase in $Minutes minute(s) with 8 cables (VM run; indicative for VCAB-24)." }
    'all-events-vm-wide' { Write-Host 'Every loss and counter increase coincided with a VM-wide stall (documented VCAB-24 deviation).' }
    'vm-correlated' { Write-Host 'Every loss and counter increase was VM-wide, shared by all four routes at once, or a relay trim recovering delay after a stall (documented VCAB-24 deviation).' }
    default { Write-Host 'Some losses did NOT coincide with a VM-wide stall; send the output for review.' }
}
Write-Host 'Send the output.'
