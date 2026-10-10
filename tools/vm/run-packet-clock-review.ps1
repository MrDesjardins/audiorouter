<# Guest-only bounded qualification of the packet-clock repair. #>
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Tone', 'TraceProbe')]
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
$probeDirectory = $null
try {
    if ($Phase -eq 'Prepare') {
        Invoke-Check preflight
        Invoke-Check smoke
        Invoke-Check install
        Invoke-Check status
    } elseif ($Phase -eq 'TraceProbe') {
        $probeDirectory = Join-Path 'C:\ar\evidence' ((Get-Date -Format 'yyyyMMdd-HHmmss') + '-trace-probe-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
        New-Item -ItemType Directory -Path $probeDirectory | Out-Null
        if ((Get-PSDrive -Name C).Free -lt 2GB) { throw 'Trace probe requires 2 GB free on guest C:.' }
        . (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
        . (Join-Path $bundle 'vm-scheduling-trace.ps1')
        Write-Host 'Checking recorder start/save for two seconds. No tone process is launched.'
        Invoke-DriverVmSchedulingProbe -Recorder (Join-Path $env:SystemRoot 'System32\wpr.exe') -Directory $probeDirectory
    } else {
        Invoke-Check status
        Write-Host "Keep the Cable A Input audio loop and Cable B Output listener running for $ToneSeconds seconds."
        Invoke-Check tone
    }
} catch {
    $failure = $_
} finally {
    if ($Phase -eq 'TraceProbe' -and $probeDirectory -and (Test-Path -LiteralPath $probeDirectory)) {
        # No transcript holds these files open. Archive this small probe only;
        # earlier multi-gigabyte audio recordings stay outside this archive.
        [pscustomobject]@{ Phase = $Phase; Passed = -not [bool]$failure; Error = [string]$failure } |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $probeDirectory 'probe-result.json') -Encoding UTF8
        $probeZip = Join-Path $bundle ('evidence-' + (Split-Path -Leaf $probeDirectory) + '.zip')
        Compress-Archive -LiteralPath $probeDirectory -DestinationPath $probeZip
        $zip = Get-Item -LiteralPath $probeZip
    } else {
        Write-Host 'Checks finished. Collecting and copying evidence; large earlier recordings can make this take longer.'
        $collectionStart = Get-Date
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -Step collect
        if ($LASTEXITCODE -ne 0) { throw 'Collection failed; preserve C:\ar\evidence.' }
        $zip = Get-ChildItem -LiteralPath $bundle -Filter 'evidence-*.zip' -File |
            Where-Object { $_.LastWriteTime -ge $collectionStart } |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
    }
    if (-not $zip) { throw 'No new evidence archive was found.' }
    Copy-Item -LiteralPath $zip.FullName -Destination 'Z:\'
    Get-FileHash -LiteralPath $zip.FullName -Algorithm SHA256
    Write-Host "Evidence copied to shared folder: $($zip.Name)"
}
if ($failure) { throw $failure }
Write-Host "$Phase passed."
