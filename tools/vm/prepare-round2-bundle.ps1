<#
  HOST-only, no build, no driver load, no audio. Assembles the Stage A round-2
  bundle from the 0.2.5 bundle that already ran on 2026-11-12:

    * every file the base manifest lists, unchanged (the same 0.2.5 driver,
      driver-next 0.2.6 and tools),
    * run-stage-a.ps1 and vm-checks.ps1 from this commit (they gained the round2
      session and -PackageName),
    * the round-2 helpers (round2-support.ps1, round2-restart-render.ps1),
    * driver-rc3\ : the RC3 0.2.4 test-signed package, for the paired comparison,
    * a new MANIFEST.txt with SHA-256 of every file.

  Refuses to run unless the git tree is clean, the 0.2.5 package still has the
  recorded identity and the RC3 SYS still has the recorded hash.

  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-round2-bundle.ps1
#>
[CmdletBinding()]
param(
    [string] $BaseBundle = 'C:\VMs\ar-share\rc-0.2.5',
    [string] $Rc3Package = 'C:\VMs\ar-share\rc-0.2.4-base\driver',
    [string] $Destination = 'C:\VMs\ar-share\rc-0.2.5-r2'
)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
$base = [IO.Path]::GetFullPath($BaseBundle).TrimEnd('\')
$output = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
$rc3 = [IO.Path]::GetFullPath($Rc3Package).TrimEnd('\')
if ($base -ieq $output -or $output.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase) -or $base.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Use separate base and destination folders.' }
if ((Test-Path -LiteralPath $output) -and @(Get-ChildItem -LiteralPath $output -Force).Count -gt 0) { throw 'Destination must be new or empty.' }
if (@(& git -C $workspace status --porcelain)) { throw 'Package only from a clean, committed source tree.' }
$commit = (& git -C $workspace rev-parse HEAD).Trim()
Assert-PairedTraceBundle $base

# Identity of the two drivers (recorded 2026-11-21 in the gap analysis).
$expected025 = 'D2C26DD92C0038FDCF661159FE333F962C9C3C5FB0942A5ED825A5649EBC234A'
$expectedRc3 = 'AF3B4E5EDEA301220EC0BE0CF1675E1723874A3464BE3ADF517E80D61B1F39B1'   # RC3 0.2.4 SYS SHA-256
$sys025 = (Get-FileHash -LiteralPath (Join-Path $base 'driver\audioroutervirtual.sys') -Algorithm SHA256).Hash
if ($sys025 -ine $expected025) { throw "The base driver is not the frozen 0.2.5 SYS ($sys025)." }
$sysRc3 = (Get-FileHash -LiteralPath (Join-Path $rc3 'audioroutervirtual.sys') -Algorithm SHA256).Hash
if ($sysRc3 -ine $expectedRc3) { throw "The RC3 package is not the frozen 0.2.4 SYS ($sysRc3)." }
$rc3Json = Get-Content -LiteralPath (Join-Path $rc3 'package.json') -Raw | ConvertFrom-Json
if ($rc3Json.driverVersion -ne '0.2.4.0' -or $rc3Json.signed -ne 'test' -or $rc3Json.timingDiagnostics) { throw 'The RC3 package is not a normal test-signed 0.2.4.0 build.' }
$certBase = (Get-FileHash -LiteralPath (Join-Path $base 'driver\AudioRouterTest.cer') -Algorithm SHA256).Hash
$certRc3 = (Get-FileHash -LiteralPath (Join-Path $rc3 'AudioRouterTest.cer') -Algorithm SHA256).Hash
if ($certBase -ne $certRc3) { throw 'RC3 and 0.2.5 use different test certificates; the guest would not trust both.' }

New-Item -ItemType Directory -Path $output | Out-Null
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    $file = [IO.Path]::GetFullPath((Join-Path $output $relative))
    if (-not $file.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $file
}
# Scripts that changed in this commit, and the new helpers.
foreach ($name in 'run-stage-a.ps1', 'vm-checks.ps1', 'round2-support.ps1', 'round2-restart-render.ps1') {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $output $name) -Force
}
Copy-Item -LiteralPath $rc3 -Destination (Join-Path $output 'driver-rc3') -Recurse
# The host watcher is not a guest file; it stays in the repository.

$manifest = @('AudioRouter Stage A round 2 (0.2.5 gap analysis, 2026-11-21); guest-only measurement',
    "source: $commit", "base: $base",
    "driver: 0.2.5.0 SYS $sys025", "driver-rc3: 0.2.4.0 SYS $sysRc3",
    "prepared: $([DateTime]::UtcNow.ToString('u'))", '')
foreach ($file in Get-ChildItem -LiteralPath $output -File -Recurse | Where-Object { $_.Name -ne 'MANIFEST.txt' } | Sort-Object FullName) {
    $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(), $file.FullName.Substring($output.Length + 1)
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $output
$count = @(Get-ChildItem -LiteralPath $output -File -Recurse).Count
Write-Host "Round-2 bundle ready: $output ($count files, source $commit)."
