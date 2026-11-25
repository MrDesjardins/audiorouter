<#
  Host-only sampler for VirtualBox sessions. One CSV row every IntervalSeconds
  so a guest freeze can be matched to host load (owner rule, 2026-11-22: a game
  on the host froze the VM for minutes with no driver installed).

  Start it before the VM (round2-host-preflight.ps1 does this for you) and leave
  the window open. Default output is the share root, so the guest reads the same
  rows as Z:\host-load.csv. On a live run the file is truncated once at start.

  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\host-load.ps1
  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\host-load.ps1 -Minutes 60 -IntervalSeconds 10
#>
[CmdletBinding()]
param(
    [string] $OutputPath = 'C:\VMs\ar-share\host-load.csv',
    [ValidateRange(1, 1440)][int] $Minutes = 480,
    [ValidateRange(5, 300)][int] $IntervalSeconds = 20,
    [switch] $Truncate,
    [switch] $Once
)
$ErrorActionPreference = 'Continue'
function Get-HostLoadSample {
    $cpu = 0
    $processors = @(Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue)
    if ($processors.Count -gt 0) {
        $cpu = [int][Math]::Round((($processors | Measure-Object -Property LoadPercentage -Average).Average), 0)
    }
    $availableMb = -1
    $pages = -1
    $memory = Get-CimInstance Win32_PerfRawData_PerfOS_Memory -ErrorAction SilentlyContinue
    if ($memory) {
        $availableMb = [int]$memory.AvailableMBytes
    } else {
        $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
        if ($os) { $availableMb = [int]($os.FreePhysicalMemory / 1024) }
    }
    $formatted = Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory -ErrorAction SilentlyContinue
    if ($formatted -and $null -ne $formatted.PagesPerSec) { $pages = [int]$formatted.PagesPerSec }
    return [pscustomobject]@{
        Utc = [DateTime]::UtcNow
        CpuPercent = $cpu
        AvailableMb = $availableMb
        PagesPerSec = $pages
    }
}
function Format-HostLoadRow($Sample) {
    return '{0},{1},{2},{3}' -f $Sample.Utc.ToString('yyyy-MM-ddTHH:mm:ssZ'), $Sample.CpuPercent, $Sample.AvailableMb, $Sample.PagesPerSec
}
if ($Once) {
    Format-HostLoadRow (Get-HostLoadSample) | Write-Output
    exit 0
}
$folder = Split-Path -Parent $OutputPath
if ($folder -and -not (Test-Path -LiteralPath $folder)) { New-Item -ItemType Directory -Path $folder -Force | Out-Null }
$header = 'utc,cpu_pct,avail_mb,pages_sec'
if ($Truncate -or -not (Test-Path -LiteralPath $OutputPath)) {
    Set-Content -LiteralPath $OutputPath -Value $header -Encoding ASCII
}
$end = (Get-Date).AddMinutes($Minutes)
while ((Get-Date) -lt $end) {
    $sample = Get-HostLoadSample
    if ($sample.CpuPercent -ge 0 -and $sample.AvailableMb -ge 0) {
        try { Add-Content -LiteralPath $OutputPath -Value (Format-HostLoadRow $sample) -Encoding ASCII -ErrorAction Stop } catch { }
    }
    Start-Sleep -Seconds $IntervalSeconds
}