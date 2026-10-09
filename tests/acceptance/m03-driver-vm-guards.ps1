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
$vmRunnerSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'm03-driver-vm.ps1') -Raw
if ($vmRunnerSource.IndexOf('endpoint-names-on-install-failure.json', [StringComparison]::Ordinal) -lt 0 -or
    $vmRunnerSource.IndexOf('endpoint-names-on-install-failure.json', [StringComparison]::Ordinal) -lt $vmRunnerSource.IndexOf('Invoke-DriverTool $Helper', [StringComparison]::Ordinal)) {
    throw 'A failed helper install must save the raw endpoint names before cleanup.'
}
$script:count++
$vmChecksPath = Join-Path $PSScriptRoot '../../tools/vm/vm-checks.ps1'
if (-not (Test-Path -LiteralPath $vmChecksPath)) {
    $vmChecksPath = Join-Path $PSScriptRoot '../../../vm-checks.ps1'
}
$vmChecksSource = Get-Content -LiteralPath $vmChecksPath -Raw
$parseErrors = $null
$tokens = $null
[Management.Automation.Language.Parser]::ParseFile($vmChecksPath, [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($parseErrors.Count) { throw ($parseErrors -join "`n") }
$stopTranscriptIndex = $vmChecksSource.LastIndexOf('Stop-Transcript', [StringComparison]::Ordinal)
$archiveIndex = $vmChecksSource.IndexOf('Compress-Archive -Path (Join-Path $evidenceRoot', [StringComparison]::Ordinal)
if ($stopTranscriptIndex -lt 0 -or $archiveIndex -le $stopTranscriptIndex) {
    throw 'Evidence collection must close its transcript before compressing the evidence tree.'
}
$script:count += 2
$retryPath = Join-Path $PSScriptRoot '../../tools/vm/retry-smoke.ps1'
if (-not (Test-Path -LiteralPath $retryPath)) { $retryPath = Join-Path $PSScriptRoot '../../../retry-smoke.ps1' }
$parseErrors = $null
$tokens = $null
[Management.Automation.Language.Parser]::ParseFile($retryPath, [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($parseErrors.Count) { throw ($parseErrors -join "`n") }
$retrySource = Get-Content -LiteralPath $retryPath -Raw
if ($retrySource.IndexOf('Assert-DriverTestVmIdentity') -gt $retrySource.IndexOf('& robocopy.exe') -or
    $retrySource.IndexOf('Get-FileHash') -gt $retrySource.IndexOf('-Step preflight') -or
    $retrySource.IndexOf("throw 'Preflight failed; smoke was not started.'") -gt $retrySource.IndexOf('-Step smoke') -or
    $retrySource.IndexOf('-Step collect') -gt $retrySource.IndexOf('if ($smokeExit -ne 0)')) {
    throw 'Retry must refuse the host, verify the copy, stop on preflight failure, and collect smoke failures.'
}
$script:count += 2
$nameFixtures = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'm03-endpoint-names.json') -Raw | ConvertFrom-Json
foreach ($fixture in $nameFixtures) {
    foreach ($cable in 0..7) {
        foreach ($direction in @('input', 'output')) {
            $label = if ($direction -eq 'input') { 'Input' } else { 'Output' }
            $expectedName = "AudioRouter Cable $([char](65 + $cable)) $label"
            $expectedMatch = $null -ne $fixture.expected -and $fixture.expected[0] -eq $cable -and $fixture.expected[1] -ceq $direction
            if ((Test-CableEndpointName $fixture.friendly $expectedName) -ne $expectedMatch) {
                throw "Rust/PowerShell endpoint-name fixture mismatch: $($fixture.friendly) / $expectedName"
            }
            $script:count++
        }
    }
}
$emptySnapshotPath = Join-Path ([IO.Path]::GetTempPath()) ('ar-empty-endpoints-' + [guid]::NewGuid().ToString('N') + '.json')
try {
    Save-DriverEndpointSnapshot @() $emptySnapshotPath
    if ((Get-Content -LiteralPath $emptySnapshotPath -Raw).Trim() -ne '[]') { throw 'Empty endpoint evidence must be a JSON array.' }
    $script:count++
} finally {
    Remove-Item -LiteralPath $emptySnapshotPath -ErrorAction SilentlyContinue
}
foreach ($accepted in @('AudioRouter Cable A Input', 'Speakers (AudioRouter Cable A Input)',
        'AudioRouter Cable A Input (AudioRouter Virtual Cable)')) {
    if (-not (Test-CableEndpointName $accepted 'AudioRouter Cable A Input')) { throw "Cable endpoint name rejected: $accepted" }
    $script:count++
}
foreach ($rejected in @('AudioRouter Cable A Output', 'Speakers (AudioRouter Cable A Output)',
        'AudioRouter Cable A Input 2', 'audiorouter cable a input', 'Speakers (Realtek)', '', 'X (Y) (AudioRouter Cable A Input)')) {
    if (Test-CableEndpointName $rejected 'AudioRouter Cable A Input') { throw "Unrelated endpoint name accepted: $rejected" }
    $script:count++
}
$sample = @(
    [pscustomobject]@{ FriendlyName = 'Speakers (AudioRouter Cable A Input)'; InstanceId = 'a-in' },
    [pscustomobject]@{ FriendlyName = 'Line (AudioRouter Cable A Output)'; InstanceId = 'a-out' })
$selected = Select-CableEndpoints $sample @('AudioRouter Cable A Input', 'AudioRouter Cable A Output')
if ($null -eq $selected -or $selected[0].InstanceId -ne 'a-in' -or $selected[1].InstanceId -ne 'a-out') { throw 'Cable endpoints not selected in order.' }
if ($null -ne (Select-CableEndpoints $sample @('AudioRouter Cable B Input'))) { throw 'Missing cable endpoint was not reported.' }
$script:count += 2
Add-Type -Path (Join-Path $PSScriptRoot 'm03-default-endpoints.cs')
# A host-safe read-only COM check. Store only count, never private endpoint IDs.
if ([AudioRouterVmEvidence.DefaultEndpoints]::Read().Length -ne 6) { throw 'Default endpoint role snapshot is incomplete.' }
$script:count++
foreach ($endpoint in [AudioRouterVmEvidence.EndpointStates]::Read()) {
    if ([string]::IsNullOrEmpty($endpoint.Id) -or $endpoint.Flow -notin 0,1 -or
        $endpoint.State -eq 0 -or ($endpoint.State -band 0xFFFFFFF0) -ne 0) {
        throw 'All-state endpoint snapshot returned invalid SDK metadata.'
    }
}
$script:count++
Write-Output "VM guards and read-only default endpoint snapshot passed: $script:count checks."
