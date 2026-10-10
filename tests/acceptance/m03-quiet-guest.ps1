<# Host-safe checks of tools\vm\quiet-guest.ps1 with an in-memory registry,
   fake Defender preferences and a fake CPU counter; plus the real host refusal. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\quiet-guest-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$script = Join-Path $workspace 'tools\vm\quiet-guest.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell syntax'
# Load the constants and functions only; never run the guarded entry block.
foreach ($node in $ast.EndBlock.Statements) {
    if ($node -is [Management.Automation.Language.FunctionDefinitionAst] -or
        ($node -is [Management.Automation.Language.AssignmentStatementAst] -and $node.Left.Extent.Text -match '^\$(UpdateKey|UpdateValues|MaintenanceKey|ExclusionPaths|ExclusionProcesses)$')) {
        . ([scriptblock]::Create($node.Extent.Text))
    }
}

function Reset-Fakes([hashtable] $Registry, [string[]] $Paths, [string[]] $Processes) {
    $script:registry = $Registry
    $script:paths = [Collections.Generic.List[string]]::new(); $Paths | ForEach-Object { $script:paths.Add($_) }
    $script:processes = [Collections.Generic.List[string]]::new(); $Processes | ForEach-Object { $script:processes.Add($_) }
    $script:signatureUpdates = 0
}
function Get-ItemProperty { param($Path, $Name, $ErrorAction)
    $key = "$Path|$Name"
    if ($script:registry.ContainsKey($key)) { return [pscustomobject]@{ $Name = $script:registry[$key] } }
    return $null }
function Set-ItemProperty { param($Path, $Name, $Value, $Type)
    Assert ($Path -like 'HKLM:\SOFTWARE\*') 'writes only the documented HKLM keys'
    $script:registry["$Path|$Name"] = $Value }
function Remove-ItemProperty { param($Path, $Name, $ErrorAction) $script:registry.Remove("$Path|$Name") }
function New-Item { param($Path, [switch] $Force) Assert ($Path -like 'HKLM:\SOFTWARE\*') 'creates only the documented keys' }
function Test-Path { param($LiteralPath, $PathType)
    if ($LiteralPath -like 'HKLM:*') { return $true }
    return [IO.File]::Exists($LiteralPath) }
function Get-MpPreference { [pscustomobject]@{ ExclusionPath = @($script:paths); ExclusionProcess = @($script:processes) } }
function Add-MpPreference { param([string[]] $ExclusionPath, [string[]] $ExclusionProcess)
    $ExclusionPath | Where-Object { $_ } | ForEach-Object { Assert (-not $script:paths.Contains($_)) 'never adds a duplicate path'; $script:paths.Add($_) }
    $ExclusionProcess | Where-Object { $_ } | ForEach-Object { Assert (-not $script:processes.Contains($_)) 'never adds a duplicate process'; $script:processes.Add($_) } }
function Remove-MpPreference { param([string[]] $ExclusionPath, [string[]] $ExclusionProcess)
    $ExclusionPath | Where-Object { $_ } | ForEach-Object { [void]$script:paths.Remove($_) }
    $ExclusionProcess | Where-Object { $_ } | ForEach-Object { [void]$script:processes.Remove($_) } }
function Update-MpSignature { $script:signatureUpdates++ }

$StatePath = Join-Path $root 'state.json'
$now = [DateTime]::new(2026, 10, 10, 20, 0, 0, [DateTimeKind]::Utc)

