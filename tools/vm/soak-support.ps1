<#
  Pure rules for the driver soak campaign (soak.ps1 on the PC, soak-guest.ps1 in
  the VM). No process or VM operation; only the two JSON helpers touch files, so
  soak.tests.ps1 can exercise the grading before anything runs. ASCII only.
#>
function Read-SoakJson([string] $Path) {
    # The other side may be replacing the file; a miss is $null, never an error.
    for ($try = 0; $try -lt 3; $try++) {
        try {
            if (Test-Path -LiteralPath $Path -PathType Leaf) {
                $text = [IO.File]::ReadAllText($Path)
                if ($text.Trim()) { return ($text | ConvertFrom-Json) }
            }
        } catch { }
        Start-Sleep -Milliseconds 100
    }
    return $null
}
function Write-SoakJson([string] $Path, $Value) {
    $temporary = "$Path.$([Guid]::NewGuid().ToString('N')).tmp"
    [IO.File]::WriteAllText($temporary, ($Value | ConvertTo-Json -Depth 8), (New-Object Text.UTF8Encoding($false)))
    for ($try = 0; $try -lt 10; $try++) {
        try {
            if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Force -ErrorAction Stop }
            [IO.File]::Move($temporary, $Path)
            return
        } catch { Start-Sleep -Milliseconds 200 }
    }
    Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    throw "Could not write $Path."
}

# One segment's outcome. Only 'valid' adds time. Corrupted audio is excused by
# nothing; other failures only by a measured busy PC. Time is the audio really
# delivered (one impulse per 10 ms, slowest route), not wall-clock time: an
# overloaded guest ran at 70 percent of real time.
function Get-SegmentOutcome {
    param([bool] $Interrupted, $Result, [string] $HostVerdict, [int] $Minutes, [int] $Routes)
    $make = { param($outcome, $reason, $seconds, $rate)
        [pscustomobject]@{ Outcome = $outcome; Reason = $reason; ValidSeconds = [double]$seconds; RealTime = [double]$rate } }
    $rows = @()
    if ($Result) { $rows = @($Result.Results | Where-Object { $_ }) }
    # A finished report with corrupted audio counts even if a pause followed.
    if (@($rows | Where-Object { $null -ne $_.Corrupted -and [double]$_.Corrupted -gt 0 }).Count -gt 0) { return (& $make 'fail' 'corrupted audio' 0 0) }
    if ($Interrupted) { return (& $make 'invalid' 'paused' 0 0) }
    if (-not $Result) { return (& $make 'review' 'no result from the test' 0 0) }
    $starved = ($HostVerdict -eq 'Starved')
    if (@($rows | Where-Object { $_.ProbeCode -ne 0 -or $_.RouteCode -ne 0 }).Count -gt 0) {
        if ($starved) { return (& $make 'invalid' 'PC busy' 0 0) }
        return (& $make 'fail' 'a route or probe ended with an error' 0 0)
    }
    if (-not $Result.Passed -or -not $Result.Verdict -or $rows.Count -ne $Routes) { return (& $make 'review' "test stopped: $($Result.Error)" 0 0) }
    if (@($rows | Where-Object { $null -eq $_.Emitted -or $null -eq $_.Lost -or $null -eq $_.Corrupted }).Count -gt 0) { return (& $make 'review' 'incomplete counters' 0 0) }
    $least = [double](($rows | ForEach-Object { [double]$_.Emitted } | Measure-Object -Minimum).Minimum)
    $rate = [Math]::Round($least / ($Minutes * 6000.0), 3)
    if ($starved) { return (& $make 'invalid' 'PC busy' 0 $rate) }
    if ($Result.Verdict -notin @('clean', 'all-events-vm-wide', 'vm-correlated')) { return (& $make 'fail' "events not explained by a guest stall ($($Result.Verdict))" 0 $rate) }
    if ($HostVerdict -ne 'Idle') { return (& $make 'invalid' 'PC load not measured' 0 $rate) }
    if ($Result.Verdict -ne 'clean') { return (& $make 'invalid' "guest stall ($($Result.Verdict))" 0 $rate) }
    if (@($rows | Where-Object { [double]$_.Lost -gt 0 }).Count -gt 0) { return (& $make 'fail' 'lost audio in a clean verdict' 0 $rate) }
    # A stall the cable absorbed without any loss stays valid; a gap that the
    # impulse pairing did not see is not proven clean and needs a look.
    if (@($rows | Where-Object { [double]$_.CaptureDropped -gt 0 -or [double]$_.RenderSteps -gt 0 }).Count -gt 0) { return (& $make 'review' 'capture gap or render step in a clean verdict' 0 $rate) }
    if ($rate -lt 0.95) { return (& $make 'invalid' 'guest below real time' 0 $rate) }
    return (& $make 'valid' 'clean' ([Math]::Min($Minutes * 60.0, $least * 0.01)) $rate)
}

