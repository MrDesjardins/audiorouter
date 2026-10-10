# Helpers: no native calls, inventory or mutation when imported.
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

# Explicit invocation only; importing this file never starts a process.
function ConvertTo-DriverProcessArgument {
    param([AllowEmptyString()][string] $Value)
    # Windows argv quoting: double backslashes before quotes and at the end
    # of a quoted argument. Never use a shell or Invoke-Expression.
    $escaped = [regex]::Replace($Value, '(\\*)"', '$1$1\"')
    $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
    return '"' + $escaped + '"'
}

function Invoke-DriverVmProcess {
    param(
        [Parameter(Mandatory = $true)][string] $Executable,
        [string[]] $Arguments,
        [Parameter(Mandatory = $true)][string] $Stdout,
        [Parameter(Mandatory = $true)][string] $Stderr,
        [Parameter(Mandatory = $true)][ValidateRange(1, 3720)][int] $TimeoutSeconds
    )
    if ([IO.Path]::GetFullPath($Stdout) -ieq [IO.Path]::GetFullPath($Stderr)) {
        throw 'Process stdout and stderr need separate files.'
    }
    $quoted = @($Arguments | ForEach-Object { ConvertTo-DriverProcessArgument $_ }) -join ' '
    $process = $null
    $watch = [Diagnostics.Stopwatch]::StartNew()
    try {
        $process = Start-Process -FilePath $Executable -ArgumentList $quoted -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput $Stdout -RedirectStandardError $Stderr
        # Retain the native handle before waiting. Windows PowerShell 5.1's
        # Start-Process object can otherwise lose its exit code on completion.
        $null = $process.Handle
        $finished = $process.WaitForExit($TimeoutSeconds * 1000)
        if (-not $finished) {
            # Only this child is owned here. Never stop another tone instance,
            # the VM, audio services, or any process on the host.
            if (-not $process.HasExited) { $process.Kill() }
            if (-not $process.WaitForExit(5000)) { throw 'Timed-out child did not terminate; preserve evidence.' }
            return [pscustomobject]@{ Code = 124; TimedOut = $true; ProcessId = $process.Id; ElapsedSeconds = $watch.Elapsed.TotalSeconds }
        }
        return [pscustomobject]@{ Code = $process.ExitCode; TimedOut = $false; ProcessId = $process.Id; ElapsedSeconds = $watch.Elapsed.TotalSeconds }
    } finally {
        if ($process) {
            # Cancellation must not leave our child holding bridge leases.
            try {
                if (-not $process.HasExited) {
                    $process.Kill()
                    [void]$process.WaitForExit(5000)
                }
            } finally { $process.Dispose() }
        }
    }
}
