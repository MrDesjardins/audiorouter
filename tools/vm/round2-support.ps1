<#
  Pure helpers for the Stage A round-2 session (run-stage-a.ps1 -Session round2).
  Dot-sourced by the guest runner and by the host test round2-support.tests.ps1,
  so the grading rules are exercised before anything goes to the VM.
  ASCII only: Windows PowerShell 5.1 reads BOM-less files as Windows-1252.
#>

# --- tone report parsing ------------------------------------------------------
# m03_bridge_tone prints, per direction:
#   capture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 0, overrun_frames: 0, sequence_gaps: 0, ... }
#   render-source packet writes (cable-a): NativeBridgePacketCounters { accepted: 0, late: 0, overrun: 0 }
#   maximum control loop gap: 155748 us
#   maximum pump gaps: capture=153963 us render=154121 us; ...
function Get-ToneAbCounters {
    param([string] $Text)
    $field = {
        param([string] $Line, [string] $Name)
        $m = [regex]::Matches($Line, "\b${Name}: (\d+)")
        if ($m.Count -ne 1) { return $null }
        return [long]$m[0].Groups[1].Value
    }
    $capture = @($Text -split "`r?`n" | Where-Object { $_ -match '^capture-sink counters \(' })
    $render = @($Text -split "`r?`n" | Where-Object { $_ -match '^render-source counters \(' })
    $writes = @($Text -split "`r?`n" | Where-Object { $_ -match '^render-source packet writes \(' })
    $result = [ordered]@{
        Complete = $false; HasPacketWrites = $false
        RenderUnderrun = $null; RenderOverrun = $null; RenderSequenceGaps = $null
        CaptureUnderrun = $null; CaptureOverrun = $null; CaptureSequenceGaps = $null
        Accepted = $null; Late = $null; PacketOverrun = $null
        MaxGapMs = $null
    }
    if ($capture.Count -ne 1 -or $render.Count -ne 1) { return [pscustomobject]$result }
    $result.RenderUnderrun = & $field $render[0] 'underrun_frames'
    $result.RenderOverrun = & $field $render[0] 'overrun_frames'
    $result.RenderSequenceGaps = & $field $render[0] 'sequence_gaps'
    $result.CaptureUnderrun = & $field $capture[0] 'underrun_frames'
    $result.CaptureOverrun = & $field $capture[0] 'overrun_frames'
    $result.CaptureSequenceGaps = & $field $capture[0] 'sequence_gaps'
    if ($writes.Count -eq 1) {
        $result.Accepted = & $field $writes[0] 'accepted'
        $result.Late = & $field $writes[0] 'late'
        $result.PacketOverrun = & $field $writes[0] 'overrun'
        $result.HasPacketWrites = ($null -ne $result.Accepted -and $null -ne $result.Late -and $null -ne $result.PacketOverrun)
    }
    $gaps = @()
    $control = [regex]::Match($Text, 'maximum control loop gap: (\d+) us')
    if ($control.Success) { $gaps += [long]$control.Groups[1].Value }
    $pumps = [regex]::Match($Text, 'maximum pump gaps: capture=(\d+) us render=(\d+) us')
    if ($pumps.Success) { $gaps += [long]$pumps.Groups[1].Value; $gaps += [long]$pumps.Groups[2].Value }
    if ($gaps.Count -eq 3) { $result.MaxGapMs = [Math]::Round((($gaps | Measure-Object -Maximum).Maximum) / 1000.0, 1) }
    $needed = 'RenderUnderrun', 'RenderOverrun', 'RenderSequenceGaps', 'CaptureUnderrun', 'CaptureOverrun', 'CaptureSequenceGaps'
    $result.Complete = (@($needed | Where-Object { $null -eq $result[$_] }).Count -eq 0) -and ($null -ne $result.MaxGapMs)
    return [pscustomobject]$result
}

