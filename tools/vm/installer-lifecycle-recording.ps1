# Runs only inside the isolated lifecycle guest. Use the installed shell's
# desktop RPC bridge so folder approval and recording exercise production code.
function Read-QualificationRpc([scriptblock]$Receive, [Threading.CancellationToken]$Token, [string]$Method, [string]$RequestId) {
    $buffer = [byte[]]::new(65536)
    do {
        $stream = [IO.MemoryStream]::new()
        try {
            do {
                $Token.ThrowIfCancellationRequested()
                $received = & $Receive $buffer $Token
                if ($received.MessageType -ne [Net.WebSockets.WebSocketMessageType]::Text) { throw 'Expected a WebView text frame' }
                $stream.Write($buffer, 0, $received.Count)
                if ($stream.Length -gt 1048576) { throw 'WebView response exceeded fixture limit' }
            } until ($received.EndOfMessage)
            $reply = [Text.Encoding]::UTF8.GetString($stream.ToArray()) | ConvertFrom-Json
        } finally { $stream.Dispose() }
    } until ($reply.id -eq 1)
    $rpc = $reply.result.result.value
    if ($reply.error -or $reply.result.exceptionDetails -or -not $rpc -or $rpc.error -or $rpc.id -ne $RequestId) {
        throw "Installed desktop RPC $Method failed: $($rpc.error.message)"
    }
    return $rpc.result
}

function Complete-QualificationCleanup([scriptblock[]]$Actions, $PrimaryError) {
    $first = $null
    foreach ($action in $Actions) {
        try { & $action } catch {
            if (-not $first) { $first = $_ }
            Write-Warning ('Recorder cleanup failed: ' + $_.Exception.Message) -WarningAction Continue
        }
    }
    if ($first -and -not $PrimaryError) { throw $first }
}

