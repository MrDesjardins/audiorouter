param(
    [switch]$AllowLiveAudio,
    [Parameter(Mandatory = $true)][string]$CaptureEndpointId,
    [Parameter(Mandatory = $true)][string]$RenderEndpointId,
    [ValidateRange(1, 30)][int]$DurationSeconds = 10
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Pass -AllowLiveAudio to run this guarded native playback check.' }

function Invoke-Rpc([string]$Method, [object]$Params = $null) {
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'audiorouter-control', [System.IO.Pipes.PipeDirection]::InOut)
    try {
        $pipe.Connect(3000)
        $request = @{ jsonrpc = '2.0'; id = 1; method = $Method }
        if ($null -ne $Params) { $request.params = $Params }
        $payload = [System.Text.Encoding]::UTF8.GetBytes(($request | ConvertTo-Json -Compress -Depth 40))
        if ($payload.Length -gt 4194304) { throw 'Request exceeds frame limit.' }
        $length = [BitConverter]::GetBytes([uint32]$payload.Length)
        $pipe.Write($length, 0, 4)
        $pipe.Write($payload, 0, $payload.Length)
        $pipe.Flush()
        $header = New-Object byte[] 4
        $offset = 0
        while ($offset -lt 4) {
            $count = $pipe.Read($header, $offset, 4 - $offset)
            if ($count -eq 0) { throw 'Control pipe closed before the response header.' }
            $offset += $count
        }
        $size = [BitConverter]::ToUInt32($header, 0)
        if ($size -gt 4194304) { throw 'Response exceeds frame limit.' }
        $body = New-Object byte[] $size
        $offset = 0
        while ($offset -lt $size) {
            $count = $pipe.Read($body, $offset, $size - $offset)
            if ($count -eq 0) { throw 'Control pipe closed before the full response.' }
            $offset += $count
        }
        $response = [System.Text.Encoding]::UTF8.GetString($body) | ConvertFrom-Json
        if ($response.error) { throw "$Method failed: $($response.error.message) [$($response.error.code)]" }
        return $response.result
    } finally {
        $pipe.Dispose()
    }
}

$before = Invoke-Rpc 'status.get'
if ($before.activeSessionCount -ne 0) { throw 'Another session is active; live check refused.' }
$attached = Invoke-Rpc 'system.diagnostics'
if ($attached.nativeSessionId) {
    if ($attached.nativeSessionId -notlike 'm05-signal-*' -or $attached.nativeAdapter -ne 'configured-stopped') {
        throw 'A native worker is attached outside this test; live check refused.'
    }
    [void](Invoke-Rpc 'nativeEndpoints.detach' @{ sessionId = $attached.nativeSessionId })
}
$devicesBefore = @(Invoke-Rpc 'devices.list')
$capture = $devicesBefore | Where-Object { $_.id -eq $CaptureEndpointId -and $_.direction -eq 'capture' -and $_.state -eq 'active' }
$render = $devicesBefore | Where-Object { $_.id -eq $RenderEndpointId -and $_.direction -eq 'render' -and $_.state -eq 'active' }
if (@($capture).Count -ne 1 -or @($render).Count -ne 1) { throw 'Exact active capture/render endpoint pair was not found.' }

