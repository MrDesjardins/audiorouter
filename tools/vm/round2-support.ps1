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
        DriverVersion = $null
    }
    # "driver: NativeBridgeDriverInfo { ..., driver_version: [0, 2, 5, 0], ... }": what the loaded driver reports.
    $version = [regex]::Match($Text, '(?m)^\W*driver: NativeBridgeDriverInfo \{[^}]*driver_version: \[(\d+), (\d+), (\d+), (\d+)\]')
    if ($version.Success) { $result.DriverVersion = ($version.Groups[1..4] | ForEach-Object { $_.Value }) -join '.' }
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
# Valid (tone tool exit 0, a complete report, and the loaded driver reported the
# version the phase expected), and the counters above.
#
# Rule, pre-registered 2026-11-21 before any round-2 result and recalibrated the
# same day on the existing tone reports (only 1 of 9 earlier runs had a largest
# service gap under 40 ms; 61, 78, 154, 248 and 332 ms were typical, so a "quiet
# run" filter would have discarded the night). The VM stalls hit both drivers,
# because the phases alternate, so the decision uses MEDIANS over all valid runs
# of each render shape, which a minority of stalled runs cannot move:
#   * a shape is judged only with at least MinValid valid runs on both drivers;
#   * 0.2.5 regresses in a shape when its median render underrun frames, or its
#     median late packets, exceed RC3's by more than one packet (480 frames, 1
#     packet) AND by more than 50 percent;
#   * a shape whose median largest service gap exceeds MaxMedianGapMs on either
#     driver is stall-dominated and inconclusive;
#   * FAIL if any shape regresses; PASS only if both shapes were judged and none
#     regressed; otherwise REVIEW (inconclusive), never PASS.
function Get-Median([double[]] $Values) {
    $v = @($Values | Sort-Object)
    if ($v.Count -eq 0) { return $null }
    $mid = [int][Math]::Floor($v.Count / 2)
    if ($v.Count % 2 -eq 1) { return [double]$v[$mid] }
    return ([double]$v[$mid - 1] + [double]$v[$mid]) / 2.0
}
function Compare-RenderAb {
    param([object[]] $Rows, [int] $MinValid = 4, [double] $MaxMedianGapMs = 500)
    $worse = {
        param($New, $Old, [double] $Margin)
        if ($null -eq $New -or $null -eq $Old) { return $false }
        return ($New -gt $Old + $Margin) -and ($New -gt $Old * 1.5)
    }
    $shapes = [ordered]@{}
    foreach ($variant in 'steady', 'restarts') {
        $info = [ordered]@{ Judged = $false; Regressed = $false; Note = ''; Rc3 = $null; New = $null }
        $set = @{}
        foreach ($driver in 'rc3', 'new') {
            $valid = @($Rows | Where-Object { $_.Driver -eq $driver -and $_.Variant -eq $variant -and $_.Valid })
            $withWrites = @($valid | Where-Object { $_.HasPacketWrites })
            $set[$driver] = [pscustomobject]@{
                Valid = $valid.Count
                MedianUnderrun = Get-Median @($valid | ForEach-Object { [double]$_.RenderUnderrun })
                MedianLate = if ($withWrites.Count -ge $MinValid) { Get-Median @($withWrites | ForEach-Object { [double]$_.Late }) } else { $null }
                MedianGapMs = Get-Median @($valid | Where-Object { $null -ne $_.MaxGapMs } | ForEach-Object { [double]$_.MaxGapMs })
            }
        }
        $r = $set['rc3']; $n = $set['new']
        $info.Rc3 = $r; $info.New = $n
        $text = "${variant}: RC3 $($r.Valid) valid runs (median underrun $($r.MedianUnderrun), late $($r.MedianLate), gap $($r.MedianGapMs) ms); 0.2.5 $($n.Valid) valid runs (median underrun $($n.MedianUnderrun), late $($n.MedianLate), gap $($n.MedianGapMs) ms)"
        if ($r.Valid -lt $MinValid -or $n.Valid -lt $MinValid) {
            $info.Note = "inconclusive, fewer than $MinValid valid runs on a driver; $text"
        } elseif (($r.MedianGapMs -gt $MaxMedianGapMs) -or ($n.MedianGapMs -gt $MaxMedianGapMs)) {
            $info.Note = "inconclusive, stall-dominated (median largest gap above $MaxMedianGapMs ms); $text"
        } else {
            $info.Judged = $true
            $info.Regressed = (& $worse $n.MedianUnderrun $r.MedianUnderrun 480) -or (& $worse $n.MedianLate $r.MedianLate 1)
            $lateNote = if ($null -eq $r.MedianLate -or $null -eq $n.MedianLate) { ' (late packets not compared: too few runs with packet counters)' } else { '' }
            $info.Note = "$(if ($info.Regressed) { 'REGRESSED' } else { 'no regression' })$lateNote; $text"
        }
        $shapes[$variant] = [pscustomobject]$info
    }
    $verdict = 'REVIEW'
    if (@($shapes.Values | Where-Object { $_.Regressed }).Count -gt 0) { $verdict = 'FAIL' }
    elseif (@($shapes.Values | Where-Object { $_.Judged }).Count -eq 2) { $verdict = 'PASS' }
    $detail = (@($shapes.Keys | ForEach-Object { $shapes[$_].Note }) -join ' | ')
    if ($verdict -eq 'FAIL') { $detail = "0.2.5 is worse than RC3 (more than one packet and 50 percent above RC3's median); the frame-clock change (review item 6) is the revert candidate. $detail" }
    return [pscustomobject]@{ Verdict = $verdict; Detail = $detail; Shapes = [pscustomobject]$shapes }
}

