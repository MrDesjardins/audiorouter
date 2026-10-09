[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$source = Get-Content -LiteralPath (Join-Path $workspace 'tools/vm/vm-checks.ps1') -Raw
# Evaluate only the tool wrapper, never the VM script's driver operations.
$start = $source.IndexOf('function Invoke-Inventory(')
$end = $source.IndexOf('function Get-CableIds(', $start)
if ($start -lt 0 -or $end -le $start) { throw 'Inventory function boundary missing.' }
. ([scriptblock]::Create($source.Substring($start, $end - $start)))
$evidence = Join-Path $workspace ('target/inventory-output-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$inventoryTool = Join-Path $evidence 'fixture.cmd'
@'
@echo off
echo fixture JSON report
echo fixture summary 1>&2
exit /b %2%
'@ | Set-Content -LiteralPath $inventoryTool -Encoding ASCII
foreach ($code in @(0, 1)) {
    $result = Invoke-Inventory $code "case-$code"
    if ($result.Code -ne $code -or
        -not $result.Text.Contains('fixture JSON report') -or
        -not $result.Text.Contains('fixture summary') -or
        -not (Test-Path -LiteralPath (Join-Path $evidence "case-$code.txt"))) {
        throw "Native inventory output/exit preservation failed for $code."
    }
    if ($ErrorActionPreference -ne 'Stop') { throw 'Error preference was not restored.' }
}
Write-Host "VM inventory output regression passed (native exit 0/1, stdout/stderr retained). Evidence: $evidence"
$tone = $inventoryTool
foreach ($code in @(0, 1)) {
    $result = Invoke-ToneTool @('--cables', $code) (Join-Path $evidence 'fixture.wav')
    if ($result.Code -ne $code -or
        -not $result.Text.Contains('fixture JSON report') -or
        -not $result.Text.Contains('fixture summary') -or
        $ErrorActionPreference -ne 'Stop') {
        throw 'Tone live output/exit preservation failed.'
    }
}
Write-Host 'Tone live output regression passed (native stdout/stderr and exit 0/1 retained).'
$summaryStart = $source.IndexOf('function Show-ToneSummary(')
$summaryEnd = $source.IndexOf('try {', $summaryStart)
if ($summaryStart -lt 0 -or $summaryEnd -le $summaryStart) { throw 'Tone summary boundary missing.' }
. ([scriptblock]::Create($source.Substring($summaryStart, $summaryEnd - $summaryStart)))
$lines = @(Show-ToneSummary (@(
    'lease-open capture=NativeBridgeStreamCounters { underrun_frames: 1 }',
    'progress 1000 ms: capture=NativeBridgeStreamCounters { underrun_frames: 1 }',
    'capture-sink counters (cable-b): NativeBridgeStreamCounters { underrun_frames: 1 }',
    'render-source counters (cable-a): NativeBridgeStreamCounters { underrun_frames: 0 }'
) -join "`n"))
if ($lines.Count -ne 2 -or $lines[0] -notmatch '^capture-sink counters') {
    throw 'Tone progress must not be counted again as a final counter summary.'
}
Write-Host 'Tone summary regression passed (only the two final counter reports).'
$zero = 'underrun_frames: 0, overrun_frames: 0, sequence_gaps: 0, non_finite_samples: 0, format_mismatches: 0'
$capture = "capture-sink counters (cable-b): $zero"
$render = "render-source counters (cable-a): $zero"
$good = Get-ToneCounterResult @($capture, $render)
if (-not $good.Complete -or $good.Sum -ne 0) { throw 'Complete zero report rejected.' }
foreach ($bad in @(
    @{ Lines = @($capture) },
    @{ Lines = @($capture, 'render-source counters (cable-a): underrun_frames: 0') },
    @{ Lines = @($capture, $render + ', underrun_frames: 0') },
    @{ Lines = @($capture, $capture) }
)) {
    if ((Get-ToneCounterResult $bad.Lines).Complete) { throw 'Incomplete/duplicate counter report accepted.' }
}
$failure = Get-ToneCounterResult @($capture.Replace('underrun_frames: 0', 'underrun_frames: 1104'), $render)
if (-not $failure.Complete -or $failure.Sum -ne 1104) { throw 'Underruns were hidden.' }
Write-Host 'Tone counter regression passed (zero, missing, duplicate and 1104-underrun reports).'
