<# Host-only: build the static pass-through/engine-route tone tool and impulse probe from a
   clean commit, then copy a verified base bundle with the latency and sustained-continuity
   runners added. Never starts audio or installs/loads a driver. #>
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $BaseBundle,
    [Parameter(Mandatory = $true)][string] $Destination,
    [string] $TimingDriverPackage)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
. (Join-Path $PSScriptRoot 'portable-tool-support.ps1')
$base = [IO.Path]::GetFullPath($BaseBundle).TrimEnd('\')
$output = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if ($base -ieq $output -or $output.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase) -or $base.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Use separate base and destination folders.' }
if ((Test-Path -LiteralPath $output) -and @(Get-ChildItem -LiteralPath $output -Force).Count -gt 0) { throw 'Destination must be new or empty.' }
Assert-PairedTraceBundle $base
$status = & git -C $workspace status --porcelain
if ($LASTEXITCODE -ne 0 -or [bool]$status) { throw 'Package only from a clean identified source commit.' }
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
$driver = Get-Content -LiteralPath (Join-Path $base 'driver\package.json') -Raw | ConvertFrom-Json
if (-not $driver.gitCommit -or $driver.dirty) { throw 'Base driver package lacks a clean source identity.' }

$vmTarget = Join-Path $workspace 'target\vm-tools'
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    & cargo build --locked --release --target-dir $vmTarget -p audiorouter-windows-audio --example m03_bridge_tone
    if ($LASTEXITCODE -ne 0) { throw 'Static tone tool build failed.' }
    # Product cable route through the real control plane (WP-09).
    & cargo build --locked --release --target-dir $vmTarget -p audiorouter-control --example m03_cable_session
    if ($LASTEXITCODE -ne 0) { throw 'Static cable session tool build failed.' }
    # The CLI, for WP-08's `virtual-cable status` acceptance in the guest.
    & cargo build --locked --release --target-dir $vmTarget -p audiorouter-cli
    if ($LASTEXITCODE -ne 0) { throw 'Static CLI build failed.' }
    $qualityManifest = Join-Path $workspace 'tools\m03-cable-quality\Cargo.toml'
    & cargo build --locked --release --manifest-path $qualityManifest --target-dir $vmTarget
    if ($LASTEXITCODE -ne 0) { throw 'Static cable quality analyzer build failed.' }
} finally { $env:RUSTFLAGS = $previousFlags }
$toneTool = Join-Path $vmTarget 'release\examples\m03_bridge_tone.exe'
$cableSessionTool = Join-Path $vmTarget 'release\examples\m03_cable_session.exe'
$cliTool = Join-Path $vmTarget 'release\audiorouter-cli.exe'
$qualityTool = Join-Path $vmTarget 'release\m03-cable-quality.exe'
$probe = Join-Path $workspace 'target\m00-probe-cable\m00-probe.exe'
New-Item -ItemType Directory -Path (Split-Path -Parent $probe) -Force | Out-Null
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'tools\m00-native-wasapi-probe\build.ps1') -Output $probe -StaticRuntime
if ($LASTEXITCODE -ne 0) { throw 'Static impulse probe build failed.' }
foreach ($tool in @($toneTool, $cableSessionTool, $cliTool, $qualityTool, $probe)) { $null = Assert-PortableVmTool $tool }
# Every request the product tool makes, except session.start (no driver here).
# The CLI's cable status must run as a standard user and parse; on the host
# (no AudioRouter driver) it reports notInstalled.
$cliStatus = & $cliTool --json virtual-cable status 2>&1 | Out-String
if ($LASTEXITCODE -ne 0 -or -not (($cliStatus | ConvertFrom-Json).state)) { throw "CLI virtual-cable status check failed: $cliStatus" }
$sessionCheck = & $cableSessionTool --check 2>&1
if ($LASTEXITCODE -ne 0) { throw "Cable session tool request check failed: $sessionCheck" }
$selfTest = & $probe cable-impulse-selftest
if ($LASTEXITCODE -ne 0) { throw "Probe pairing self-test failed: $selfTest" }
$sampleFormatTest = & $probe cable-sample-format-selftest
if ($LASTEXITCODE -ne 0) { throw "Probe integer sample-format self-test failed: $sampleFormatTest" }

