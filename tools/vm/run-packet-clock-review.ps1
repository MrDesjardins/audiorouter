<# Guest-only bounded qualification of the packet-clock repair. #>
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Tone')]
    [string] $Phase = 'Prepare',
    [ValidateSet(30, 300)]
    [int] $ToneSeconds = 30,
    [switch] $TraceScheduling
)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') {
    throw 'Run this script only from the copied bundle under C:\ar inside AR-DriverTest.'
}
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Open Administrator PowerShell inside the VM.'
}
if (-not (Test-Path -LiteralPath 'Z:\' -PathType Container)) {
    throw 'The Z: shared folder must be connected before starting.'
}
if ($TraceScheduling -and $Phase -ne 'Tone') { throw 'Scheduling trace is available only for the Tone phase.' }
$entries = @(Get-Content -LiteralPath (Join-Path $bundle 'MANIFEST.txt') |
    Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
if ($entries.Count -eq 0) { throw 'Bundle manifest has no hashes.' }
foreach ($entry in $entries) {
    $relative = $entry.Substring(66)
    $path = [IO.Path]::GetFullPath((Join-Path $bundle $relative))
    if (-not $path.StartsWith($bundle + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Manifest entry is outside the bundle.'
    }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $entry.Substring(0, 64)) {
        throw "Bundle hash mismatch: $relative"
    }
    Unblock-File -LiteralPath $path
}
Write-Host "Verified $($entries.Count) bundle files."
Get-Content -LiteralPath (Join-Path $bundle 'driver\package.json') -Raw | Write-Host
$script = Join-Path $bundle 'vm-checks.ps1'
function Invoke-Check([string] $Step) {
    $checkArguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $script, '-Step', $Step, '-ToneSeconds', $ToneSeconds)
    if ($TraceScheduling -and $Step -eq 'tone') { $checkArguments += '-TraceScheduling' }
    & powershell.exe @checkArguments
    if ($LASTEXITCODE -ne 0) { throw "$Step failed. Stop here; collecting evidence." }
}
$failure = $null
try {
    if ($Phase -eq 'Prepare') {
        Invoke-Check preflight
        Invoke-Check smoke
        Invoke-Check install
        Invoke-Check status
    } else {
        Invoke-Check status
        Write-Host "Keep the Cable A Input audio loop and Cable B Output listener running for $ToneSeconds seconds."
        Invoke-Check tone
    }
} catch {
    $failure = $_
} finally {
    Write-Host 'Checks finished. Collecting and copying evidence; large earlier recordings can make this take longer.'
    $collectionStart = Get-Date
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -Step collect
    if ($LASTEXITCODE -ne 0) { throw 'Collection failed; preserve C:\ar\evidence.' }
    $zip = Get-ChildItem -LiteralPath $bundle -Filter 'evidence-*.zip' -File |
        Where-Object { $_.LastWriteTime -ge $collectionStart } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $zip) { throw 'No new evidence archive was found.' }
    Copy-Item -LiteralPath $zip.FullName -Destination 'Z:\'
    Get-FileHash -LiteralPath $zip.FullName -Algorithm SHA256
    Write-Host "Evidence copied to shared folder: $($zip.Name)"
}
if ($failure) { throw $failure }
Write-Host "$Phase passed."
