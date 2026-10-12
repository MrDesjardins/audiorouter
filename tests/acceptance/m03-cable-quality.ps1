<# Host-safe checks for the WP-09b sample analyzer and guarded VM runner.
   Does not open audio streams, inspect/install drivers, or start test tools. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\cable-quality-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}

$runner = Join-Path $workspace 'tools\vm\run-cable-bitexact.ps1'
$qualityRunner = Join-Path $workspace 'tools\vm\run-cable-quality.ps1'
$isolationRunner = Join-Path $workspace 'tools\vm\run-cable-isolation-matrix.ps1'
$thdnRunner = Join-Path $workspace 'tools\vm\run-cable-thdn.ps1'
$sweepRunner = Join-Path $workspace 'tools\vm\run-cable-sweep.ps1'
$prepare = Join-Path $workspace 'tools\vm\prepare-cable-latency-update.ps1'
foreach ($scriptPath in @($runner,$qualityRunner,$isolationRunner,$thdnRunner,$sweepRunner,$prepare)) {
    $tokens = $null; $parseErrors = $null
    [void][Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$parseErrors)
    Assert (($null -eq $parseErrors) -or ($parseErrors.Count -eq 0)) "PowerShell parses: $([IO.Path]::GetFileName($scriptPath))"
}
$runnerText = Get-Content -LiteralPath $runner -Raw
Assert ($runnerText -match 'm03-cable-quality\.exe') 'runner requires the Rust analyzer in the bundle'
Assert ($runnerText -match 'cable-bitexact-reference\.wav' -and $runnerText -match 'cable-bitexact-capture\.wav') 'runner requires both real sample captures'
Assert ($runnerText -match 'metrics\.json' -and $runnerText -match '\$metrics\.bitExact') 'the machine report contributes to the verdict'
Assert ($runnerText -match '\$formatRate -ne \$Rate') 'runner refuses a result at the wrong shared mix rate'
Assert ($runnerText -match 'Qualification = \$false') 'diagnostic runner cannot claim qualification'
Assert ($runnerText -match '\$PcmBits' -and $runnerText -match 'bitexact_encoding') 'product runner verifies the actual negotiated PCM encoding'
$qualityText = Get-Content -LiteralPath $qualityRunner -Raw
Assert ($qualityText -match "float32-44k1.*float32-48k.*float32-96k") 'VM entry point exposes only implemented float32 rate cases'
Assert ($qualityText -match 'pcm16-44k1.*pcm16-48k.*pcm16-96k.*pcm24-44k1.*pcm24-48k.*pcm24-96k.*pcm32-44k1.*pcm32-48k.*pcm32-96k') 'VM entry point exposes PCM16/24/32 at 44.1/48/96 kHz'
Assert ($qualityText -match '\[ValidateRange\(1, 8\)\]\[int\] \$Channels' -and $qualityText -match '\-44k1\$.*44100' -and $qualityText -match '\-96k\$.*96000' -and $runnerText -match 'ValidateSet\(0,16,24,32\)' -and $qualityText -match '\$Matches\[1\]') 'VM entry point routes selected format, rate and channel count'
Assert ($qualityText -match 'isolation-A-to-B' -and $qualityText -match "'silence'.*'-140'") 'VM entry point records a cable isolation measurement against VCAB-26'
Assert ($qualityText -match 'isolation-all-pairs' -and $qualityText -match 'run-cable-isolation-matrix') 'VM entry point wires the ordered isolation matrix'
Assert ($qualityText -match 'thdn-997-48k' -and $qualityText -match 'run-cable-thdn\.ps1') 'VM entry point wires the product-route THD+N diagnostic'
Assert ($qualityText -match 'tone-continuity' -and $qualityText -match 'run-cable-tone\.ps1' -and $qualityText -match '-Minutes \$Minutes') 'VM entry point exposes the existing 997/47 Hz continuity runner'
Assert ($qualityText -match 'eight-cable-continuity' -and $qualityText -match 'run-cable-continuity\.ps1') 'VM entry point exposes the guarded eight-cable continuity runner'
Assert ($qualityText -match 'sweep-44k1.*sweep-48k.*sweep-96k' -and $qualityText -match 'run-cable-sweep\.ps1') 'VM entry point wires logarithmic sweep cases'
$sweepText = Get-Content -LiteralPath $sweepRunner -Raw
Assert ($sweepText -match "'cable-sweep'" -and $sweepText -match "'sweep'.*'20'.*'20000'") 'sweep runner captures and analyzes the 20 Hz-20 kHz product route'
Assert ($sweepText -match 'CaptureDropped' -and $sweepText -match 'CaptureFlagged' -and $sweepText -match 'CaptureOverflow') 'sweep runner rejects discontinuous or overflowed captures'
Assert ($sweepText -match 'Qualification=\$false' -and $sweepText -match 'aliasing/phase not measured') 'sweep runner keeps its partial diagnostic scope explicit'
$thdnText = Get-Content -LiteralPath $thdnRunner -Raw
Assert ($thdnText.Contains("'cable-tone'") -and $thdnText.Contains("'tone',`$reference,`$captureWav")) 'THD+N runner captures and analyzes native tone WAVs'
Assert ($thdnText -match '\$metrics\.pass' -and $thdnText -match 'Qualification=\$false') 'THD+N metric gates only a diagnostic result'
$isolationText = Get-Content -LiteralPath $isolationRunner -Raw
Assert ($isolationText -match '56 ordered float32 cable isolation pairs') 'matrix report labels its measured scope'
Assert ($isolationText -match 'restore-cable-count' -and $isolationText -match '\$originalCount') 'matrix restores the original cable count'
Assert ($qualityText -match 'does not pass A16' -and $qualityText -match 'A16 remains incomplete') 'VM entry point cannot imply complete matrix coverage'
$prepareText = Get-Content -LiteralPath $prepare -Raw
Assert ($prepareText -match 'm03-cable-quality\.exe') 'bundle preparation builds and stages the analyzer'
Assert ($prepareText -match 'run-cable-quality\.ps1') 'bundle preparation stages the WP-09b VM entry point'
Assert ($prepareText -match 'run-cable-isolation-matrix\.ps1') 'bundle preparation stages the isolation matrix runner'
Assert ($prepareText -match 'cable-sample-format-selftest') 'bundle preparation checks the integer sample conversion helpers'
Assert ($prepareText -match 'run-cable-sweep\.ps1') 'bundle preparation stages the sweep runner'
Assert ($prepareText -match 'run-cable-thdn\.ps1') 'bundle preparation stages the THD+N runner'

if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner rejects host before opening audio or starting processes'
}
Write-Host "$checks cable quality harness checks passed. No audio stream or driver tool was opened. Evidence: $root"
