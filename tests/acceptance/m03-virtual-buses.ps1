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
        $output = & cargo run --quiet -p audiorouter-cli -- --json @Arguments 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "CLI command failed: $($Arguments -join ' '): $($output -join ' ')"
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
        & cargo run --quiet -p audiorouter-cli -- import (Join-Path $fixtureRoot 'valid-session.json') --database $database 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'failed to import the producer session fixture' }
        $duplicate = Invoke-CliJson @('session', 'duplicate', 'session-fixture', 'session-consumer', '--idempotency-key', 'duplicate-consumer', '--database', $database)
        if ($duplicate.session.id -ne 'session-consumer') { throw 'failed to create the consumer session fixture' }
        $operations = @(
                @{ Fixture = 'virtual-bus-create.json'; Key = 'create-desktop' },
                @{ Fixture = 'virtual-bus-voice-create.json'; Key = 'create-voice' },
                @{ Fixture = 'virtual-bus-recording-create.json'; Key = 'create-recording' },
                @{ Fixture = 'virtual-bus-monitor-create.json'; Key = 'create-monitor' },
                @{ Fixture = 'virtual-bus-chat-create.json'; Key = 'create-chat' },
                @{ Fixture = 'virtual-bus-music-create.json'; Key = 'create-music' },
                @{ Fixture = 'virtual-bus-aux-create.json'; Key = 'create-aux' },
                @{ Fixture = 'virtual-bus-test-create.json'; Key = 'create-test' }
        )
        foreach ($operation in $operations) {
            $document = Get-Content -LiteralPath (Join-Path $fixtureRoot $operation.Fixture) -Raw | ConvertFrom-Json
            $plan = Invoke-CliJson @('virtual-devices', 'plan', '--operation', (Join-Path $fixtureRoot $operation.Fixture), '--database', $database)
            if ($plan.operation.id -ne $document.id) { throw 'create plan returned the wrong bus identity' }
            $applied = Invoke-CliJson @('virtual-devices', 'apply', $plan.planId, '--idempotency-key', $operation.Key, '--database', $database)
            if ($applied.state -ne 'applied' -or $applied.operation.id -ne $document.id) { throw 'create apply returned the wrong lifecycle result' }
        }

        $devices = Invoke-CliJson @('virtual-devices', 'list', '--database', $database)
        if ($devices.Count -ne 8) { throw "expected eight managed buses, got $($devices.Count)" }
        foreach ($device in $devices) {
            if ($device.direction -ne 'bidirectional' -or $device.channels -ne 2 -or
                $device.availability.status -ne 'unavailable' -or
                $null -ne $device.driverInstanceId -or
                $null -ne $device.endpointIds.render -or $null -ne $device.endpointIds.capture) {
                throw "managed-driver boundary or endpoint identity is incorrect for $($device.id)"
            }
        }
        $overflowRejected = $false
        try {
            $null = Invoke-CliJson @(
                'virtual-devices', 'plan',
                '--operation', (Join-Path $fixtureRoot 'virtual-bus-overflow-create.json'),
                '--database', $database)
        } catch {
            if ($_.Exception.Message -match 'capacity|too many|LimitReached') {
                $overflowRejected = $true
            } else {
                throw
            }
        }
        if (-not $overflowRejected) { throw 'ninth managed bus was not rejected at the declared capacity' }

        $routeFile = Join-Path $fixtureRoot 'virtual-route-desktop.json'
        $routeApplied = Invoke-CliJson @(
            'virtual-routes', 'replace', '--base-revision', '0', '--file', $routeFile,
            '--idempotency-key', 'route-desktop', '--database', $database)
        if ($routeApplied.revision -ne 1 -or $routeApplied.routes.Count -ne 1) {
            throw 'explicit cross-session virtual route was not applied'
        }
        $routeListed = Invoke-CliJson @('virtual-routes', 'list', '--database', $database)
        if ($routeListed.revision -ne 1 -or $routeListed.routes.Count -ne 1 -or
            $routeListed.routes[0].busId -ne 'desktop-bus') {
            throw 'explicit virtual route was not persisted across CLI processes'
        }
        $cycleRejected = $false
        try {
            $null = Invoke-CliJson @(
                'virtual-routes', 'replace', '--base-revision', '1',
                '--file', (Join-Path $fixtureRoot 'virtual-route-cycle.json'),
                '--idempotency-key', 'route-cycle', '--database', $database)
        } catch {
            if ($_.Exception.Message -match 'Cycle|cycle') {
                $cycleRejected = $true
            } else {
                throw
            }
        }
        if (-not $cycleRejected) { throw 'cross-session virtual route cycle was not rejected' }
        $writerRejected = $false
        try {
            $null = Invoke-CliJson @(
                'virtual-routes', 'replace', '--base-revision', '1',
                '--file', (Join-Path $fixtureRoot 'virtual-route-conflicting-writers.json'),
                '--idempotency-key', 'route-writers', '--database', $database)
        } catch {
            if ($_.Exception.Message -match 'DuplicateVirtualBusWriter|writer') {
                $writerRejected = $true
            } else {
                throw
            }
        }
        if (-not $writerRejected) { throw 'multiple virtual-bus writers were not rejected' }
        $revisionRejected = $false
        try {
            $null = Invoke-CliJson @(
                'virtual-routes', 'replace', '--base-revision', '0',
                '--file', $routeFile, '--idempotency-key', 'route-stale-revision',
                '--database', $database)
        } catch {
            if ($_.Exception.Message -match 'baseRevision') {
                $revisionRejected = $true
            } else {
                throw
            }
        }
        if (-not $revisionRejected) { throw 'stale virtual-route revision was not rejected' }

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
        if ($remaining.Count -ne 7 -or $null -eq ($remaining | Where-Object id -eq 'desktop-bus') -or
            $null -eq ($remaining | Where-Object id -eq 'voice-bus')) {
            throw 'managed bus lifecycle did not retain seven buses after deleting one'
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
