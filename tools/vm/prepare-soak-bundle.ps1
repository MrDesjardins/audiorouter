<#
  HOST-only: no build, driver load, VM start or audio. Copies a verified base
  bundle (same driver and tools, bytes unchanged), adds the soak scripts and
  writes a new manifest. Run once, or again after a script changes (new folder).

  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-soak-bundle.ps1 -AllowDirty
#>
[CmdletBinding()]
param(
    [string] $BaseBundle = 'C:\VMs\ar-share\rc-0.2.5-r7',
    [string] $Destination = 'C:\VMs\ar-share\rc-0.2.5-soak',
    # Rehearsal only: the manifest then says the scripts were not committed.
    [switch] $AllowDirty,
    # Cables the guest enables (cables.txt): 8 for 0.2.5, 16 for the 16-cable driver.
    [ValidateRange(1, 16)] [int] $Cables = 8
)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
$base = [IO.Path]::GetFullPath($BaseBundle).TrimEnd('\')
$output = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if ($base -ieq $output -or $output.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Use separate base and destination folders.' }
if ((Test-Path -LiteralPath $output) -and @(Get-ChildItem -LiteralPath $output -Force).Count -gt 0) { throw 'Destination must be new or empty.' }
$scripts = 'soak-guest.ps1', 'soak-support.ps1', 'run-cable-continuity.ps1', 'round2-support.ps1', 'paired-trace-support.ps1', 'quiet-guest.ps1', 'vm-checks.ps1'
$dirty = @(& git -C $workspace status --porcelain -- ($scripts | ForEach-Object { "tools/vm/$_" }))
if ($dirty.Count -gt 0 -and -not $AllowDirty) { throw 'The soak scripts are not committed. Commit them, or use -AllowDirty for a rehearsal.' }
$commit = (& git -C $workspace rev-parse HEAD).Trim()
Assert-PairedTraceBundle $base
New-Item -ItemType Directory -Path $output -Force | Out-Null
foreach ($entry in @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })) {
    $relative = $entry.Substring(66)
    # Only the driver under test belongs in this bundle.
    if ($relative -like 'driver-next\*' -or $relative -like 'driver-rc3\*') { continue }
    $file = [IO.Path]::GetFullPath((Join-Path $output $relative))
    if (-not $file.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $file
}
foreach ($script in $scripts) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot $script) -Destination (Join-Path $output $script) -Force }
[IO.File]::WriteAllText((Join-Path $output 'cables.txt'), "$Cables")
$sys = (Get-FileHash -LiteralPath (Join-Path $output 'driver\audioroutervirtual.sys') -Algorithm SHA256).Hash
$package = Get-Content -LiteralPath (Join-Path $output 'driver\package.json') -Raw | ConvertFrom-Json
if ($package.signed -ne 'test' -or $package.timingDiagnostics -or $package.dirty) { throw 'The driver is not a clean, test-signed build without timing diagnostics.' }
$manifest = @('AudioRouter driver soak; guest-only measurement', "source: $commit$(if ($dirty.Count) { ' plus uncommitted soak scripts' })",
    "base: $base", "driver: $($package.driverVersion) SYS $sys", "cables: $Cables", '')
foreach ($file in Get-ChildItem -LiteralPath $output -File -Recurse | Where-Object { $_.Name -ne 'MANIFEST.txt' } | Sort-Object FullName) {
    $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(), $file.FullName.Substring($output.Length + 1)
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $output
Write-Host "Soak bundle ready: $output (driver $($package.driverVersion), $($sys.Substring(0, 12)))."
