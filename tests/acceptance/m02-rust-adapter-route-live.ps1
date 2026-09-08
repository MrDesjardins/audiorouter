param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500,
    [string]$RenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$CaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)',
    [string]$RenderEndpointId = '',
    [string]$CaptureEndpointId = ''
)

$ErrorActionPreference = 'Stop'

if (-not $AllowLiveAudio) { throw 'Refusing adapter route acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) { throw 'DurationMilliseconds must be between 100 and 2000' }

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$output = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m02-route-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [IO.Path]::ChangeExtension($output, '.obj')
$explicitEndpointIds = -not [string]::IsNullOrWhiteSpace($RenderEndpointId) -and
    -not [string]::IsNullOrWhiteSpace($CaptureEndpointId)
$renderLabel = if ($explicitEndpointIds) { $RenderEndpointId } else { $RenderFriendlyName }
$captureLabel = if ($explicitEndpointIds) { $CaptureEndpointId } else { $CaptureFriendlyName }
if (([string]::IsNullOrWhiteSpace($RenderEndpointId)) -xor
    ([string]::IsNullOrWhiteSpace($CaptureEndpointId))) {
    throw 'RenderEndpointId and CaptureEndpointId must be supplied together'
}

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

function Invoke-External([scriptblock]$Command) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $result = @(& $Command 2>&1)
        [pscustomobject]@{ Output = $result; ExitCode = $LASTEXITCODE }
    }
    finally { $ErrorActionPreference = $saved }
}

