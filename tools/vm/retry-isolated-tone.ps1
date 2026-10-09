# Run inside the test VM from the shared diagnostic bundle.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = 'C:\ar'
$bundle = $PSScriptRoot
$metadata = Get-Content -LiteralPath (Join-Path $bundle 'diagnostics.json') -Raw | ConvertFrom-Json
$expectedPaths = @('tools\m03_bridge_tone.exe', 'vm-checks.ps1')
if (@($metadata.files).Count -ne 2 -or @($metadata.files.path | Sort-Object -Unique).Count -ne 2) {
    throw 'Unexpected diagnostic manifest.'
}
if ((Get-FileHash -LiteralPath "$root\driver\audioroutervirtual.sys").Hash -ine $metadata.driverSha256) {
    throw 'This diagnostic requires the installed capture-tick-primed candidate. Send this error before continuing.'
}
foreach ($entry in $metadata.files) {
    if ($entry.path -cnotin $expectedPaths) { throw 'Unexpected diagnostic file path.' }
    $source = Join-Path $bundle $entry.path
    $destination = Join-Path $root $entry.path
    if ((Get-FileHash -LiteralPath $source).Hash -ine $entry.sha256) { throw 'Shared diagnostic hash mismatch.' }
    Copy-Item -LiteralPath $source -Destination $destination -Force
    if ((Get-FileHash -LiteralPath $destination).Hash -ine $entry.sha256) { throw 'Copied diagnostic hash mismatch.' }
    Unblock-File -LiteralPath $destination
}
Write-Host "Verified isolated harness from $($metadata.gitCommit)."
powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step status
if ($LASTEXITCODE -ne 0) { throw 'Installed-driver status failed. Send the output before continuing.' }
Write-Host '30-second check: keep Cable B Output Listen enabled; click Cable A Input Test several times now.'
powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step tone -ToneSeconds 30
$toneExit = $LASTEXITCODE
$collectStart = Get-Date
powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step collect
if ($LASTEXITCODE -ne 0) { throw 'Collection failed. Send the output.' }
$zip = Get-ChildItem -LiteralPath $root -Filter 'evidence-*.zip' -File |
    Where-Object { $_.LastWriteTime -ge $collectStart.AddSeconds(-2) } |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $zip) { throw 'No new evidence archive found.' }
Copy-Item -LiteralPath $zip.FullName -Destination (Join-Path (Split-Path $bundle -Parent) $zip.Name) -Force
Write-Host "Evidence copied to shared folder: $($zip.Name)"
if ($toneExit -ne 0) { throw 'Short tone failed; evidence was copied. Stop here and send the output.' }
Write-Host 'Short tone passed. Send the output for review before any long or stall run.'