# --- the paired RC3 / 0.2.5 render comparison --------------------------------
# Rows: objects with Driver ('rc3' or 'new'), Variant ('steady' or 'restarts'),
# Valid (tone tool exit 0 and a complete report) and the counters above.
# A run is "quiet" when its largest service gap is at most QuietGapMs: a
# VirtualBox stall inflates counters on either driver and says nothing about
# the driver (2026-10-10 lesson), so only quiet runs decide.
# Pre-registered rule (2026-11-21, before any result): 0.2.5 regresses when its
# mean per-run render underrun frames, or its mean late packets, exceed RC3's by
# more than one packet (480 frames, 1 packet) AND by more than 50 percent.
# Fewer than MinQuiet quiet runs on either driver is inconclusive, not a pass.
function Compare-RenderAb {
    param([object[]] $Rows, [double] $QuietGapMs = 40, [int] $MinQuiet = 4)
    $summary = [ordered]@{ Verdict = 'REVIEW'; Detail = ''; Rc3 = $null; New = $null }
    $stats = @{}
    foreach ($driver in 'rc3', 'new') {
        $all = @($Rows | Where-Object { $_.Driver -eq $driver })
        $valid = @($all | Where-Object { $_.Valid })
        $quiet = @($valid | Where-Object { $null -ne $_.MaxGapMs -and $_.MaxGapMs -le $QuietGapMs })
        $withWrites = @($quiet | Where-Object { $_.HasPacketWrites })
        $meanOf = {
            param($Set, [string] $Name)
            if (@($Set).Count -eq 0) { return $null }
            return [Math]::Round((@($Set | ForEach-Object { [double]$_.$Name } | Measure-Object -Sum).Sum) / @($Set).Count, 2)
        }
        $stats[$driver] = [pscustomobject]@{
            Runs = $all.Count; Valid = $valid.Count; Quiet = $quiet.Count
            MeanRenderUnderrun = (& $meanOf $quiet 'RenderUnderrun')
            MeanLate = (& $meanOf $withWrites 'Late')
            MeanPacketOverrun = (& $meanOf $withWrites 'PacketOverrun')
            LateRuns = $withWrites.Count
        }
    }
    $r = $stats['rc3']; $n = $stats['new']
    $summary.Rc3 = $r; $summary.New = $n
    $shown = "RC3: $($r.Quiet)/$($r.Runs) quiet runs, mean render underrun $($r.MeanRenderUnderrun) frames, mean late $($r.MeanLate); 0.2.5: $($n.Quiet)/$($n.Runs) quiet runs, mean render underrun $($n.MeanRenderUnderrun) frames, mean late $($n.MeanLate)"
    if ($r.Quiet -lt $MinQuiet -or $n.Quiet -lt $MinQuiet) {
        $summary.Detail = "inconclusive: fewer than $MinQuiet quiet runs on a driver (stalls decide nothing); $shown"
        return [pscustomobject]$summary
    }
    $worse = {
        param($New, $Old, [double] $Margin)
        if ($null -eq $New -or $null -eq $Old) { return $false }
        return ($New -gt $Old + $Margin) -and ($New -gt $Old * 1.5)
    }
    $underrunWorse = & $worse $n.MeanRenderUnderrun $r.MeanRenderUnderrun 480
    $lateWorse = & $worse $n.MeanLate $r.MeanLate 1
    if ($underrunWorse -or $lateWorse) {
        $summary.Verdict = 'FAIL'
        $summary.Detail = "0.2.5 is worse than RC3 (rule: more than one packet and 50 percent above RC3); the frame-clock change (review item 6) is the revert candidate; $shown"
    } else {
        $summary.Verdict = 'PASS'
        $summary.Detail = "no regression visible against RC3 on quiet runs; $shown"
    }
    return [pscustomobject]$summary
}

