# Pure helpers: no native calls, inventory or mutation when imported.
function Assert-DriverTestVmIdentity {
    param([string] $ComputerName, [bool] $MarkerPresent)
    if ($ComputerName -ne 'AR-DriverTest' -and -not $MarkerPresent) {
        throw 'Refusing driver smoke test outside AR-DriverTest (or C:\ar\IS_TEST_VM).'
    }
}
function Assert-DriverTestSigning {
    param([int] $ExitCode, [string] $BootEntry)
    if ($ExitCode -ne 0 -or $BootEntry -notmatch '(?im)^\s*testsigning\s+Yes\s*$') {
        throw 'Refusing driver smoke test: the VM current boot entry must have testsigning Yes.'
    }
}
function Compare-DriverVmBaseline {
    param($Before, $After)
    $differences = @()
    foreach ($category in @('drivers', 'devices', 'defaults')) {
        $left = @($Before.$category | Sort-Object)
        $right = @($After.$category | Sort-Object)
        if (($left -join "`n") -cne ($right -join "`n")) { $differences += $category }
    }
    return $differences
}
# Windows composes an endpoint's friendly name from the pin/jack description
# and the interface friendly name, e.g. "Speakers (AudioRouter Cable A Input)".
# Accept the exact cable name or either composed form; the runner records
# the actual names so the first VM run settles the format (17 §5.5).
function Test-CableEndpointName {
    param([string] $FriendlyName, [string] $Expected)
    if ([string]::IsNullOrEmpty($FriendlyName) -or [string]::IsNullOrEmpty($Expected)) { return $false }
    if ($FriendlyName -ceq $Expected) { return $true }
    $escaped = [regex]::Escape($Expected)
    return ($FriendlyName -cmatch "^[^()]+ \($escaped\)$") -or ($FriendlyName -ceq "$Expected (AudioRouter Virtual Cable)")
}

function Save-DriverEndpointSnapshot {
    param([object[]] $Endpoints, [string] $Path)
    # InputObject preserves an empty JSON array instead of writing an empty file.
    ConvertTo-Json -InputObject @($Endpoints) -Depth 4 -Compress |
        Set-Content -LiteralPath $Path -Encoding UTF8
}
function Select-CableEndpoints {
    param($Endpoints, [string[]] $Expected)
    $result = @()
    foreach ($name in $Expected) {
        $found = @($Endpoints | Where-Object { Test-CableEndpointName $_.FriendlyName $name })
        if ($found.Count -ne 1) { return $null }
        $result += $found[0]
    }
    return $result
}