$before = Get-MediaSnapshot
try {
    if (-not $explicitEndpointIds) {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output -Object $temporaryObject
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) { throw 'native endpoint inventory probe build failed' }
        $inventory = Invoke-External { & $output inventory }
        $inventoryText = $inventory.Output -join "`n"
        if ($inventory.ExitCode -ne 0) { throw "native endpoint inventory failed`n$inventoryText" }
        $render = [regex]::Match($inventoryText, ('render\[\d+\] name=' + [regex]::Escape($RenderFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
        $capture = [regex]::Match($inventoryText, ('capture\[\d+\] name=' + [regex]::Escape($CaptureFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
        if (-not $render.Success -or -not $capture.Success) { throw "requested route endpoints were not both found: '$RenderFriendlyName' / '$CaptureFriendlyName'" }
        $RenderEndpointId = $render.Groups[1].Value.Trim()
        $CaptureEndpointId = $capture.Groups[1].Value.Trim()
    }

    $route = Invoke-External {
        & cargo run --manifest-path (Join-Path $workspace 'tools/m00-wasapi-probe/Cargo.toml') -- adapter-route $DurationMilliseconds $CaptureEndpointId $RenderEndpointId
    }
    $routeText = $route.Output -join "`n"
    if ($route.ExitCode -ne 0) { throw "Rust adapter route failed`n$routeText" }
    $line = $route.Output | Where-Object { $_ -match '^adapter_smoke ' } | Select-Object -Last 1
    if (-not $line -or $line -notmatch 'route=true' -or $line -notmatch 'capture_frames=(\d+)' -or $line -notmatch 'graph_blocks=(\d+)' -or $line -notmatch 'scheduler_frames=(\d+)' -or $line -notmatch 'routed_frames=(\d+)' -or $line -notmatch 'scheduler_processing_time_ns_total=(\d+)' -or $line -notmatch 'scheduler_processing_time_ns_max=(\d+)' -or $line -notmatch 'scheduler_processing_time_p999_upper_bound_ns=(\d+)' -or $line -notmatch 'scheduler_processing_time_histogram_samples=(\d+)' -or $line -notmatch 'scheduler_processing_time_histogram=([0-9:,]+)' -or $line -notmatch 'scheduler_deadline_misses=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_ns_total=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_ns_max=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_p999_upper_bound_ns=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_histogram=([0-9:,]+)') { throw "adapter route did not report bounded routed frames and timing telemetry`n$routeText" }
    $captureFrames = [int]([regex]::Match($line, 'capture_frames=(\d+)').Groups[1].Value)
    $graphBlocks = [int]([regex]::Match($line, 'graph_blocks=(\d+)').Groups[1].Value)
    $schedulerFrames = [int]([regex]::Match($line, 'scheduler_frames=(\d+)').Groups[1].Value)
    $routedFrames = [int]([regex]::Match($line, 'routed_frames=(\d+)').Groups[1].Value)
    $processingTimeTotal = [long]([regex]::Match($line, 'scheduler_processing_time_ns_total=(\d+)').Groups[1].Value)
    $processingTimeMax = [long]([regex]::Match($line, 'scheduler_processing_time_ns_max=(\d+)').Groups[1].Value)
    $processingTimeP999 = [long]([regex]::Match($line, 'scheduler_processing_time_p999_upper_bound_ns=(\d+)').Groups[1].Value)
    $processingTimeSamples = [long]([regex]::Match($line, 'scheduler_processing_time_histogram_samples=(\d+)').Groups[1].Value)
    $processingTimeHistogram = [regex]::Match($line, 'scheduler_processing_time_histogram=([0-9:,]+)').Groups[1].Value
    $deadlineMisses = [long]([regex]::Match($line, 'scheduler_deadline_misses=(\d+)').Groups[1].Value)
    $deadlineLatenessTotal = [long]([regex]::Match($line, 'scheduler_deadline_lateness_ns_total=(\d+)').Groups[1].Value)
    $deadlineLatenessMax = [long]([regex]::Match($line, 'scheduler_deadline_lateness_ns_max=(\d+)').Groups[1].Value)
    $deadlineLatenessP999 = [long]([regex]::Match($line, 'scheduler_deadline_lateness_p999_upper_bound_ns=(\d+)').Groups[1].Value)
    $deadlineLatenessHistogram = [regex]::Match($line, 'scheduler_deadline_lateness_histogram=([0-9:,]+)').Groups[1].Value
    $deadlineLatenessEntries = $deadlineLatenessHistogram.Split(',')
    if ($deadlineLatenessEntries.Count -ne 32) { throw "adapter route reported an incomplete deadline-lateness histogram: $line" }
    $deadlineLatenessSamples = [long]0
    for ($bucket = 0; $bucket -lt 32; $bucket++) {
        $parts = $deadlineLatenessEntries[$bucket].Split(':')
        if ($parts.Count -ne 2 -or $parts[0] -ne "$bucket" -or $parts[1] -notmatch '^\d+$') { throw "adapter route reported malformed deadline-lateness histogram bucket: $line" }
        $deadlineLatenessSamples += [long]$parts[1]
    }
    $processingTimeEntries = $processingTimeHistogram.Split(',')
    if ($processingTimeEntries.Count -ne 32) { throw "adapter route reported an incomplete processing-time histogram: $line" }
    $processingTimeHistogramSum = [long]0
    for ($bucket = 0; $bucket -lt 32; $bucket++) {
        $parts = $processingTimeEntries[$bucket].Split(':')
        if ($parts.Count -ne 2 -or $parts[0] -ne "$bucket" -or $parts[1] -notmatch '^\d+$') { throw "adapter route reported malformed processing-time histogram bucket: $line" }
        $processingTimeHistogramSum += [long]$parts[1]
    }
    if ($processingTimeHistogramSum -ne $processingTimeSamples) { throw "adapter route processing-time histogram sum did not match its sample count: $line" }
    if ($captureFrames -le 0 -or $graphBlocks -le 0 -or $schedulerFrames -le 0 -or $routedFrames -le 0 -or $processingTimeTotal -lt $processingTimeMax -or $processingTimeP999 -lt $processingTimeMax -or $processingTimeSamples -ne $graphBlocks -or $deadlineMisses -gt $graphBlocks -or $deadlineLatenessSamples -ne $deadlineMisses -or $deadlineLatenessTotal -lt $deadlineLatenessMax -or $deadlineLatenessP999 -lt $deadlineLatenessMax) { throw "adapter route reported invalid frame or timing counts: $line" }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) { throw 'media-device identity/state changed during adapter route acceptance' }
    Write-Output ("M02 Rust adapter route passed: render='{0}' capture='{1}' capture_frames={2} graph_blocks={3} scheduler_frames={4} routed_frames={5} processing_time_ns_total={6} processing_time_ns_max={7} processing_time_p999_upper_bound_ns={8} processing_time_histogram_samples={9} processing_time_histogram={10} deadline_misses={11} deadline_lateness_ns_total={12} deadline_lateness_ns_max={13} deadline_lateness_p999_upper_bound_ns={14} deadline_lateness_histogram={15}" -f $renderLabel, $captureLabel, $captureFrames, $graphBlocks, $schedulerFrames, $routedFrames, $processingTimeTotal, $processingTimeMax, $processingTimeP999, $processingTimeSamples, $processingTimeHistogram, $deadlineMisses, $deadlineLatenessTotal, $deadlineLatenessMax, $deadlineLatenessP999, $deadlineLatenessHistogram)
    Write-Output 'Scope: explicitly selected existing endpoints only; defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged.'
}
finally {
    Remove-Item -LiteralPath $output, $temporaryObject -Force -ErrorAction SilentlyContinue
    for ($attempt = 0; $attempt -lt 5 -and (Test-Path -LiteralPath $temporaryObject); $attempt++) {
        Remove-Item -LiteralPath $temporaryObject -Force -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $temporaryObject) { Start-Sleep -Milliseconds 100 }
    }
    if (Test-Path -LiteralPath $temporaryObject) { throw "native generated object cleanup failed: $temporaryObject" }
}
