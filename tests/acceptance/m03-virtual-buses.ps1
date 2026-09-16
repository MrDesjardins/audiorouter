[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $repositoryRoot
try {
    $suffix = "audiorouter-m03-buses-$PID"
    $database = Join-Path ([IO.Path]::GetTempPath()) "$suffix.sqlite"
    $fixtureRoot = Join-Path $repositoryRoot 'tests\fixtures'
    Remove-Item -LiteralPath $database -Force -ErrorAction SilentlyContinue

    function Invoke-CliJson([string[]] $Arguments) {
        $output = & cargo run --quiet -p audiorouter-cli -- --json @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "CLI command failed: $($Arguments -join ' ')"
        }
        return ($output -join [Environment]::NewLine | ConvertFrom-Json)
    }

    function Apply-Operation([string] $Fixture, [string] $Key) {
        $plan = Invoke-CliJson @(
            'virtual-devices', 'plan',
            '--operation', (Join-Path $fixtureRoot $Fixture),
            '--database', $database)
        if ($plan.availability.status -ne 'unavailable' -or
            $plan.requiredScopes -notcontains 'deviceAdministration') {
            throw "virtual-device plan did not report the managed-driver boundary"
        }
        return Invoke-CliJson @(
            'virtual-devices', 'apply', $plan.planId,
            '--idempotency-key', $Key,
            '--database', $database)
    }

    try {
        $operations = @(
                @{ Fixture = 'virtual-bus-create.json'; Key = 'create-desktop' },
                @{ Fixture = 'virtual-bus-voice-create.json'; Key = 'create-voice' },
                @{ Fixture = 'virtual-bus-recording-create.json'; Key = 'create-recording' }
        )
        foreach ($operation in $operations) {
            $document = Get-Content -LiteralPath (Join-Path $fixtureRoot $operation.Fixture) -Raw | ConvertFrom-Json
            $plan = Invoke-CliJson @('virtual-devices', 'plan', '--operation', (Join-Path $fixtureRoot $operation.Fixture), '--database', $database)
            if ($plan.operation.id -ne $document.id) { throw 'create plan returned the wrong bus identity' }
            $applied = Invoke-CliJson @('virtual-devices', 'apply', $plan.planId, '--idempotency-key', $operation.Key, '--database', $database)
            if ($applied.state -ne 'applied' -or $applied.operation.id -ne $document.id) { throw 'create apply returned the wrong lifecycle result' }
        }

        $devices = Invoke-CliJson @('virtual-devices', 'list', '--database', $database)
        if ($devices.Count -ne 3) { throw "expected three managed buses, got $($devices.Count)" }
        foreach ($device in $devices) {
            if ($device.direction -ne 'bidirectional' -or $device.channels -ne 2 -or
                $device.availability.status -ne 'unavailable' -or
                $null -ne $device.driverInstanceId -or
                $null -ne $device.endpointIds.render -or $null -ne $device.endpointIds.capture) {
                throw "managed-driver boundary or endpoint identity is incorrect for $($device.id)"
            }
        }

        $renamed = Apply-Operation 'virtual-bus-rename.json' 'rename-desktop'
        if ($renamed.operation.name -ne 'Desktop Monitor') { throw 'rename lifecycle result was not persisted' }
        $disabled = Apply-Operation 'virtual-bus-disable.json' 'disable-voice'
        if ($disabled.operation.enabled -ne $false) { throw 'disable lifecycle result was not persisted' }
        $enabled = Apply-Operation 'virtual-bus-enable.json' 'enable-voice'
        if ($enabled.operation.enabled -ne $true) { throw 'enable lifecycle result was not persisted' }
        $recordingDisabled = Apply-Operation 'virtual-bus-recording-disable.json' 'disable-recording'
        if ($recordingDisabled.operation.enabled -ne $false) { throw 'recording bus disable was not persisted' }
        $deleted = Apply-Operation 'virtual-bus-delete.json' 'delete-recording'
        if ($deleted.operation.id -ne 'recording-bus') { throw 'delete lifecycle result was not returned' }

        $remaining = Invoke-CliJson @('virtual-devices', 'list', '--database', $database)
        if ($remaining.Count -ne 2 -or $null -eq ($remaining | Where-Object id -eq 'desktop-bus') -or
            $null -eq ($remaining | Where-Object id -eq 'voice-bus')) {
            throw 'managed bus lifecycle did not retain the expected two buses'
        }
        if (($remaining | Where-Object id -eq 'desktop-bus').name -ne 'Desktop Monitor') {
            throw 'renamed bus was not retained after reopening the CLI database'
        }
        Write-Output 'M03 virtual-bus CLI acceptance passed'
        Write-Output 'Scope: desired-state and persistence lifecycle only; no native device provisioning, installation, loading, or audio configuration action.'
    } finally {
        Remove-Item -LiteralPath $database -Force -ErrorAction SilentlyContinue
    }
}
finally {
    Pop-Location
}