$sessionId = 'm05-signal-' + [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$session = @{
    id = $sessionId; name = '10 second Test Signal to Gain to Output'; schemaVersion = 1; revision = 0
    nodes = @(
        @{ id = 'test-signal'; kind = 'testSignal'; name = 'Test Signal'; enabled = $true; bypass = $false; parameters = @{ frequencyHz = 440; levelDb = -18; durationMs = $DurationSeconds * 1000 }; ports = @(@{ name = 'out'; direction = 'output'; channels = 2 }) },
        @{ id = 'gain'; kind = 'gain'; name = 'Gain'; enabled = $true; bypass = $false; parameters = @{ gainDb = 0 }; ports = @(@{ name = 'in'; direction = 'input'; channels = 2 }, @{ name = 'out'; direction = 'output'; channels = 2 }) },
        @{ id = 'physical-output'; kind = 'physicalOutput'; name = 'Physical output'; enabled = $true; bypass = $false; parameters = @{}; ports = @(@{ name = 'in'; direction = 'input'; channels = 2 }) }
    )
    edges = @(
        @{ id = 'signal-to-gain'; sourceNode = 'test-signal'; sourcePort = 'out'; destinationNode = 'gain'; destinationPort = 'in'; matrix = @(1.0, 0.0, 0.0, 1.0); enabled = $true },
        @{ id = 'gain-to-output'; sourceNode = 'gain'; sourcePort = 'out'; destinationNode = 'physical-output'; destinationPort = 'in'; matrix = @(1.0, 0.0, 0.0, 1.0); enabled = $true }
    )
}
$created = Invoke-Rpc 'sessions.create' @{ session = $session; idempotencyKey = "m05-create-$sessionId" }
if ($created.session.id -ne $sessionId) { throw 'Created session identity mismatch.' }
Write-Output "Session: $sessionId"
Write-Output "Capture: $($capture.name)"
Write-Output "Render: $($render.name)"

$started = $false
$prepared = $false
$completed = $false
try {
    $preparation = Invoke-Rpc 'nativeEndpoints.prepare' @{ sessionId = $sessionId; captureEndpointId = $CaptureEndpointId; renderEndpointId = $RenderEndpointId }
    if ($preparation.state -ne 'configured-stopped') { throw "Unexpected preparation state: $($preparation.state)" }
    $prepared = $true
    $running = Invoke-Rpc 'sessions.start' @{ sessionId = $sessionId; idempotencyKey = "m05-start-$sessionId" }
    if ($running.runtime -ne 'native') { throw "Expected native runtime; received $($running.runtime)." }
    $started = $true
    $transport = Invoke-Rpc 'audioSources.transport' @{ sessionId = $sessionId; nodeId = 'test-signal'; action = 'play' }
    if ($transport.state -ne 'playing') { throw "Expected playing Test Signal; received $($transport.state)." }
    $pumps = 0
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    while ($clock.Elapsed.TotalSeconds -lt $DurationSeconds) {
        [void](Invoke-Rpc 'nativeEndpoints.pump' @{ sessionId = $sessionId; generation = $running.generation; maxPackets = 64 })
        $pumps++
        Start-Sleep -Milliseconds 12
    }
    $diagnostics = Invoke-Rpc 'system.diagnostics'
    Write-Output "Native generation: $($running.generation)"
    Write-Output "Pump calls: $pumps"
    Write-Output "Processed quanta: $($diagnostics.schedulerTelemetry.processedQuanta)"
    Write-Output "XRuns: $($diagnostics.schedulerTelemetry.xruns)"
    Write-Output "Elapsed seconds: $([Math]::Round($clock.Elapsed.TotalSeconds, 2))"
    $completed = $true
} finally {
    try {
        if ($started) {
            $stopped = Invoke-Rpc 'sessions.stop' @{ sessionId = $sessionId; idempotencyKey = "m05-stop-$sessionId" }
            Write-Output "Final state: $($stopped.state)"
        }
    } finally {
        try {
            if ($prepared) { [void](Invoke-Rpc 'nativeEndpoints.detach' @{ sessionId = $sessionId }) }
        } finally {
            if (-not $completed) {
                [void](Invoke-Rpc 'sessions.delete' @{ sessionId = $sessionId; idempotencyKey = "m05-failed-$sessionId" })
            }
        }
    }
}
$after = Invoke-Rpc 'status.get'
if ($after.activeSessionCount -ne 0) { throw 'The live check left an active session.' }
$devicesAfter = @(Invoke-Rpc 'devices.list')
foreach ($endpoint in @($capture, $render)) {
    $match = $devicesAfter | Where-Object { $_.id -eq $endpoint.id }
    if (@($match).Count -ne 1 -or $match.state -ne $endpoint.state) { throw "Endpoint changed state during check: $($endpoint.name)" }
}
Write-Output 'Exact endpoint states were unchanged.'
Write-Output "Saved test session: $sessionId"
