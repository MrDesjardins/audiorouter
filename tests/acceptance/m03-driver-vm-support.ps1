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
