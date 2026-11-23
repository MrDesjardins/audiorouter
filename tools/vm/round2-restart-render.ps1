<#
  Guest-only helper for run-stage-a.ps1 -Session round2. Plays several short
  tones into Cable A Input, one stream after another, the way repeated clicks on
  the Windows "Test" button do (the 2026-11-11 traced run lost about one packet
  of render audio per stream start). It is started in the background while the
  bridge tone tool holds the cable; its own output is only kept for the record.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $Probe,
    [Parameter(Mandatory = $true)][ValidatePattern('^\d+$')][string] $RenderIndex,
    [ValidateRange(0, 60000)][int] $DelayMs = 4000,
    [ValidateRange(1, 20)][int] $Count = 4,
    [ValidateRange(500, 20000)][int] $ToneMs = 4000,
    [ValidateRange(0, 10000)][int] $GapMs = 1500
)
$ErrorActionPreference = 'Continue'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') { Write-Host 'Run only inside AR-DriverTest.'; exit 2 }
if (-not (Test-Path -LiteralPath $Probe -PathType Leaf)) { Write-Host "Probe not found: $Probe"; exit 2 }
Start-Sleep -Milliseconds $DelayMs
$codes = @()
for ($i = 1; $i -le $Count; $i++) {
    & $Probe tone ([string]$ToneMs) $RenderIndex | Out-Host
    $codes += $LASTEXITCODE
    Start-Sleep -Milliseconds $GapMs
}
Write-Host "restart-render exit codes: $($codes -join ',')"
if (@($codes | Where-Object { $_ -ne 0 }).Count -gt 0) { exit 1 }
exit 0
