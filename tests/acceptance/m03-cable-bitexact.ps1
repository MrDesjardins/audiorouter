<# Host-safe checks of the product-route bit-exact runner: syntax, the format guard,
   the verdict rule and the VM-only guard. No audio, driver or process is touched. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\cable-bitexact-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$runner = Join-Path $workspace 'tools\vm\run-cable-bitexact.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
$text = Get-Content -LiteralPath $runner -Raw
Assert ($text -match "'--channels'") 'the product route gets the channel count'
Assert ($text -match '\$formatChannels -ne \$Channels') 'a run at the wrong Windows format is refused, not judged'
Assert ($text -match '\$result.Pass -eq 1') 'pass requires the probe bit-exact verdict'
Assert ($text -match 'Qualification = \$false') 'never claims qualification'
$definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-ProbeValue' }, $true)
. ([scriptblock]::Create($definition.Extent.Text))
$sample = "bitexact_channels=8 bitexact_rate=48000 bitexact_rendered_frames=1`nbitexact_pass=1"
Assert ((Get-ProbeValue $sample 'bitexact_channels') -eq 8 -and (Get-ProbeValue $sample 'bitexact_pass') -eq 1) 'probe result lines parse'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks cable bit-exact checks passed. No audio stream or driver tool was opened. Evidence: $root"