function New-QualificationRecording([string]$Shell, [string]$Root, [switch]$ProbeOnly) {
    $profile = Join-Path $Root 'webview'
    $oldProfile = $env:WEBVIEW2_USER_DATA_FOLDER
    $oldArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
    $oldDatabase = $env:AUDIOROUTER_DATABASE
    $socket = $null
    $deadline = $null
    $shellProcess = $null
    $primaryError = $null
    try {
        $env:WEBVIEW2_USER_DATA_FOLDER = $profile
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0 --remote-debugging-address=127.0.0.1'
        if ($ProbeOnly) { $env:AUDIOROUTER_DATABASE = Join-Path $Root 'offline-shell-probe.sqlite' }
        $socket = [Net.WebSockets.ClientWebSocket]::new()
        $deadline = [Threading.CancellationTokenSource]::new(45000)
        $shellProcess = Start-Process -FilePath $Shell -WindowStyle Hidden -PassThru
        $portFile = Join-Path $profile 'EBWebView\DevToolsActivePort'
        $target = $null
        while (-not $deadline.IsCancellationRequested -and -not $shellProcess.HasExited) {
            if (Test-Path -LiteralPath $portFile) {
                $port = Get-Content -LiteralPath $portFile -TotalCount 1
                if ($port -match '^\d+$') {
                    try {
                        $pages = Invoke-RestMethod -Uri "http://127.0.0.1:$port/json/list" -TimeoutSec 2
                        $target = $pages | Where-Object { $_.type -eq 'page' -and $_.url -match '^https?://tauri\.localhost(?:/|$)' } | Select-Object -First 1
                    } catch {}
                    if ($target) { break }
                }
            }
            Start-Sleep -Milliseconds 250
        }
        if (-not $target) { throw 'Installed shell did not expose its local WebView within the fixture deadline' }
        $ws = [uri]$target.webSocketDebuggerUrl
        if ($ws.Scheme -ne 'ws' -or $ws.Host -ne '127.0.0.1' -or $ws.Port -ne [int]$port) {
            throw 'Unexpected WebView debugging address'
        }
        $null = $socket.ConnectAsync($ws, $deadline.Token).GetAwaiter().GetResult()
        function DesktopRpc([string]$Method, [hashtable]$Params) {
            $requestId = [guid]::NewGuid().ToString()
            $request = @{ jsonrpc = '2.0'; id = $requestId; method = $Method; params = $Params } | ConvertTo-Json -Depth 20 -Compress
            $message = @{ id = 1; method = 'Runtime.evaluate'; params = @{
                expression = "window.__AUDIO_ROUTER_HOST__.transport.send($request)"
                awaitPromise = $true
                returnByValue = $true
            }} | ConvertTo-Json -Depth 8 -Compress
            $bytes = [Text.Encoding]::UTF8.GetBytes($message)
            $null = $socket.SendAsync([ArraySegment[byte]]::new($bytes), [Net.WebSockets.WebSocketMessageType]::Text, $true, $deadline.Token).GetAwaiter().GetResult()
            Read-QualificationRpc {
                param($buffer, $token)
                $socket.ReceiveAsync([ArraySegment[byte]]::new($buffer), $token).GetAwaiter().GetResult()
            } $deadline.Token $Method $requestId
        }
        $description = DesktopRpc 'system.describe' @{}
        if ($description.protocolVersion.major -ne 1) { throw 'Installed shell returned an unexpected protocol version' }
        if ($ProbeOnly) { return 'usable' }
        $access = DesktopRpc 'devices.getAccess' @{}
        if ($access.allowed -ne $false) { throw 'Fresh shell unexpectedly grants audio device access' }
        # The window asks once before Play opens devices. Give that consent through
        # the same window-only method the dialog calls, then confirm it stuck.
        $null = DesktopRpc 'devices.setAccess' @{ allowed = $true; idempotencyKey = 'qualification-device-consent' }
        $granted = DesktopRpc 'devices.getAccess' @{}
        if ($granted.allowed -ne $true) { throw 'Window consent did not enable device access' }
        $summary = DesktopRpc 'sessions.summary' @{ sessionId = 'qualification-recorder' }
        if ($summary.playing -ne $false) { throw 'Recorder session did not start stopped' }
        $recordings = Join-Path $Root 'recordings'
        $null = DesktopRpc 'recordings.setRoot' @{ root = $recordings; create = $true; idempotencyKey = 'qualification-root' }
        $null = DesktopRpc 'sessions.play' @{ sessionId = 'qualification-recorder'; idempotencyKey = 'qualification-play' }
        # A Test Signal stays silent until it is played, like the Tone button in the app.
        $tone = DesktopRpc 'audioSources.transport' @{ sessionId = 'qualification-recorder'; nodeId = 'tone'; action = 'play' }
        if ($tone.state -ne 'playing') { throw 'Generated tone did not start' }
        $null = DesktopRpc 'recorders.startRecording' @{ sessionId = 'qualification-recorder'; nodeId = 'recorder'; idempotencyKey = 'qualification-start' }
        Start-Sleep -Seconds 2
        $null = DesktopRpc 'recorders.stopRecording' @{ sessionId = 'qualification-recorder'; nodeId = 'recorder'; idempotencyKey = 'qualification-finalize' }
        $null = DesktopRpc 'sessions.stop' @{ sessionId = 'qualification-recorder'; idempotencyKey = 'qualification-stop' }
        $file = @(Get-ChildItem -LiteralPath $recordings -Filter '*.wav' -File)
        if ($file.Count -ne 1) { throw 'Expected exactly one finalized production WAV' }
        return $file[0].FullName
    } catch {
        $primaryError = $_
        throw
    } finally {
        try {
            Complete-QualificationCleanup @(
                { if ($socket) { $socket.Dispose() } },
                { if ($deadline) { $deadline.Dispose() } },
                {
                    if ($shellProcess -and -not $shellProcess.HasExited) {
                        $null = $shellProcess.CloseMainWindow()
                        if (-not $shellProcess.WaitForExit(5000)) {
                            $shellProcess.Kill()
                            if (-not $shellProcess.WaitForExit(5000)) { throw 'Installed shell could not be stopped' }
                        }
                    }
                },
                { if ($shellProcess) { $shellProcess.Dispose() } }
            ) $primaryError
        } finally {
            try { $env:WEBVIEW2_USER_DATA_FOLDER = $oldProfile }
            finally {
                try { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldArguments }
                finally { $env:AUDIOROUTER_DATABASE = $oldDatabase }
            }
        }
    }
}

