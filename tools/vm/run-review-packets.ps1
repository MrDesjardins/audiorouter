<# Guest-only packet diagnostic. No host install or automatic reboot. #>
[CmdletBinding()]
param([ValidateSet('Install', 'Run', 'Rollback')][string] $Phase = 'Run')
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') {
    throw 'Run only from the copied bundle under C:\ar inside AR-DriverTest.'
}
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Use Administrator PowerShell inside the VM.'
}
foreach ($entry in @(Import-Csv -LiteralPath (Join-Path $bundle 'packet-manifest.csv'))) {
    $path = [IO.Path]::GetFullPath((Join-Path $bundle $entry.Path))
    if (-not $path.StartsWith($bundle + '\', [StringComparison]::OrdinalIgnoreCase) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $entry.Sha256) {
        throw "Bundle integrity failed: $($entry.Path)"
    }
}
$helper = Join-Path $bundle 'tools\audiorouter-driver-helper.exe'
if ($Phase -in @('Install', 'Rollback')) {
    $package = if ($Phase -eq 'Install') { Join-Path $bundle 'driver' } else { Join-Path $bundle 'rollback-driver' }
    $previous = $env:AUDIOROUTER_ALLOW_TEST_DRIVER
    try {
        $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
        & $helper repair --package $package
        $code = $LASTEXITCODE
    } finally { $env:AUDIOROUTER_ALLOW_TEST_DRIVER = $previous }
    if ($code -notin @(0, 5)) { throw "Driver repair failed ($code). Stop here." }
    Write-Host 'Restart the VM before the next diagnostic. Do not restore a snapshot.'
    exit 0
}
if (-not (Test-Path -LiteralPath 'Z:\' -PathType Container)) { throw 'Shared folder Z: is required.' }
if ((Get-PSDrive C).Free -lt 2GB) { throw 'Requires 2 GB free on guest C:.' }
if (-not (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)) {
    New-Item -ItemType File -Path 'C:\ar\IS_TEST_VM' | Out-Null
}
$run = 'review-packets-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$started = Get-Date
$code = 99
try {
    Write-Host 'Keep Cable B Output listening through VM speakers.'
    Write-Host 'During this 30-second run, repeat Cable A Input Test as in the preceding run.'
    Write-Host 'Diagnostic only. Nonzero audio counters remain failures; the packet trace will still be saved.'
    & (Join-Path $bundle 'tools\m03-timing-trace.exe') $bundle $evidence --tone
    $code = $LASTEXITCODE
} finally {
    # Preserve reports even when tone or trace validation fails. Copy only
    # this invocation's tone evidence, not earlier recordings or entire history.
    foreach ($tone in @(Get-ChildItem 'C:\ar\evidence' -Directory -Filter '*-tone' |
        Where-Object { $_.CreationTime -ge $started })) {
        Copy-Item -LiteralPath $tone.FullName -Destination $evidence -Recurse
    }
    [ordered]@{ ExitCode = $code; Qualification = $false; Started = $started.ToString('o') } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'result.json') -Encoding UTF8
    Copy-Item -LiteralPath (Join-Path $bundle 'driver\package.json') -Destination (Join-Path $evidence 'driver-package.json')
    $zip = Join-Path $bundle ($run + '.zip')
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination 'Z:\'
    Write-Host "Evidence copied to Z:\$run.zip (tool exit $code)."
}
exit $code
