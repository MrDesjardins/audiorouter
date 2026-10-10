<# Copy-only update: preserve every base file; add the verified user-mode diagnostic. #>
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string] $BaseBundle,
    [Parameter(Mandatory=$true)][string] $Destination)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
$base = [IO.Path]::GetFullPath($BaseBundle).TrimEnd('\')
$output = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if ($base -ieq $output -or $output.StartsWith($base+'\',[StringComparison]::OrdinalIgnoreCase) -or $base.StartsWith($output+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Use separate base and destination folders.' }
if ((Test-Path -LiteralPath $output) -and @(Get-ChildItem -LiteralPath $output -Force).Count -gt 0) { throw 'Destination must be new or empty.' }
Assert-PairedTraceBundle $base
$binary = Join-Path $workspace 'target\release\examples\m03_direct_audio.exe'
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Build the user-mode diagnostic first.' }
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
$status = & git -C $workspace status --porcelain
if ($LASTEXITCODE -ne 0 -or [bool]$status) { throw 'Package only from a clean identified source commit.' }
$sources = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'crates\windows-audio\examples\m03_direct_audio') -File -Recurse)
$sources += Get-Item -LiteralPath (Join-Path $workspace 'crates\windows-audio\examples\m03_direct_audio.rs')
if ((Get-Item -LiteralPath $binary).LastWriteTimeUtc -lt ($sources | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).LastWriteTimeUtc) { throw 'Diagnostic binary is older than its source.' }
New-Item -ItemType Directory -Path $output | Out-Null
$entries = @(Get-Content -LiteralPath (Join-Path $base 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    if ($relative -in @('run-direct-audio.ps1','tools\m03_direct_audio.exe')) { throw 'Base already contains the direct audio update.' }
    $file = [IO.Path]::GetFullPath((Join-Path $output $relative))
    if (-not $file.StartsWith($output+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes output.' }
    New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $base $relative) -Destination $file
}
Copy-Item -LiteralPath $binary -Destination (Join-Path $output 'tools\m03_direct_audio.exe')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-direct-audio.ps1') -Destination $output
$manifest = @('AudioRouter automatic direct audio diagnostic; guest-only record, offline analyze',"source: $commit", "base: $base", "prepared: $([DateTime]::UtcNow.ToString('u'))",'')
foreach ($file in Get-ChildItem -LiteralPath $output -File -Recurse | Sort-Object FullName) {
    $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(),$file.FullName.Substring($output.Length+1)
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $output
Write-Host "Copy-only direct audio bundle ready: $output"