New-Item -ItemType Directory -Path $output | Out-Null
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    if ($relative -in @('run-cable-latency.ps1', 'run-cable-latency-starts.ps1', 'run-cable-continuity.ps1', 'quiet-guest.ps1', 'tools\m00-probe.exe', 'tools\m03_cable_session.exe', 'tools\audiorouter-cli.exe', 'run-cable-kill.ps1', 'run-cable-tone.ps1', 'run-cable-bitexact.ps1', 'run-cable-thdn.ps1')) { throw 'Base already contains latency or continuity tools; use a standard candidate base.' }
    $file = [IO.Path]::GetFullPath((Join-Path $output $relative))
    if (-not $file.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $file
}
# The pass-through needs the tone tool from this commit; everything else,
# including the signed driver package, stays byte-identical to the base.
Copy-Item -LiteralPath $toneTool -Destination (Join-Path $output 'tools\m03_bridge_tone.exe') -Force
Copy-Item -LiteralPath $probe -Destination (Join-Path $output 'tools\m00-probe.exe')
Copy-Item -LiteralPath $cableSessionTool -Destination (Join-Path $output 'tools\m03_cable_session.exe')
Copy-Item -LiteralPath $cliTool -Destination (Join-Path $output 'tools\audiorouter-cli.exe')
Copy-Item -LiteralPath $qualityTool -Destination (Join-Path $output 'tools\m03-cable-quality.exe')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-latency.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-latency-starts.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-continuity.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-scheduling.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-kill.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-tone.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-bitexact.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-quality.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-isolation-matrix.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-thdn.ps1') -Destination $output
if ($TimingDriverPackage) {
    $timingPackage = Get-Content -LiteralPath (Join-Path $TimingDriverPackage 'package.json') -Raw | ConvertFrom-Json
    if (-not $timingPackage.timingDiagnostics -or $timingPackage.signed -ne 'test' -or
        $timingPackage.dirty -or $timingPackage.gitCommit -ne $commit) { throw 'Timing package must be test-signed from this clean source commit.' }
    $driverOutput = [IO.Path]::GetFullPath((Join-Path $output 'driver'))
    if ($driverOutput -ine ($output + '\driver') -or -not $driverOutput.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Driver output escapes bundle.' }
    Remove-Item -LiteralPath $driverOutput -Recurse -Force
    Copy-Item -LiteralPath $TimingDriverPackage -Destination $driverOutput -Recurse
    $timingTool = Join-Path $workspace 'target\m03-timing-trace\m03-timing-trace.exe'
    New-Item -ItemType Directory -Path (Split-Path -Parent $timingTool) -Force | Out-Null
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'tools\m03-timing-trace\build.ps1') -Output $timingTool -StaticRuntime
    if ($LASTEXITCODE -ne 0) { throw 'Timing tool build failed.' }
    $null = Assert-PortableVmTool $timingTool
    & $timingTool selftest
    if ($LASTEXITCODE -ne 0) { throw 'Timing tool offline checks failed.' }
    Copy-Item -LiteralPath $timingTool -Destination (Join-Path $output 'tools\m03-timing-trace.exe')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-driver-timing.ps1') -Destination $output
    $driver = $timingPackage
}
# Optional, reversible guest quieting before long diagnostics.
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'quiet-guest.ps1') -Destination $output
$support = Join-Path $output 'paired-trace-support.ps1'
if (-not (Test-Path -LiteralPath $support)) { Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'paired-trace-support.ps1') -Destination $support }
$manifest = @('AudioRouter cable latency diagnostic (proxy or engine route); guest-only measurement', "source: $commit", "base: $base",
    "driver: $($driver.gitCommit) built $($driver.builtAt)", "prepared: $([DateTime]::UtcNow.ToString('u'))", '')
foreach ($file in Get-ChildItem -LiteralPath $output -File -Recurse | Sort-Object FullName) {
    $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(), $file.FullName.Substring($output.Length + 1)
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $output
Write-Host "Cable latency bundle ready: $output"
