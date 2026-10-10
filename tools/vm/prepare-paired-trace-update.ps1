<# Copy-only diagnostics update; never build, install or invoke driver/audio tools. #>
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $BaseBundle,
    [Parameter(Mandatory = $true)][string] $Destination)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
$base = [IO.Path]::GetFullPath($BaseBundle).TrimEnd('\')
$targetBundle = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if ($base -ieq $targetBundle -or $targetBundle.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase) -or
    $base.StartsWith($targetBundle + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Base and output must be separate folders.' }
if ((Test-Path -LiteralPath $targetBundle) -and @(Get-ChildItem -LiteralPath $targetBundle -Force).Count -gt 0) { throw 'Output must be new or empty.' }
Assert-PairedTraceBundle $base
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
New-Item -ItemType Directory -Path $targetBundle -Force | Out-Null
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    $targetFile = [IO.Path]::GetFullPath((Join-Path $targetBundle $relative))
    if (-not $targetFile.StartsWith($targetBundle + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Output manifest path escapes bundle.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $targetFile) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $targetFile
}
foreach ($name in @('paired-trace-support.ps1','run-paired-scheduling-host.ps1','run-paired-scheduling-guest.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $targetBundle
}
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
$dirty = [bool](& git -C $workspace status --porcelain)
$manifest = @('AudioRouter paired scheduling diagnostics; host recorder only, guest explicit phase', "base: $base",
    "script source: $commit$(if ($dirty) { ' (working tree dirty)' })", "prepared: $([DateTime]::UtcNow.ToString('u'))", '')
foreach ($file in Get-ChildItem -LiteralPath $targetBundle -Recurse -File | Sort-Object FullName) {
    if ($file.Name -ne 'MANIFEST.txt') {
        $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(), $file.FullName.Substring($targetBundle.Length + 1)
    }
}
$manifest | Set-Content -LiteralPath (Join-Path $targetBundle 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $targetBundle
Write-Host "Copy-only diagnostics bundle ready: $targetBundle"