# --- guest auto-logon (reversible; the guest is a throwaway VM) -----------------
# A planned restart resumes through a logon task, so an unattended night needs the
# guest to sign in by itself. The password is stored in the Winlogon key only for
# the run and removed at the end; restoring the checkpoint removes it as well.
$script:WinlogonKey = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
function Set-GuestAutoLogon {
    param([string] $UserName, [string] $Domain, [Security.SecureString] $Password, [string] $KeyPath = $script:WinlogonKey)
    $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR([Runtime.InteropServices.Marshal]::SecureStringToBSTR($Password))
    if (-not (Test-Path -LiteralPath $KeyPath)) { New-Item -Path $KeyPath -Force | Out-Null }
    $before = [ordered]@{}
    foreach ($name in 'AutoAdminLogon', 'DefaultUserName', 'DefaultDomainName', 'DefaultPassword', 'AutoLogonCount') {
        $item = Get-ItemProperty -LiteralPath $KeyPath -Name $name -ErrorAction SilentlyContinue
        $before[$name] = if ($item) { [string]$item.$name } else { $null }
    }
    Set-ItemProperty -LiteralPath $KeyPath -Name AutoAdminLogon -Value '1' -Type String
    Set-ItemProperty -LiteralPath $KeyPath -Name DefaultUserName -Value $UserName -Type String
    Set-ItemProperty -LiteralPath $KeyPath -Name DefaultDomainName -Value $Domain -Type String
    Set-ItemProperty -LiteralPath $KeyPath -Name DefaultPassword -Value $plain -Type String
    Remove-ItemProperty -LiteralPath $KeyPath -Name AutoLogonCount -ErrorAction SilentlyContinue
    return [pscustomobject]$before
}
function Clear-GuestAutoLogon {
    param([object] $Before, [string] $KeyPath = $script:WinlogonKey)
    Remove-ItemProperty -LiteralPath $KeyPath -Name DefaultPassword -ErrorAction SilentlyContinue
    foreach ($name in 'AutoAdminLogon', 'DefaultUserName', 'DefaultDomainName') {
        $value = if ($Before) { $Before.$name } else { $null }
        if ($null -ne $value) { Set-ItemProperty -LiteralPath $KeyPath -Name $name -Value $value -Type String }
        elseif ($name -eq 'AutoAdminLogon') { Set-ItemProperty -LiteralPath $KeyPath -Name $name -Value '0' -Type String }
        else { Remove-ItemProperty -LiteralPath $KeyPath -Name $name -ErrorAction SilentlyContinue }
    }
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
