<# Host-safe checks of the A7 kill runner: syntax, the VM-only guard, and the host
   review's verdict logic on synthetic data. No audio, driver or process is touched. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\cable-kill-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$runner = Join-Path $workspace 'tools\vm\run-cable-kill.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($runner, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell runner syntax'
$text = Get-Content -LiteralPath $runner -Raw
Assert ($text -match 'Kill\(\)' -and $text -match 'Get-Qpc100ns') 'the route is force-killed at a QPC-dated moment'
Assert ($text -match 'Sleep -Milliseconds 3500') 'the restart waits past the 2 s lease timeout'
Assert ($text -match 'Qualification = \$false') 'never claims qualification'
# A real Python 3, not the Microsoft Store placeholder under WindowsApps.
$python = @(Get-Command python, python3 -All -ErrorAction SilentlyContinue | ForEach-Object Source |
    Where-Object { $_ -notlike '*\WindowsApps\*' }) +
    @(Get-ChildItem (Join-Path $env:APPDATA 'uv\python') -Filter python.exe -Recurse -ErrorAction SilentlyContinue | ForEach-Object FullName) |
    Select-Object -First 1
if (-not $python) { throw 'Python 3 is required for the host review self-test (install it or uv python).' }
$review = Invoke-DriverVmProcess -Executable $python -Arguments @('-I','-B',(Join-Path $workspace 'tools\vm\review-cable-kill.py'),'--selftest') `
    -Stdout (Join-Path $root 'review.txt') -Stderr (Join-Path $root 'review-stderr.txt') -TimeoutSeconds 60
Assert ($review.Code -eq 0 -and (Get-Content -LiteralPath (Join-Path $root 'review.txt') -Raw) -match '4 cases pass') 'host review verdict logic (silence, late audio, old audio, no restart audio)'
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',$runner) `
        -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 10
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real runner refuses the host before any process starts'
}
Write-Host "$checks cable kill checks passed. No audio stream or driver tool was opened. Evidence: $root"