# --- driver pool tags ---------------------------------------------------------
# NtQuerySystemInformation(SystemPoolTagInformation = 22). Entry layout on x64
# (40 bytes): Tag[4], PagedAllocs, PagedFrees, pad, PagedUsed (8), NonPagedAllocs,
# NonPagedFrees, NonPagedUsed (8); the first entry follows an 8-byte header.
# MSVC stores 'RWNM' as bytes M N W R, so the printed tags are the reverse of the
# source constants: MINWAVERT 'RWNM' -> MNWR, MINTOPORT 'RTNM' -> MNTR,
# MINADAPTER 'uAyS' -> SyAu, AUDIOROUTERVIRTUAL 'RDUA' -> AUDR.
$script:DriverPoolTags = @('MNWR', 'MNTR', 'SyAu', 'AUDR')
function Initialize-PoolTagReader {
    if ('AudioRouterPoolTags' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class AudioRouterPoolTags {
    [DllImport("ntdll.dll")]
    static extern int NtQuerySystemInformation(int cls, IntPtr buffer, int length, out int returned);
    public struct Entry { public string Tag; public ulong NonPagedUsed; public uint NonPagedAllocs; public uint NonPagedFrees; }
    public static List<Entry> Read() {
        int size = 1 << 20;
        for (int attempt = 0; attempt < 12; attempt++) {
            IntPtr buffer = Marshal.AllocHGlobal(size);
            try {
                int returned;
                int status = NtQuerySystemInformation(22, buffer, size, out returned);
                if (status == unchecked((int)0xC0000004)) { size = Math.Max(size * 2, returned + 65536); continue; }
                if (status != 0) throw new InvalidOperationException("NtQuerySystemInformation(22) failed: 0x" + status.ToString("X8"));
                int count = Marshal.ReadInt32(buffer);
                var list = new List<Entry>(count);
                for (int i = 0; i < count; i++) {
                    long at = buffer.ToInt64() + 8 + (long)i * 40;
                    var chars = new char[4];
                    for (int c = 0; c < 4; c++) chars[c] = (char)Marshal.ReadByte(new IntPtr(at + c));
                    var e = new Entry();
                    e.Tag = new string(chars);
                    e.NonPagedAllocs = (uint)Marshal.ReadInt32(new IntPtr(at + 24));
                    e.NonPagedFrees = (uint)Marshal.ReadInt32(new IntPtr(at + 28));
                    e.NonPagedUsed = (ulong)Marshal.ReadInt64(new IntPtr(at + 32));
                    list.Add(e);
                }
                return list;
            } finally { Marshal.FreeHGlobal(buffer); }
        }
        throw new InvalidOperationException("pool tag buffer kept growing");
    }
}
'@
}
# One sample: non-paged bytes in use for each driver tag, plus the whole table.
function Get-DriverPoolTagSample {
    param([string[]] $Tags = $script:DriverPoolTags)
    Initialize-PoolTagReader
    $all = [AudioRouterPoolTags]::Read()
    $row = [ordered]@{ TagsSeen = $all.Count; TotalNonPagedBytes = [double](($all | Measure-Object -Property NonPagedUsed -Sum).Sum) }
    foreach ($tag in $Tags) {
        $entry = @($all | Where-Object { $_.Tag -ceq $tag }) | Select-Object -First 1
        $row[$tag] = if ($entry) { [double]$entry.NonPagedUsed } else { 0.0 }
        $row["$tag-allocs"] = if ($entry) { [double]($entry.NonPagedAllocs - $entry.NonPagedFrees) } else { 0.0 }
    }
    return [pscustomobject]$row
}
# Growth of a series of (Minutes, Bytes) after a warm-up, MiB per hour.
function Get-SeriesGrowthMiBPerHour {
    param([object[]] $Points, [double] $WarmupMinutes = 5)
    $p = @($Points | Where-Object { $_.Minutes -ge $WarmupMinutes })
    if ($p.Count -lt 10) { return $null }
    $mx = ($p | Measure-Object -Property Minutes -Average).Average
    $my = ($p | Measure-Object -Property Bytes -Average).Average
    $sxy = 0.0; $sxx = 0.0
    foreach ($q in $p) { $sxy += ($q.Minutes - $mx) * ($q.Bytes - $my); $sxx += ($q.Minutes - $mx) * ($q.Minutes - $mx) }
    if ($sxx -eq 0) { return $null }
    return [Math]::Round(($sxy / $sxx) * 60 / 1MB, 3)
}
