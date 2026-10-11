<# Host-safe checks of the product-route tone continuity runner: syntax, verdict
   rules, tool arguments and the VM-only guard. No audio, driver or process is touched. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\cable-tone-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$runner = Join-Path $workspace 'tools\vm\run-cable-tone.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
$text = Get-Content -LiteralPath $runner -Raw
Assert ($text -match "'--tone'" -and $text -match "'--render-endpoint'" -and $text -match "'--capture-endpoint'") 'the product tool gets the tone and exact endpoint IDs'
Assert ($text -match '\$entry.Glitches -eq 0 -and \$entry.SilentRuns -eq 0') 'pass requires zero glitches and zero silent runs'
Assert ($text -match 'inconclusive') 'a disturbed reference is inconclusive, not a pass'
Assert ($text -match 'Qualification = \$false') 'never claims qualification'
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-ToneValue' }, $true)
. ([scriptblock]::Create($definition.Extent.Text))
$line = 'analysed_s=597.0 tone_fraction=0.987 peak=0.0500 glitches=0 max_residual=0.00 silent_runs=0 silent_samples=0 discontinuity_flags=1 first_glitches_s=[]'
Assert ((Get-ToneValue $line 'glitches') -eq 0 -and (Get-ToneValue $line 'analysed_s') -eq 597.0 -and (Get-ToneValue $line 'silent_runs') -eq 0 -and $null -eq (Get-ToneValue $line 'missing')) 'tool result line parses'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks cable tone checks passed. No audio stream or driver tool was opened. Evidence: $root"
