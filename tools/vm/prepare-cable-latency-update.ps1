<# Host-only: build the static pass-through/engine-route tone tool and impulse probe from a
   clean commit, then copy a verified base bundle with those tools and the
   latency runner added. Never starts audio or installs/loads a driver. #>
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $BaseBundle,
    [Parameter(Mandatory = $true)][string] $Destination)
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
} finally { $env:RUSTFLAGS = $previousFlags }
$toneTool = Join-Path $vmTarget 'release\examples\m03_bridge_tone.exe'
$probe = Join-Path $workspace 'target\m00-probe-cable\m00-probe.exe'
New-Item -ItemType Directory -Path (Split-Path -Parent $probe) -Force | Out-Null
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $workspace 'tools\m00-native-wasapi-probe\build.ps1') -Output $probe -StaticRuntime
if ($LASTEXITCODE -ne 0) { throw 'Static impulse probe build failed.' }
foreach ($tool in @($toneTool, $probe)) { $null = Assert-PortableVmTool $tool }
$selfTest = & $probe cable-impulse-selftest
if ($LASTEXITCODE -ne 0) { throw "Probe pairing self-test failed: $selfTest" }

New-Item -ItemType Directory -Path $output | Out-Null
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    if ($relative -in @('run-cable-latency.ps1', 'quiet-guest.ps1', 'tools\m00-probe.exe')) { throw 'Base already contains latency tools; use a standard candidate base.' }
    $file = [IO.Path]::GetFullPath((Join-Path $output $relative))
    if (-not $file.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $file
}
# The pass-through needs the tone tool from this commit; everything else,
# including the signed driver package, stays byte-identical to the base.
Copy-Item -LiteralPath $toneTool -Destination (Join-Path $output 'tools\m03_bridge_tone.exe') -Force
Copy-Item -LiteralPath $probe -Destination (Join-Path $output 'tools\m00-probe.exe')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-cable-latency.ps1') -Destination $output
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
