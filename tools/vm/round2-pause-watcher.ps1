<#
  HOST-side, optional. Lets the guest runner (run-stage-a.ps1 -Session round2
  -WithHostPause) ask the host to pause the test VM for a while and resume it,
  which is the nearest thing to a sleep/resume that a VirtualBox guest allows
  (it is NOT a real S3 sleep and is recorded as such).

  Start it in a normal PowerShell on the PC before the guest run:

      powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\round2-pause-watcher.ps1 -Share C:\VMs\ar-share\rc-0.2.5-r2

  It only ever runs "controlvm AR-DriverTest pause" and "resume", only after the
  guest wrote round2-pause-request.txt in the share, never more than -MaxPauses
  times, and it resumes the VM even if it is stopped with Ctrl+C. It touches no
  driver, snapshot, setting or file outside the share's marker files.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $Share,
    [ValidateSet('AR-DriverTest')][string] $VmName = 'AR-DriverTest',
    [ValidateRange(30, 300)][int] $PauseSeconds = 90,
    [ValidateRange(1, 3)][int] $MaxPauses = 1,
    [ValidateRange(10, 720)][int] $TimeoutMinutes = 480
)
$ErrorActionPreference = 'Stop'
$box = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
if (-not (Test-Path -LiteralPath $box -PathType Leaf)) { throw 'VBoxManage.exe not found.' }
$share = [IO.Path]::GetFullPath($Share).TrimEnd('\')
if (-not (Test-Path -LiteralPath $share -PathType Container)) { throw "Share folder not found: $share" }
$request = Join-Path $share 'round2-pause-request.txt'
$done = Join-Path $share 'round2-pause-done.txt'
Write-Host "Watching $request for up to $TimeoutMinutes min. Ctrl+C stops it (and resumes the VM if it was paused)."
$paused = $false
$count = 0
$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
try {
    while ((Get-Date) -lt $deadline -and $count -lt $MaxPauses) {
        if (Test-Path -LiteralPath $request) {
            $running = (& $box list runningvms) -join "`n"
            if ($running -notmatch [regex]::Escape("`"$VmName`"")) { throw "$VmName is not running." }
            $began = Get-Date
            & $box controlvm $VmName pause
            if ($LASTEXITCODE -ne 0) { throw 'controlvm pause failed.' }
            $paused = $true
            Write-Host "$(Get-Date -Format 'HH:mm:ss') paused $VmName for $PauseSeconds s"
            Start-Sleep -Seconds $PauseSeconds
            & $box controlvm $VmName resume
            if ($LASTEXITCODE -ne 0) { throw 'controlvm resume failed.' }
            $paused = $false
            $count++
            Remove-Item -LiteralPath $request -Force
            @("paused=$($began.ToString('o'))", "resumed=$((Get-Date).ToString('o'))", "seconds=$PauseSeconds") |
                Set-Content -LiteralPath $done -Encoding ASCII
            Write-Host "$(Get-Date -Format 'HH:mm:ss') resumed $VmName"
        }
        Start-Sleep -Seconds 2
    }
    if ($count -eq 0) { Write-Host 'No pause was requested before the timeout.' }
} finally {
    if ($paused) { & $box controlvm $VmName resume; Write-Host 'Resumed the VM on exit.' }
}
