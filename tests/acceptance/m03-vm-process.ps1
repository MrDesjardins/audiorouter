<# Host-safe regression: fake child processes only, no driver/VM/audio calls. #>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'm03-driver-vm-support.ps1')
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$evidence = Join-Path $workspace ('target\vm-process-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$fixture = Join-Path $evidence 'fake child.ps1'
@'
param([string] $Mode, [string] $Value)
[Console]::Out.WriteLine('argument=' + $Value)
[Console]::Error.WriteLine('diagnostic on stderr')
if ($Mode -eq 'flood') { for ($i = 0; $i -lt 10000; $i++) { [Console]::Out.WriteLine(('x' * 100) + $i) } }
if ($Mode -eq 'hang') { Start-Sleep -Seconds 30 }
exit 7
'@ | Set-Content -LiteralPath $fixture -Encoding UTF8
$checks = 0
function Assert([bool] $Passed, [string] $Name) {
    if (-not $Passed) { throw "FAIL: $Name; evidence $evidence" }
    $script:checks++
}
$powerShell = Join-Path $PSHOME 'powershell.exe'
$value = 'space "quote" trailing\'
$stdout = Join-Path $evidence 'success-out.txt'
$stderr = Join-Path $evidence 'success-err.txt'
$result = Invoke-DriverVmProcess -Executable $powerShell -Arguments @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixture, '-Mode', 'finish', '-Value', $value
) -Stdout $stdout -Stderr $stderr -TimeoutSeconds 10
Assert ($result.Code -eq 7 -and -not $result.TimedOut) 'native nonzero exit preserved'
Assert ((Get-Content -LiteralPath $stdout -Raw).Trim() -ceq ('argument=' + $value)) 'argv quoting and stdout preserved'
Assert ((Get-Content -LiteralPath $stderr -Raw).Trim() -eq 'diagnostic on stderr') 'stderr preserved without PowerShell native error'
$result = Invoke-DriverVmProcess -Executable $powerShell -Arguments @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixture, '-Mode', 'flood', '-Value', 'large output'
) -Stdout (Join-Path $evidence 'flood-out.txt') -Stderr (Join-Path $evidence 'flood-err.txt') -TimeoutSeconds 10
Assert ($result.Code -eq 7 -and -not $result.TimedOut) 'output beyond pipe capacity does not wait for console'
Assert (@(Get-Content -LiteralPath (Join-Path $evidence 'flood-out.txt')).Count -eq 10001) 'large redirected output is complete'
$watch = [Diagnostics.Stopwatch]::StartNew()
$result = Invoke-DriverVmProcess -Executable $powerShell -Arguments @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixture, '-Mode', 'hang', '-Value', 'timeout'
) -Stdout (Join-Path $evidence 'timeout-out.txt') -Stderr (Join-Path $evidence 'timeout-err.txt') -TimeoutSeconds 1
Assert ($result.Code -eq 124 -and $result.TimedOut) 'watchdog reports invalid timeout'
Assert ($watch.Elapsed.TotalSeconds -lt 10) 'watchdog stops the owned child promptly'
Assert (-not (Get-Process -Id $result.ProcessId -ErrorAction SilentlyContinue)) 'timed-out child is no longer running'
Assert ((Get-Content -LiteralPath (Join-Path $evidence 'timeout-out.txt') -Raw).Trim() -eq 'argument=timeout') 'partial output survives timeout'
$processGuard = Get-Content -LiteralPath (Join-Path $workspace 'tools\vm\vm-checks.ps1') -Raw
Assert ($processGuard -match 'Invoke-DriverVmProcess -Executable \$tone') 'tone uses bounded process runner'
Assert ($processGuard -notmatch '& \$tone @Arguments') 'tone output is not piped through console'
Write-Host "$checks host-only process regressions pass. Evidence: $evidence"
