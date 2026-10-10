<# Build-only harness update. Reuse an independently hash-verified driver package. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $BaseBundle,
    [Parameter(Mandatory = $true)][string] $Share
)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'drivers\audiorouter-virtual\package-tools.ps1')
$base = (Assert-DriverPackagePath $BaseBundle).TrimEnd('\')
$destination = (Assert-DriverPackagePath $Share).TrimEnd('\')
if ($base -ieq $destination -or $destination.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase) -or
    $base.StartsWith($destination + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Base and output bundles must be separate directories.'
}
if ((Test-Path -LiteralPath $destination) -and @(Get-ChildItem -LiteralPath $destination -Force).Count -ne 0) {
    throw 'Output must be a new or empty folder; previous bundles are preserved.'
}
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
if ($entries.Count -eq 0) { throw 'Base bundle manifest has no hashes.' }
foreach ($entry in $entries) {
    $source = [IO.Path]::GetFullPath((Join-Path $base $entry.Substring(66)))
    if (-not $source.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes base bundle.' }
    if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ine $entry.Substring(0, 64)) {
        throw "Base bundle hash mismatch: $($entry.Substring(66))"
    }
}
# Build just the user-mode tool, without rebuilding/loading/installing a driver.
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    & cargo build --locked --release --target-dir (Join-Path $workspace 'target\vm-tools') `
        -p audiorouter-windows-audio --example m03_bridge_tone
    if ($LASTEXITCODE -ne 0) { throw 'Tone example build failed.' }
} finally { $env:RUSTFLAGS = $previousFlags }
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    $target = [IO.Path]::GetFullPath((Join-Path $destination $relative))
    if (-not $target.StartsWith($destination + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output bundle.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $target
}
Copy-Item -LiteralPath (Join-Path $workspace 'target\vm-tools\release\examples\m03_bridge_tone.exe') `
    -Destination (Join-Path $destination 'tools\m03_bridge_tone.exe')
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\vm-checks.ps1') -Destination $destination
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\run-packet-clock-review.ps1') -Destination $destination
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\vm-scheduling-trace.ps1') -Destination $destination
Copy-Item -LiteralPath (Join-Path $workspace 'tools\vm\AudioRouterScheduling.wprp') -Destination $destination
Copy-Item -LiteralPath (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1') `
    -Destination (Join-Path $destination 'repo\tests\acceptance\m03-driver-vm-support.ps1')
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
$dirty = [bool](& git -C $workspace status --porcelain)
$manifest = @(
    'AudioRouter VM harness update; driver package unchanged from verified base bundle',
    "base: $base",
    "tool source: $commit$(if ($dirty) { ' (working tree dirty)' })",
    "built: $([DateTime]::UtcNow.ToString('u'))", ''
)
foreach ($file in Get-ChildItem -LiteralPath $destination -Recurse -File | Sort-Object FullName) {
    if ($file.Name -ne 'MANIFEST.txt') {
        $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(), $file.FullName.Substring($destination.Length + 1)
    }
}
$manifest | Set-Content -LiteralPath (Join-Path $destination 'MANIFEST.txt') -Encoding UTF8
Write-Host "Harness update ready: $destination"
