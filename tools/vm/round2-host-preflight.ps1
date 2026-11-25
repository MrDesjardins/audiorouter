<#
  HOST-only preflight for a VirtualBox Stage A round-2 night. No build, no
  driver, no audio, no VM start. It refuses to prepare the run on a busy PC
  (owner rule, 2026-11-22: a game on the host froze the guest for minutes with
  no driver installed, so every result that night was meaningless) and starts
  the host-load sampler the guest needs.

  It measures CPU and available memory for about 10 s, checks for a running
  WSL distro, prints the heaviest processes, starts tools\vm\host-load.ps1 if
  no fresh sampler file exists, and prints the exact guest steps. Exit 0 when
  the PC is ready, 2 when it is not.

  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\round2-host-preflight.ps1
#>
[CmdletBinding()]
param(
    [string] $OutputPath = 'C:\VMs\ar-share\host-load.csv',
    [double] $MaxCpuPercent = 20,
    [int] $MinAvailableMb = 6000,
    [ValidateRange(5, 120)][int] $MeasureSeconds = 10,
    [ValidateRange(1, 2880)][int] $SamplerMinutes = 1560,
    [switch] $NoSampler,
    [switch] $LibraryOnly
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'round2-support.ps1')

# Samples for MeasureSeconds, using the sampler's own measurement so the gate and
# the CSV agree. Returns a Get-HostLoadVerdict-shaped summary of the window.
function Measure-HostIdle {
    param([int] $Seconds = 10, [int] $IntervalSeconds = 1)
    $samples = New-Object System.Collections.Generic.List[object]
    $baseline = Get-HostCpuBaseline
    Start-Sleep -Milliseconds 500
    $until = (Get-Date).AddSeconds($Seconds)
    do {
        $sample = Get-HostLoadSnapshot -Baseline $baseline
        $baseline = $sample.Baseline
        if ($sample.CpuPercent -ge 0 -and $sample.AvailableMb -ge 0) { $samples.Add($sample) }
        if ((Get-Date) -lt $until) { Start-Sleep -Seconds $IntervalSeconds }
    } while ((Get-Date) -lt $until)
    return (Get-HostLoadWindow -Rows $samples.ToArray())
}
# Names that are worth naming when the PC is busy; the measurement decides, this
# is only the explanation.
function Get-HeavyProcesses {
    param([string[]] $Names = @('RainbowSix', 'RainbowSix_DX11', 'vmmem', 'vmmemWSL', 'VirtualBoxVM', 'VBoxHeadless', 'Memory Compression', 'Discord', 'obs64', 'dwm'))
    return @(Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $Names -contains $_.ProcessName } |
        Sort-Object -Property WorkingSet64 -Descending |
        Select-Object -First 6 |
        ForEach-Object { '{0} ({1:N0} MB)' -f $_.ProcessName, ($_.WorkingSet64 / 1MB) })
}
function Get-RunningWsl {
    # wsl.exe writes UTF-16 to a pipe and PowerShell 5.1 then shows every
    # character separated by a space, so the WSL2 host process is the signal.
    return @(Get-Process -Name 'vmmem', 'vmmemWSL' -ErrorAction SilentlyContinue |
        ForEach-Object { $_.ProcessName } | Sort-Object -Unique)
}
# Everything that makes this PC unsuitable for a VM night, in one list.
function Get-PreflightReasons {
    param($Verdict, [string[]] $Wsl)
    $reasons = @()
    if ($Verdict.Verdict -eq 'Starved') { $reasons += $Verdict.Detail }
    if ($Verdict.Verdict -eq 'NoData') { $reasons += $Verdict.Detail }
    if (@($Wsl).Count -gt 0) { $reasons += "WSL is running ($(@($Wsl) -join ', ')); stop it first (owner action): wsl --shutdown" }
    return @($reasons)
}
# A sampler is alive when the CSV gained a row recently (host clock only).
function Test-SamplerRunning {
    param([string] $Path, [int] $MaxAgeSeconds = 90)
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    try { $age = ((Get-Date) - (Get-Item -LiteralPath $Path).LastWriteTime).TotalSeconds } catch { return $false }
    return $age -lt $MaxAgeSeconds
}
function Start-Sampler {
    param([string] $Path, [int] $Minutes)
    $script = Join-Path $PSScriptRoot 'host-load.ps1'
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $script, '-OutputPath', $Path, '-Minutes', [string]$Minutes)
    $process = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -ArgumentList $arguments -WindowStyle Hidden -PassThru
    return $process.Id
}

if ($LibraryOnly) { return }

Write-Host "Stage A round-2 host preflight: measuring ${MeasureSeconds}s ..." -ForegroundColor Cyan
$summary = Measure-HostIdle -Seconds $MeasureSeconds
$verdict = Get-HostLoadVerdict $summary -MaxCpuPercent $MaxCpuPercent -MinAvailableMb $MinAvailableMb
$wsl = @(Get-RunningWsl)
$reasons = @(Get-PreflightReasons -Verdict $verdict -Wsl $wsl)
$heavy = @(Get-HeavyProcesses)
if ($heavy.Count -gt 0) { Write-Host "Heaviest processes: $($heavy -join ', ')" }

if ($reasons.Count -gt 0) {
    Write-Host "`nNOT READY: this PC is too busy for a VM night." -ForegroundColor Red
    foreach ($reason in $reasons) { Write-Host "  - $reason" }
    Write-Host "Close games and other heavy apps, run 'wsl --shutdown', wait a minute, then run this command again:"
    Write-Host '  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\round2-host-preflight.ps1'
    Write-Host "Nothing was started. A guest freeze on a busy host proves nothing about the driver."
    exit 2
}

$samplerPid = $null
if ($NoSampler) {
    Write-Host "`nReady (sampler not started: -NoSampler)." -ForegroundColor Green
} elseif (Test-SamplerRunning -Path $OutputPath) {
    Write-Host "`nReady; the host-load sampler is already writing $OutputPath." -ForegroundColor Green
} else {
    $samplerPid = Start-Sampler -Path $OutputPath -Minutes $SamplerMinutes
    Write-Host "`nReady. Host-load sampler started (pid $samplerPid) writing $OutputPath." -ForegroundColor Green
}
Write-Host @"

Next, on the PC (do not run these from this script):
1. Keep this PC awake and leave the game and WSL closed for the whole run.
2. Start AR-DriverTest from snapshot 02-test-signing-ready (untick 'create snapshot').
3. In the guest: copy Z:\<bundle> to C:\ar\<bundle>, open PowerShell as administrator, then:
     cd C:\ar\<bundle>
     powershell -NoProfile -ExecutionPolicy Bypass -File .\run-stage-a.ps1 -Session round2 -Blocks repro -FuzzSeconds 300 -ReproBoots 5 -AutoLogon
   The guest reads the sampler through Z:\host-load.csv and refuses to start the
   blocks when no sampler is writing, so a silent night cannot be mistaken for a
   driver result.
4. Record the sampler pid ($samplerPid) and the bundle name in the run notes.
"@
exit 0