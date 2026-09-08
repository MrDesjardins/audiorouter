param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 250
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing Rust process-loopback acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) {
    throw 'DurationMilliseconds must be between 100 and 2000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
try {
    foreach ($mode in @('include', 'exclude')) {
        $result = @(& cargo run --quiet --manifest-path (Join-Path $workspace 'tools/m00-wasapi-probe/Cargo.toml') -- process-loopback $DurationMilliseconds $mode 2>&1)
        $exitCode = $LASTEXITCODE
        $text = $result -join "`n"
        if ($exitCode -ne 0 -or $text -notmatch ("process_loopback mode=" + $mode) -or
            $text -notmatch 'source_rate_hz=44100' -or $text -notmatch 'engine_rate_hz=48000' -or
            $text -notmatch 'packets=(\d+)' -or $text -notmatch 'source_frames=(\d+)' -or
            $text -notmatch 'engine_frames=(\d+)' -or
            $text -notmatch 'quantum_blocks=(\d+)' -or $text -notmatch 'scheduler_generation=1' -or
            $text -notmatch 'rejected_packets=0') {
            throw "Rust process-loopback $mode failed`n$text"
        }
        $sourceFrames = [regex]::Match($text, 'source_frames=(\d+)')
        $engineFrames = [regex]::Match($text, 'engine_frames=(\d+)')
        if ([int]$sourceFrames.Groups[1].Value -le 0 -or [int]$engineFrames.Groups[1].Value -le 0) {
            throw "Rust process-loopback $mode returned no converted frames`n$text"
        }
        $blocks = [regex]::Match($text, 'quantum_blocks=(\d+)')
        if ([int]$blocks.Groups[1].Value -le 0) { throw "Rust process-loopback $mode emitted no scheduler blocks`n$text" }
        Write-Output ("Rust process-loopback {0} passed: {1}" -f $mode, $text.Trim())
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during Rust process-loopback acceptance'
    }
    Write-Output 'Scope: Rust asynchronous process-loopback API only; streams stop/reset and no persistent audio configuration changed.'
}
finally {
    # cargo owns its build outputs; this acceptance performs no machine-state cleanup.
}
