<# Package a static user-mode recorder; no driver files or settings are touched. #>
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string] $ReferenceFile,
      [Parameter(Mandatory=$true)][string] $Destination)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $PSScriptRoot 'paired-trace-support.ps1')
. (Join-Path $PSScriptRoot 'portable-tool-support.ps1')
$output = [IO.Path]::GetFullPath($Destination).TrimEnd('\')
if (Test-Path -LiteralPath $output) { throw 'Use a new destination; existing bundles are immutable.' }
$binary = Join-Path $workspace 'target\vm-direct-audio\release\examples\m03_direct_audio.exe'
$null = Assert-PortableVmTool $binary
if ((Get-FileHash -LiteralPath $ReferenceFile -Algorithm SHA256).Hash -cne 'C9652D3C629C45FE7BAC8AB0C332A6668F7BD584E3A55323BA0CE9F54842D6A7') { throw 'Use the verified independent 44.1-kHz reference.' }
$commit = (& git -C $workspace rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
$status = & git -C $workspace status --porcelain
if ($LASTEXITCODE -ne 0 -or [bool]$status) { throw 'Package only from a clean source commit.' }
$sources = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'crates\windows-audio\examples\m03_direct_audio') -File -Recurse)
$sources += Get-Item -LiteralPath (Join-Path $workspace 'crates\windows-audio\examples\m03_direct_audio.rs')
if ((Get-Item -LiteralPath $binary).LastWriteTimeUtc -lt ($sources | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).LastWriteTimeUtc) { throw 'Rebuild the diagnostic after its source changes.' }
New-Item -ItemType Directory -Path $output | Out-Null
foreach ($file in @('run-speaker-loopback.ps1','paired-trace-support.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination $output
}
Copy-Item -LiteralPath (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1') -Destination $output
Copy-Item -LiteralPath $binary -Destination (Join-Path $output 'm03_direct_audio.exe')
Copy-Item -LiteralPath $ReferenceFile -Destination (Join-Path $output 'reference-44100.wav')
$manifest = @('Guest speaker loopback; raw triage evidence, not qualification',"source: $commit", "prepared: $([DateTime]::UtcNow.ToString('u'))",'')
foreach ($file in Get-ChildItem -LiteralPath $output -File | Sort-Object Name) {
    $manifest += '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLower(),$file.Name
}
$manifest | Set-Content -LiteralPath (Join-Path $output 'MANIFEST.txt') -Encoding UTF8
Assert-PairedTraceBundle $output
Write-Host "New speaker capture bundle prepared: $output"
