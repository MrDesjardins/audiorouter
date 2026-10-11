<# Guest-only bit-exact check of the product cable route at N channels (WP-09 slice 5,
   VCAB-20): the probe renders float32-grid noise into Cable A Input at its shared mix
   format and compares Cable B Output sample for sample, while the product route
   (m03_cable_session --channels N: identity map, multichannel direct mapping when N != 2)
   carries it. Both cables' Windows formats must already be N channels, 48 kHz (Sound
   settings); the run refuses to judge otherwise. Diagnostic only, never qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 8)][int] $Channels = 8,
    [ValidateRange(5, 60)][int] $Seconds = 20)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') { throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.' }
. (Join-Path $bundle 'paired-trace-support.ps1')
. (Join-Path $bundle 'repo\tests\acceptance\m03-driver-vm-support.ps1')
Assert-PairedTraceAdministrator
Assert-PairedTraceBundle $bundle
if (Get-Process -Name 'm03_bridge_tone','m03_cable_session','m03_direct_audio','m00-probe' -ErrorAction SilentlyContinue) { throw 'Another audio test is running. Wait for it to end.' }
$peer = Join-Path 'Z:\' (Split-Path -Leaf $bundle)
if (-not (Test-Path -LiteralPath $peer -PathType Container)) { throw 'Copy the complete bundle from Z: first; evidence destination is missing.' }
$probe = Join-Path $bundle 'tools\m00-probe.exe'
$cableSession = Join-Path $bundle 'tools\m03_cable_session.exe'
if (-not (Test-Path -LiteralPath $cableSession)) { throw 'This bundle has no m03_cable_session.exe.' }
$run = "bitexact-$Channels" + 'ch-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$failure = $null
$route = $null
$result = $null
function Get-ProbeValue([string] $Text, [string] $Name) {
    $match = [regex]::Match($Text, "(?m)\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
try {
    Write-Host "Cable bit-exact (product route, $Channels channels): $Seconds s of noise. No manual playback needed."
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $render = [regex]::Matches($inventoryText, '(?m)^render\[(\d+)\] name=AudioRouter Cable A Input \(AudioRouter Virtual Cable\) id=')
    $capture = [regex]::Matches($inventoryText, '(?m)^capture\[(\d+)\] name=AudioRouter Cable B Output \(AudioRouter Virtual Cable\) id=')
    if ($inventory.Code -ne 0 -or $render.Count -ne 1 -or $capture.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output; no audio test started.' }
    $routeOut = Join-Path $evidence 'route.txt'
    $route = Start-Process -FilePath $cableSession -ArgumentList ((@('--channels',[string]$Channels,'--seconds',[string]($Seconds + 15)) | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -join ' ') `
        -WindowStyle Hidden -PassThru -RedirectStandardOutput $routeOut -RedirectStandardError (Join-Path $evidence 'route-stderr.txt')
    $null = $route.Handle
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while (-not ((Test-Path -LiteralPath $routeOut) -and ((Get-Content -LiteralPath $routeOut -Raw) -match 'cable route running'))) {
        if ($route.HasExited) { throw "The route exited before it ran (exit $($route.ExitCode)); see route-stderr.txt." }
        if ($watch.Elapsed.TotalSeconds -ge 15) { throw 'Route readiness timed out.' }
        Start-Sleep -Milliseconds 50
    }
    Start-Sleep -Milliseconds 1500
    $measured = Invoke-DriverVmProcess -Executable $probe -Arguments @('cable-bitexact',[string]$Seconds,$render[0].Groups[1].Value,$capture[0].Groups[1].Value) `
        -Stdout (Join-Path $evidence 'probe.txt') -Stderr (Join-Path $evidence 'probe-stderr.txt') -TimeoutSeconds ($Seconds + 60)
    if (-not $route.WaitForExit(($Seconds + 60) * 1000)) { throw 'The route did not finish within its watchdog.' }
    $probeText = Get-Content -LiteralPath (Join-Path $evidence 'probe.txt') -Raw
    $routeText = Get-Content -LiteralPath $routeOut -Raw
    $formatChannels = Get-ProbeValue $probeText 'bitexact_channels'
    $result = [pscustomobject][ordered]@{
        Channels = $Channels; ProbeCode = $measured.Code; RouteCode = $route.ExitCode
        FormatChannels = $formatChannels; Pass = Get-ProbeValue $probeText 'bitexact_pass'
        CaptureDropped = Get-ProbeValue $probeText 'bitexact_capture_dropped_frames'
        CableRoute = ([regex]::Match($routeText, '(?m)^final cableRoute=(.*)$')).Groups[1].Value
    }
    if ($formatChannels -ne $Channels) {
        throw "Cable A Input / Cable B Output run at $formatChannels channels in Windows, not $Channels. Set both to $Channels channels, 48000 Hz in Sound settings, then run again."
    }
    $verdict = if ($measured.Code -eq 0 -and $route.ExitCode -eq 0 -and $result.Pass -eq 1) { 'PASS' } else { 'FAIL' }
    $result | Add-Member NoteProperty Verdict $verdict
    Write-Host ("{0} channels: {1} (bitexact_pass={2}, capture dropped {3} frames, probe exit {4}, route exit {5})" -f `
        $Channels, $verdict, $result.Pass, $result.CaptureDropped, $measured.Code, $route.ExitCode)
    Write-Host "  cableRoute=$($result.CableRoute)"
} catch { $failure = $_ } finally {
    if ($route) {
        try { if (-not $route.HasExited) { $route.Kill(); [void]$route.WaitForExit(5000) } } finally { $route.Dispose() }
    }
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Channels = $Channels; Seconds = $Seconds; Result = $result; Qualification = $false
        Scope = 'Bit-exact float32 noise: Cable A Input -> product cable route (control plane, CableRouteWorker; direct mapping when not stereo) -> Cable B Output at the shared mix format; NEM VM, not VCAB-20 qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Bit-exact check measured. Send the output.'