function Measure-QualificationWav([string]$Path) {
    if ((Get-Item -LiteralPath $Path).Length -gt 4194304) { throw 'WAV exceeds the fixture size limit' }
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 44 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 4) -ne 'RIFF' -or
        [Text.Encoding]::ASCII.GetString($bytes, 8, 4) -ne 'WAVE' -or
        [BitConverter]::ToUInt32($bytes, 4) + 8 -ne $bytes.Length) { throw 'Invalid RIFF/WAVE envelope' }
    $format = $null
    $data = $null
    for ($offset = 12; $offset + 8 -le $bytes.Length; $offset = $end + ($size % 2)) {
        $id = [Text.Encoding]::ASCII.GetString($bytes, $offset, 4)
        $size = [BitConverter]::ToUInt32($bytes, $offset + 4)
        $body = $offset + 8
        $end = $body + $size
        if ($end -gt $bytes.Length) { throw 'Truncated WAV chunk' }
        if ($id -eq 'fmt ' -and $size -ge 16) { $format = $body }
        if ($id -eq 'data') { $data = @{ start = $body; size = $size } }
    }
    if ($null -eq $format -or -not $data) { throw 'Missing WAV format or data' }
    $rate = [BitConverter]::ToUInt32($bytes, $format + 4)
    # A Recorder branch is stereo, so accept mono or stereo and judge the first channel.
    $channels = [int][BitConverter]::ToUInt16($bytes, $format + 2)
    $blockAlign = 2 * $channels
    if ([BitConverter]::ToUInt16($bytes, $format) -ne 1 -or
        ($channels -ne 1 -and $channels -ne 2) -or
        [BitConverter]::ToUInt16($bytes, $format + 12) -ne $blockAlign -or
        [BitConverter]::ToUInt16($bytes, $format + 14) -ne 16 -or
        $rate -ne 48000 -or $data.size % $blockAlign) { throw 'Expected 48 kHz mono or stereo PCM16 WAV' }
    $frames = $data.size / $blockAlign
    if ($frames -lt 1.5 * $rate -or $frames -gt 12 * $rate) { throw 'Recording duration is outside the expected range' }
    $positive = 0
    $negative = 0
    $rising = 0
    $belowThreshold = $false
    $firstCrossing = $null
    $lastCrossing = $null
    $energy = 0.0
    for ($index = $data.start; $index -lt $data.start + $data.size; $index += $blockAlign) {
        $sample = [BitConverter]::ToInt16($bytes, $index)
        if ($sample -gt 128) { $positive++ }
        if ($sample -lt -128) { $negative++ }
        if ($sample -lt -128) { $belowThreshold = $true }
        if ($belowThreshold -and $sample -gt 128) {
            if ($null -eq $firstCrossing) { $firstCrossing = $index }
            $lastCrossing = $index
            $rising++
            $belowThreshold = $false
        }
        $energy += [double]$sample * $sample
    }
    if ($positive -lt 100 -or $negative -lt 100) { throw 'Recording lacks the expected non-silent signal' }
    if ($rising -lt 2) { throw 'Recording lacks repeating tone cycles' }
    $frequency = ($rising - 1) * $rate / (($lastCrossing - $firstCrossing) / $blockAlign)
    $rms = [Math]::Sqrt($energy / $frames) / 32768
    if ([Math]::Abs($frequency - 440) -gt 8 -or $rms -lt 0.06 -or $rms -gt 0.12) {
        throw 'Recording did not match the expected 440 Hz tone level and frequency'
    }
    return @{ frames = $frames; channels = $channels; sampleRate = $rate; frequencyHz = $frequency; rms = $rms; sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
}
