<#
.SYNOPSIS
  Build everything the AudioRouter cable test VM needs into one folder.

.DESCRIPTION
  Host-only and build-only: nothing here installs, loads or trusts a driver
  on this PC, and nothing changes boot, Secure Boot, test-signing or audio
  settings. The output folder is what you copy into the VM (see
  docs/operations/virtual-cable-vm-guide.md):

    <Share>\driver\   test-signed driver package (+ AudioRouterTest.cer, public part only)
    <Share>\tools\    audiorouter-driver-helper.exe (debug, accepts test packages only
                      with AUDIOROUTER_ALLOW_TEST_DRIVER=1), m03_bridge_tone.exe,
                      m03_cable_inventory.exe, m03-bridge-fuzz.exe
    <Share>\repo\     the VM scripts with the folder layout they expect
    <Share>\MANIFEST.txt  versions, git commit and SHA-256 of every file

  The Rust tools are linked with a static C runtime so the VM needs no
  Visual C++ redistributable.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\prepare-vm-share.ps1 -Share D:\ar-share
#>
[CmdletBinding()]
param(
    [string] $Share = 'D:\ar-share',
    [string] $Version = '0.1.0'
)
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$driverRoot = Join-Path $workspace 'drivers\audiorouter-virtual'
. (Join-Path $driverRoot 'package-tools.ps1')

$Share = Assert-DriverPackagePath $Share
if (Test-Path -LiteralPath $Share) {
    if (@(Get-ChildItem -LiteralPath $Share -Force).Count -ne 0) {
        throw "The share folder must be new or empty so no older file is mistaken for this build: $Share"
    }
} else {
    New-Item -ItemType Directory -Path $Share | Out-Null
}
$tools = Join-Path $Share 'tools'
$repo = Join-Path $Share 'repo'
New-Item -ItemType Directory -Path $tools, $repo | Out-Null

function Step([string] $Text) { Write-Host "`n== $Text" -ForegroundColor Cyan }

Step "1/6 Test-signed driver package (version $Version)"
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $driverRoot 'build.ps1') `
    -Configuration Release -Platform x64 -Version $Version -TestSign -KeepOutput -Output (Join-Path $Share 'driver')
if ($LASTEXITCODE -ne 0) { throw "Driver build/test-sign failed ($LASTEXITCODE)." }

Step '2/6 Package integrity checks'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'tests\acceptance\m03-driver-package.ps1') `
    -Package (Join-Path $Share 'driver')
if ($LASTEXITCODE -ne 0) { throw "Package verification failed ($LASTEXITCODE)." }

Step '3/6 VM tools (static C runtime, separate target folder)'
$vmTarget = Join-Path $workspace 'target\vm-tools'
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    # Debug helper on purpose: only a debug build accepts test-signed packages (17 §9.1).
    & cargo build --locked --target-dir $vmTarget -p audiorouter-driver-helper
    if ($LASTEXITCODE -ne 0) { throw 'Helper build failed.' }
    & cargo build --locked --release --target-dir $vmTarget -p audiorouter-windows-audio --example m03_bridge_tone --example m03_cable_inventory
    if ($LASTEXITCODE -ne 0) { throw 'Example tool build failed.' }
} finally {
    $env:RUSTFLAGS = $previousFlags
}
Copy-Item -LiteralPath (Join-Path $vmTarget 'debug\audiorouter-driver-helper.exe') -Destination $tools
Copy-Item -LiteralPath (Join-Path $vmTarget 'release\examples\m03_bridge_tone.exe') -Destination $tools
Copy-Item -LiteralPath (Join-Path $vmTarget 'release\examples\m03_cable_inventory.exe') -Destination $tools

Step '4/6 IOCTL fuzzer'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'tools\m03-bridge-fuzz\build.ps1')
if ($LASTEXITCODE -ne 0) { throw 'Fuzzer build failed.' }
Copy-Item -LiteralPath (Join-Path $workspace 'target\m03-bridge-fuzz\m03-bridge-fuzz.exe') -Destination $tools

Step '5/6 VM scripts'
$acceptance = Join-Path $repo 'tests\acceptance'
$driverScripts = Join-Path $repo 'drivers\audiorouter-virtual'
New-Item -ItemType Directory -Path $acceptance, $driverScripts | Out-Null
foreach ($name in 'm03-driver-vm.ps1', 'm03-driver-vm-support.ps1', 'm03-default-endpoints.cs', 'm03-driver-vm-guards.ps1', 'm03-endpoint-names.json') {
    Copy-Item -LiteralPath (Join-Path $workspace "tests\acceptance\$name") -Destination $acceptance
}
foreach ($name in 'manage.ps1', 'package-tools.ps1') {
    Copy-Item -LiteralPath (Join-Path $driverRoot $name) -Destination $driverScripts
}
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\vm-checks.ps1') -Destination $Share
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\retry-smoke.ps1') -Destination $Share
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\run-packet-clock-review.ps1') -Destination $Share
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\vm-scheduling-trace.ps1') -Destination $Share
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\AudioRouterScheduling.wprp') -Destination $Share

Step '6/6 Manifest'
$commit = (& git -C $workspace rev-parse HEAD).Trim()
$dirty = [bool](& git -C $workspace status --porcelain)
$lines = @(
    "AudioRouter cable VM share",
    "built:   $([DateTime]::UtcNow.ToString('u'))",
    "commit:  $commit$(if ($dirty) { ' (working tree had uncommitted changes)' })",
    "driver:  $Version (test-signed)",
    ""
)
Get-ChildItem -LiteralPath $Share -Recurse -File | Where-Object { $_.Name -ne 'MANIFEST.txt' } | Sort-Object FullName | ForEach-Object {
    $lines += "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower(), $_.FullName.Substring($Share.Length + 1)
}
$lines | Set-Content -LiteralPath (Join-Path $Share 'MANIFEST.txt') -Encoding UTF8

Write-Host "`nReady: $Share" -ForegroundColor Green
Write-Host 'Next: copy this folder into the VM as C:\ar (see docs\operations\virtual-cable-vm-guide.md, part 4).'
Write-Host 'Nothing was installed or trusted on this PC.'