# 1. Clean guest: apply, refuse a second apply, revert to exactly nothing.
Reset-Fakes @{} @() @()
Invoke-QuietApply $now
Assert ($script:registry["$UpdateKey|PauseUpdatesExpiryTime"] -eq '2026-10-17T20:00:00Z') 'Windows Update paused for seven days'
Assert ($script:registry["$UpdateKey|PauseQualityUpdatesStartTime"] -eq '2026-10-10T20:00:00Z') 'pause start recorded'
Assert ($script:registry["$MaintenanceKey|MaintenanceDisabled"] -eq 1) 'automatic maintenance disabled'
Assert ($script:paths.Contains('C:\ar') -and $script:processes.Count -eq $ExclusionProcesses.Count) 'test path and tools excluded'
Assert ($script:signatureUpdates -eq 1) 'signatures fetched before measuring'
Assert ([IO.File]::Exists($StatePath)) 'previous values recorded before changing anything'
$second = $false
try { Invoke-QuietApply $now } catch { $second = [string]$_ -match 'Already applied' }
Assert $second 'a second apply is refused instead of overwriting the recorded state'
Invoke-QuietRevert
Assert ($script:registry.Count -eq 0) 'revert removes every value it created'
Assert ($script:paths.Count -eq 0 -and $script:processes.Count -eq 0) 'revert removes the exclusions it added'
Assert (-not [IO.File]::Exists($StatePath)) 'state file removed after revert'
$nothing = $false
try { Invoke-QuietRevert } catch { $nothing = [string]$_ -match 'Nothing to revert' }
Assert $nothing 'revert without a recorded state is refused'

# 2. Guest with prior settings: they survive apply + revert unchanged.
Reset-Fakes @{ "$MaintenanceKey|MaintenanceDisabled" = 0; "$UpdateKey|PauseUpdatesExpiryTime" = '2030-01-01T00:00:00Z' } @('C:\ar', 'D:\other') @('other.exe')
Invoke-QuietApply $now
Assert ($script:paths.Count -eq 2 -and $script:processes.Count -eq 1 + $ExclusionProcesses.Count) 'existing exclusions kept, only missing ones added'
Invoke-QuietRevert
Assert ($script:registry["$MaintenanceKey|MaintenanceDisabled"] -eq 0) 'previous maintenance value restored'
Assert ($script:registry["$UpdateKey|PauseUpdatesExpiryTime"] -eq '2030-01-01T00:00:00Z') 'previous pause restored'
Assert (-not $script:registry.ContainsKey("$UpdateKey|PauseFeatureUpdatesEndTime")) 'values that did not exist are removed again'
Assert (($script:paths -join ',') -eq 'C:\ar,D:\other' -and ($script:processes -join ',') -eq 'other.exe') 'pre-existing exclusions untouched'

# 3. Settle wait: quiet after a burst returns true; a busy guest times out.
$script:samples = [Collections.Generic.Queue[double]]::new()
@(55, 40, 12) + @(1..12 | ForEach-Object { 3 }) | ForEach-Object { $script:samples.Enqueue($_) }
function Get-Counter { param($Counter, $SampleInterval, $MaxSamples)
    $value = if ($script:samples.Count) { $script:samples.Dequeue() } else { 90 }
    [pscustomobject]@{ CounterSamples = @([pscustomobject]@{ CookedValue = $value }) } }
Assert (Wait-QuietCpu 30) 'sixty quiet seconds after a burst are accepted'
Assert ($script:samples.Count -eq 0) 'quiet detection needs twelve consecutive quiet samples'
Assert (-not (Wait-QuietCpu 1)) 'a guest that never settles times out'
Assert (Wait-QuietCpu 0) 'settling can be skipped explicitly'

# 4. The real script refuses the host before touching anything.
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $copy = Join-Path $root 'bundle'
    [void][IO.Directory]::CreateDirectory((Join-Path $copy 'repo\tests\acceptance'))
    [IO.File]::Copy($script, (Join-Path $copy 'quiet-guest.ps1'))
    [IO.File]::Copy((Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1'), (Join-Path $copy 'repo\tests\acceptance\m03-driver-vm-support.ps1'))
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',(Join-Path $copy 'quiet-guest.ps1'),'-Status') `
        -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 20
    Assert ($guard.Code -eq 1 -and ([IO.File]::ReadAllText((Join-Path $root 'guard-stderr.txt'))) -match 'outside AR-DriverTest') 'real script refuses the host'
}
Write-Host "$checks quiet-guest checks passed. No registry, Defender or host setting was changed. Evidence: $root"
