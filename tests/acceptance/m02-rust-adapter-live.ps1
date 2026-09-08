param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500
)

$ErrorActionPreference = 'Stop'

if (-not $AllowLiveAudio) {
    throw 'Refusing live audio acceptance without explicit -AllowLiveAudio'
}
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 5000) {
    throw 'DurationMilliseconds must be between 100 and 5000'
}

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
$savedErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$output = @(cargo run --manifest-path tools\m00-wasapi-probe\Cargo.toml -- adapter-smoke $DurationMilliseconds 2>&1)
$cargoExitCode = $LASTEXITCODE
$ErrorActionPreference = $savedErrorActionPreference
if ($cargoExitCode -ne 0) {
    throw "production Rust adapter smoke failed with exit code $cargoExitCode`n$($output -join [Environment]::NewLine)"
}
$line = $output | Where-Object { $_ -match '^adapter_smoke ' } | Select-Object -Last 1
if (-not $line -or $line -notmatch 'capture_packets=(\d+)' -or $line -notmatch 'capture_frames=(\d+)' -or $line -notmatch 'graph_generation=(\d+)' -or $line -notmatch 'graph_blocks=(\d+)' -or $line -notmatch 'scheduler_frames=(\d+)' -or $line -notmatch 'render_frames=(\d+)' -or $line -notmatch 'scheduler_processing_time_ns_total=(\d+)' -or $line -notmatch 'scheduler_processing_time_ns_max=(\d+)' -or $line -notmatch 'scheduler_processing_time_histogram_samples=(\d+)' -or $line -notmatch 'scheduler_processing_time_histogram=([0-9:,]+)' -or $line -notmatch 'scheduler_deadline_misses=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_ns_total=(\d+)' -or $line -notmatch 'scheduler_deadline_lateness_ns_max=(\d+)') {
    throw "adapter smoke did not report bounded capture/render counts`n$($output -join [Environment]::NewLine)"
}
$capturePackets = [int]([regex]::Match($line, 'capture_packets=(\d+)').Groups[1].Value)
if ($capturePackets -le 0) {
    throw 'adapter smoke reported no capture packets'
}
$captureFrames = [int]([regex]::Match($line, 'capture_frames=(\d+)').Groups[1].Value)
$graphGeneration = [int]([regex]::Match($line, 'graph_generation=(\d+)').Groups[1].Value)
$graphBlocks = [int]([regex]::Match($line, 'graph_blocks=(\d+)').Groups[1].Value)
$schedulerFrames = [int]([regex]::Match($line, 'scheduler_frames=(\d+)').Groups[1].Value)
$renderFrames = [int]([regex]::Match($line, 'render_frames=(\d+)').Groups[1].Value)
$processingTimeTotal = [long]([regex]::Match($line, 'scheduler_processing_time_ns_total=(\d+)').Groups[1].Value)
$processingTimeMax = [long]([regex]::Match($line, 'scheduler_processing_time_ns_max=(\d+)').Groups[1].Value)
$processingTimeSamples = [long]([regex]::Match($line, 'scheduler_processing_time_histogram_samples=(\d+)').Groups[1].Value)
$processingTimeHistogram = [regex]::Match($line, 'scheduler_processing_time_histogram=([0-9:,]+)').Groups[1].Value
$deadlineMisses = [long]([regex]::Match($line, 'scheduler_deadline_misses=(\d+)').Groups[1].Value)
$deadlineLatenessTotal = [long]([regex]::Match($line, 'scheduler_deadline_lateness_ns_total=(\d+)').Groups[1].Value)
$deadlineLatenessMax = [long]([regex]::Match($line, 'scheduler_deadline_lateness_ns_max=(\d+)').Groups[1].Value)
$processingTimeEntries = $processingTimeHistogram.Split(',')
if ($processingTimeEntries.Count -ne 32) {
    throw "adapter smoke reported an incomplete processing-time histogram: $line"
}
$processingTimeHistogramSum = [long]0
for ($bucket = 0; $bucket -lt 32; $bucket++) {
    $parts = $processingTimeEntries[$bucket].Split(':')
    if ($parts.Count -ne 2 -or $parts[0] -ne "$bucket" -or $parts[1] -notmatch '^\d+$') {
        throw "adapter smoke reported malformed processing-time histogram bucket: $line"
    }
    $processingTimeHistogramSum += [long]$parts[1]
}
if ($processingTimeHistogramSum -ne $processingTimeSamples) {
    throw "adapter smoke processing-time histogram sum did not match its sample count: $line"
}
if ($captureFrames -le 0 -or $graphGeneration -ne 1 -or $graphBlocks -le 0 -or $schedulerFrames -le 0 -or $renderFrames -le 0 -or $processingTimeTotal -lt $processingTimeMax -or $processingTimeSamples -ne $graphBlocks -or $deadlineMisses -gt $graphBlocks -or $deadlineLatenessTotal -lt $deadlineLatenessMax) {
    throw "adapter smoke reported invalid frame counts: $line"
}
$after = Get-MediaSnapshot
if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
    throw 'media-device identity/state changed during live adapter acceptance'
}
Write-Output "M02 production Rust adapter live acceptance passed: $line"
Write-Output 'Scope: explicit bounded live capture, generation-1 gain graph processing, and zero-valued caller-owned render buffers; streams stop/reset; media-device identity/state unchanged.'
