$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'm03-driver-vm-support.ps1')
$script:count = 0
function Reject {
    param([scriptblock] $Operation)
    $rejected = $false
    try { & $Operation } catch { $rejected = $true }
    if (-not $rejected) { throw 'Unsafe guard combination accepted.' }
    $script:count++
}
Assert-DriverTestVmIdentity 'AR-DriverTest' $false
Assert-DriverTestVmIdentity 'VM-CUSTOM' $true
$script:count += 2
Reject { Assert-DriverTestVmIdentity 'DAILY-PC' $false }
Assert-DriverTestSigning 0 "testsigning    Yes`r`n"
$script:count++
foreach ($text in @('testsigning No', 'testsigning', 'testsigning Yesterday', 'description testsigning Yes', '')) {
    Reject { Assert-DriverTestSigning 0 $text }
}
Reject { Assert-DriverTestSigning 1 'testsigning Yes' }
$before = [pscustomobject]@{ drivers = @('d1', 'd2'); devices = @('p1', 'p2'); defaults = @('render=r1', 'capture=c1') }
$equal = [pscustomobject]@{ drivers = @('d2', 'd1'); devices = @('p2', 'p1'); defaults = @('capture=c1', 'render=r1') }
if (@(Compare-DriverVmBaseline $before $equal).Count -ne 0) { throw 'Snapshot ordering changed baseline result.' }
$script:count++
foreach ($category in @('drivers', 'devices', 'defaults')) {
    $after = [pscustomobject]@{ drivers = @('d1', 'd2'); devices = @('p1', 'p2'); defaults = @('render=r1', 'capture=c1') }
    $after.$category += 'unexpected'
    $result = @(Compare-DriverVmBaseline $before $after)
    if ($result.Count -ne 1 -or $result[0] -ne $category) { throw 'Baseline delta was missed.' }
    $script:count++
}
$parseErrors = $null
$tokens = $null
[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'm03-driver-vm.ps1'), [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($parseErrors.Count) { throw ($parseErrors -join "`n") }
$script:count++
Add-Type -Path (Join-Path $PSScriptRoot 'm03-default-endpoints.cs')
# A host-safe read-only COM check. Store only count, never private endpoint IDs.
if ([AudioRouterVmEvidence.DefaultEndpoints]::Read().Length -ne 6) { throw 'Default endpoint role snapshot is incomplete.' }
$script:count++
Write-Output "VM guards and read-only default endpoint snapshot passed: $script:count checks."
