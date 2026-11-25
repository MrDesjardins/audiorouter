<#
  Host-only sampler for VirtualBox sessions. One CSV row every IntervalSeconds
  so a guest freeze can be matched to host load (owner rule, 2026-11-22: a game
  on the host froze the VM for minutes with no driver installed).

  Columns: utc,cpu_pct,cpu_other_pct,vm_cpu_pct,avail_mb,pages_sec. cpu_pct is
  total host CPU; vm_cpu_pct is the VirtualBox processes' share of it (measured
  from process CPU time over the interval); cpu_other_pct is the remainder, the
  load the verdict judges, so the guest's own vCPUs do not count as starvation.

  Start it before the VM (round2-host-preflight.ps1 does this for you) and leave
  the window open. Default output is the share root, so the guest reads the same
  rows as Z:\host-load.csv. On a live run the file is truncated once at start.

  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\host-load.ps1
  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\host-load.ps1 -Minutes 1560 -IntervalSeconds 20
#>
[CmdletBinding()]
param(
    [string] $OutputPath = 'C:\VMs\ar-share\host-load.csv',
    [ValidateRange(1, 2880)][int] $Minutes = 480,
    [ValidateRange(5, 300)][int] $IntervalSeconds = 20,
    [switch] $Truncate,
    [switch] $Once
)
$ErrorActionPreference = 'Continue'
# The measurement lives with the round-2 helpers, so the preflight and the
# sampler cannot drift apart.
. (Join-Path $PSScriptRoot 'round2-support.ps1')
function Format-HostLoadRow($Sample) {
    # Integers only, so the CSV never depends on a decimal comma locale.
    return '{0},{1},{2},{3},{4},{5}' -f $Sample.Utc.ToString('yyyy-MM-ddTHH:mm:ssZ'),
        [int][Math]::Round($Sample.CpuPercent), [int][Math]::Round($Sample.OtherCpuPercent),
        [int][Math]::Round($Sample.VmCpuPercent), [int][Math]::Round($Sample.AvailableMb),
        [int][Math]::Round($Sample.PagesPerSec)
}
$baseline = Get-HostCpuBaseline
if ($Once) {
    Start-Sleep -Seconds 1
    Format-HostLoadRow (Get-HostLoadSnapshot -Baseline $baseline) | Write-Output
    exit 0
}
$folder = Split-Path -Parent $OutputPath
if ($folder -and -not (Test-Path -LiteralPath $folder)) { New-Item -ItemType Directory -Path $folder -Force | Out-Null }
$header = 'utc,cpu_pct,cpu_other_pct,vm_cpu_pct,avail_mb,pages_sec'
if ($Truncate -or -not (Test-Path -LiteralPath $OutputPath)) {
    Set-Content -LiteralPath $OutputPath -Value $header -Encoding ASCII
}
Start-Sleep -Seconds 1
$end = (Get-Date).AddMinutes($Minutes)
while ((Get-Date) -lt $end) {
    $sample = Get-HostLoadSnapshot -Baseline $baseline
    $baseline = $sample.Baseline
    if ($sample.CpuPercent -ge 0 -and $sample.AvailableMb -ge 0) {
        try { Add-Content -LiteralPath $OutputPath -Value (Format-HostLoadRow $sample) -Encoding ASCII -ErrorAction Stop } catch { }
    }
    Start-Sleep -Seconds $IntervalSeconds
}