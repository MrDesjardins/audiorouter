<# Run from a freshly staged shared folder inside AR-DriverTest. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$share = $PSScriptRoot
$support = Join-Path $share 'repo\tests\acceptance\m03-driver-vm-support.ps1'
. $support
Assert-DriverTestVmIdentity $env:COMPUTERNAME (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run in Administrator PowerShell inside the VM.'
}
$localRoot = 'C:\ar'
if ([IO.Path]::GetFullPath($share).TrimEnd('\') -ieq $localRoot) {
    throw 'Run retry-smoke.ps1 from the new Z: shared package, not C:\ar.'
}
& robocopy.exe $share $localRoot /E /R:1 /W:1
if ($LASTEXITCODE -ge 8) { throw "Copy failed (robocopy exit $LASTEXITCODE)." }
# Verify tools and scripts as well as the signed driver. Snapshot restoration
# can otherwise silently put an older helper next to a new driver package.
$manifestEntries = @(Get-Content -LiteralPath (Join-Path $share 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
if ($manifestEntries.Count -eq 0) { throw 'Package manifest contains no file hashes.' }
foreach ($entry in $manifestEntries) {
    $expectedHash = $entry.Substring(0, 64)
    $relative = $entry.Substring(66)
    $path = [IO.Path]::GetFullPath((Join-Path $localRoot $relative))
    if (-not $path.StartsWith($localRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Manifest path is outside C:\ar.'
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $expectedHash) {
        throw "Copied package hash mismatch: $relative"
    }
    Unblock-File -LiteralPath $path
}
Write-Host "Verified $($manifestEntries.Count) copied package files." -ForegroundColor Green
Get-Content -LiteralPath (Join-Path $localRoot 'driver\package.json') -Raw | Write-Host
Set-Location -LiteralPath $localRoot
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight
if ($LASTEXITCODE -ne 0) { throw 'Preflight failed; smoke was not started.' }
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step smoke
$smokeExit = $LASTEXITCODE
# Smoke performs its own cleanup. Preserve its failure while collecting logs.
$collectionStart = Get-Date
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step collect
if ($LASTEXITCODE -ne 0) { throw 'Evidence collection failed; retain C:\ar\evidence.' }
$zip = Get-ChildItem -LiteralPath $localRoot -Filter 'evidence-*.zip' -File |
    Where-Object { $_.LastWriteTime -ge $collectionStart } |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $zip) { throw 'No new evidence ZIP was produced.' }
Copy-Item -LiteralPath $zip.FullName -Destination (Split-Path -Parent $share)
Write-Host "Evidence copied to shared folder: $($zip.Name)" -ForegroundColor Green
if ($smokeExit -ne 0) { throw 'Smoke failed; evidence was collected and copied. Stop here.' }
Write-Host 'Smoke passed. Restore the clean snapshot before the second run.' -ForegroundColor Green
