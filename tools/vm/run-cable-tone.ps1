<# Guest-only sine continuity of the product cable route (WP-09, VCAB-24 method,
   validated lesson 2026-09-26): a quiet tone plays into Cable A Input, the product
   route (control plane + CableRouteWorker) carries it to Cable B Output, and every
   sample-level discontinuity is counted in Cable B and in a loopback reference of
   Cable A Input. 997 Hz then 47 Hz (block-sized jumps cannot alias at 47 Hz).
   Pass per tone: Cable B 0 glitches and 0 silent runs with the tone present; a
   reference with glitches makes that tone inconclusive (harness disturbed).
   NEM VM deviation run, never VCAB-24 qualification. #>
[CmdletBinding()]
param([ValidateRange(1, 60)][int] $Minutes = 10,
    [double[]] $ToneHz = @(997, 47))
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
$run = 'tone-' + [Guid]::NewGuid().ToString('N')
$evidence = Join-Path 'C:\ar\evidence' $run
New-Item -ItemType Directory -Path $evidence | Out-Null
$powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$results = @()
$failure = $null
function Get-ToneValue([string] $Line, [string] $Name) {
    $match = [regex]::Match($Line, "\b$([regex]::Escape($Name))=(-?[0-9.]+)")
    if ($match.Success) { return [double]$match.Groups[1].Value }
    return $null
}
try {
    Write-Host "Cable tone continuity (product route): $($ToneHz -join ' Hz, ') Hz, $Minutes minute(s) each. No manual playback needed."
    $status = Invoke-DriverVmProcess -Executable $powershell -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $bundle 'vm-checks.ps1'),'-Step','status') `
        -Stdout (Join-Path $evidence 'status.txt') -Stderr (Join-Path $evidence 'status-stderr.txt') -TimeoutSeconds 60
    if ($status.Code -ne 0) { throw 'Installed-driver status failed. No audio test started.' }
    $inventory = Invoke-DriverVmProcess -Executable $probe -Arguments @('inventory') -Stdout (Join-Path $evidence 'inventory.txt') `
        -Stderr (Join-Path $evidence 'inventory-stderr.txt') -TimeoutSeconds 20
    $inventoryText = Get-Content -LiteralPath (Join-Path $evidence 'inventory.txt') -Raw
    $render = [regex]::Matches($inventoryText, '(?m)^render\[\d+\] name=AudioRouter Cable A Input \(AudioRouter Virtual Cable\) id=(\S+)')
    $capture = [regex]::Matches($inventoryText, '(?m)^capture\[\d+\] name=AudioRouter Cable B Output \(AudioRouter Virtual Cable\) id=(\S+)')
    if ($inventory.Code -ne 0 -or $render.Count -ne 1 -or $capture.Count -ne 1) { throw 'Expected exactly one Cable A Input and one Cable B Output; no audio test started.' }
    $seconds = $Minutes * 60 + 3
    foreach ($hz in $ToneHz) {
        $name = "tone-$([int]$hz)hz"
        Write-Host "$name running for $seconds s..."
        $routed = Invoke-DriverVmProcess -Executable $cableSession `
            -Arguments @('--seconds',[string]$seconds,'--tone',[string]$hz,'--render-endpoint',$render[0].Groups[1].Value,'--capture-endpoint',$capture[0].Groups[1].Value) `
            -Stdout (Join-Path $evidence "$name.txt") -Stderr (Join-Path $evidence "$name-stderr.txt") -TimeoutSeconds ($seconds + 60)
        $text = Get-Content -LiteralPath (Join-Path $evidence "$name.txt") -Raw
        $result = ([regex]::Match($text, '(?m)^tone result \(Cable B Output\): (.*)$')).Groups[1].Value
        $reference = ([regex]::Match($text, '(?m)^tone reference \(Cable A Input loopback\): (.*)$')).Groups[1].Value
        $routeLine = ([regex]::Match($text, '(?m)^final cableRoute=(.*)$')).Groups[1].Value
        $entry = [pscustomobject][ordered]@{
            ToneHz = $hz; ExitCode = $routed.Code
            AnalysedSeconds = Get-ToneValue $result 'analysed_s'; ToneFraction = Get-ToneValue $result 'tone_fraction'
            Glitches = Get-ToneValue $result 'glitches'; SilentRuns = Get-ToneValue $result 'silent_runs'
            DiscontinuityFlags = Get-ToneValue $result 'discontinuity_flags'
            ReferenceGlitches = Get-ToneValue $reference 'glitches'; ReferenceSilentRuns = Get-ToneValue $reference 'silent_runs'
            Result = $result; Reference = $reference; CableRoute = $routeLine
        }
        $clean = $routed.Code -eq 0 -and $entry.ToneFraction -gt 0.9 -and $entry.Glitches -eq 0 -and $entry.SilentRuns -eq 0
        $referenceClean = $entry.ReferenceGlitches -eq 0 -and $entry.ReferenceSilentRuns -eq 0
        $verdict = if ($clean) { 'pass' } elseif (-not $referenceClean) { 'inconclusive (reference disturbed)' } else { 'fail' }
        $entry | Add-Member NoteProperty Verdict $verdict
        $results += $entry
        Write-Host ("{0}: {1}; Cable B {2:N1} s analysed, {3} glitches, {4} silent runs, {5} flags; reference {6} glitches, {7} silent runs; route exit {8}" -f `
            $name, $verdict, $entry.AnalysedSeconds, $entry.Glitches, $entry.SilentRuns, $entry.DiscontinuityFlags,
            $entry.ReferenceGlitches, $entry.ReferenceSilentRuns, $routed.Code)
    }
} catch { $failure = $_ } finally {
    Write-PairedTraceJson (Join-Path $evidence 'result.json') @{ Run = $run; Passed = (-not [bool]$failure); Error = [string]$failure
        Minutes = $Minutes; ToneHz = $ToneHz; Results = $results; Qualification = $false
        Scope = 'Sine continuity: Cable A Input -> product cable route (control plane, CableRouteWorker) -> Cable B Output, with a Cable A Input loopback reference; NEM VM deviation run, not VCAB-24 qualification' }
    $zip = Join-Path $bundle "$run.zip"
    Compress-Archive -LiteralPath $evidence -DestinationPath $zip
    Copy-Item -LiteralPath $zip -Destination $peer
    Get-FileHash -LiteralPath $zip -Algorithm SHA256
    Write-Host "Current-run reports copied to Z:\$(Split-Path -Leaf $bundle)\$run.zip"
}
if ($failure) { throw $failure }
Write-Host 'Tone continuity measured. Send the output.'