# Totals per driver image and load profile. Different drivers never add up.
function Get-SoakScore {
    param([object[]] $Records, [object[]] $Events = @(), [double] $TargetHours = 300)
    $all = @($Records | Where-Object { $_ })
    # A crash record has no route count: it belongs to every profile of its driver.
    $crashes = @($all | Where-Object { [int]$_.routes -eq 0 })
    $groups = @($all | Where-Object { [int]$_.routes -gt 0 } | Group-Object { "$($_.sysSha256)|$($_.routes)" })
    foreach ($sha in @($crashes | ForEach-Object { $_.sysSha256 } | Sort-Object -Unique)) {
        if (@($groups | Where-Object { $_.Group[0].sysSha256 -eq $sha }).Count -eq 0) {
            $groups += [pscustomobject]@{ Group = @($crashes | Where-Object { $_.sysSha256 -eq $sha } | Select-Object -First 1) }
        }
    }
    foreach ($group in $groups) {
        $first = $group.Group[0]
        $rows = @($group.Group | Where-Object { [int]$_.routes -gt 0 }) + @($crashes | Where-Object { $_.sysSha256 -eq $first.sysSha256 })
        $valid = @($rows | Where-Object { $_.outcome -eq 'valid' })
        # A pause is the owner's choice, not a measurement problem.
        $measured = @($rows | Where-Object { $_.reason -ne 'paused' })
        $invalid = @($measured | Where-Object { $_.outcome -eq 'invalid' })
        $failed = @($rows | Where-Object { $_.outcome -eq 'fail' })
        $review = @($rows | Where-Object { $_.outcome -eq 'review' })
        $incidents = @($Events | Where-Object { $_ -and $_.kind -eq 'incident' -and $_.sysSha256 -eq $first.sysSha256 })
        $hours = [double](($valid | ForEach-Object { [double]$_.validSeconds } | Measure-Object -Sum).Sum) / 3600.0
        $share = if ($measured.Count) { 100.0 * $invalid.Count / $measured.Count } else { 0.0 }
        $version = [Version]'0.0.0.0'
        [void][Version]::TryParse([string]$first.driverVersion, [ref]$version)
        $claim = if ($failed.Count + $review.Count + $incidents.Count -gt 0) { 'none while failures, reviews or incidents are open' }
            elseif ($share -gt 20) { 'inconclusive: more than 20 % of segments invalid' }
            elseif ($hours -gt 0) { [string]::Format([Globalization.CultureInfo]::InvariantCulture, 'fewer than {0:0.00} failures per 100 h (95 % confidence, VM only)', -[Math]::Log(0.05) / $hours * 100) }
            else { 'no valid time yet' }
        [pscustomobject]@{
            SysSha256 = [string]$first.sysSha256; DriverVersion = [string]$first.driverVersion; Routes = [int]$first.routes
            Rehearsal = ($version -lt [Version]'0.3.0.0'); Hours = [Math]::Round($hours, 2); TargetHours = $TargetHours
            Valid = $valid.Count; Invalid = $invalid.Count; Paused = $rows.Count - $measured.Count
            Failed = $failed.Count; Review = $review.Count; Incidents = $incidents.Count
            InvalidPercent = [Math]::Round($share, 1); Claim = $claim
            Reasons = (@($rows | Where-Object { $_.outcome -ne 'valid' } | Group-Object reason | Sort-Object Count -Descending |
                ForEach-Object { "$($_.Name) x$($_.Count)" }) -join '; ')
        }
    }
}

# The next time the clock shows HH:mm, after Now.
function Get-SoakDeadline([string] $Until, [DateTime] $Now) {
    if (-not $Until) { return $null }
    $time = [DateTime]::ParseExact($Until, 'HH:mm', [Globalization.CultureInfo]::InvariantCulture)
    $at = $Now.Date.Add($time.TimeOfDay)
    if ($at -le $Now) { $at = $at.AddDays(1) }
    return $at
}
# 'ok', 'stale' (the VM runs but the guest runner is silent) or 'down'.
# After a fresh boot the owner may still be signing in: no heartbeat yet is not a
# freeze. After a resume it is.
function Get-SoakHealth([string] $VmState, $HeartbeatAgeSeconds, [double] $SinceStartSeconds, [int] $Limit = 300, [bool] $FreshBoot = $false) {
    if ($VmState -ne 'running') { return 'down' }
    if ($FreshBoot -and $null -eq $HeartbeatAgeSeconds) { return 'ok' }
    if ($SinceStartSeconds -lt $Limit) { return 'ok' }
    if ($null -eq $HeartbeatAgeSeconds -or $HeartbeatAgeSeconds -gt $Limit) { return 'stale' }
    return 'ok'
}
