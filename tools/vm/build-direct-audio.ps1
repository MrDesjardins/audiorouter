<# Build-only user-mode helper. Never starts audio or loads/installs a driver. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'portable-tool-support.ps1')
$vmTarget = Join-Path $workspace 'target\vm-direct-audio'
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    & cargo build --locked --release --target-dir $vmTarget --manifest-path (Join-Path $workspace 'Cargo.toml') -p audiorouter-windows-audio --example m03_direct_audio
    if ($LASTEXITCODE -ne 0) { throw 'Static VM diagnostic build failed.' }
} finally { $env:RUSTFLAGS = $previousFlags }
$imports = @(Assert-PortableVmTool (Join-Path $vmTarget 'release\examples\m03_direct_audio.exe'))
Write-Host "Static-runtime VM helper built and imports verified: $($imports -join ', ')"
