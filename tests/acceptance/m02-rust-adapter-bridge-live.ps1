param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 500,
    [int]$Cycles = 1,
    [string]$RenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$CaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)',
    [string]$RenderEndpointId = '',
    [string]$CaptureEndpointId = ''
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing live bridge acceptance without explicit -AllowLiveAudio' }
if ($DurationMilliseconds -lt 100 -or $DurationMilliseconds -gt 2000) { throw 'DurationMilliseconds must be between 100 and 2000' }
if ($Cycles -lt 1 -or $Cycles -gt 5) { throw 'Cycles must be between 1 and 5' }
if (([string]::IsNullOrWhiteSpace($RenderEndpointId)) -xor ([string]::IsNullOrWhiteSpace($CaptureEndpointId))) {
    throw 'RenderEndpointId and CaptureEndpointId must be supplied together'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$nativeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$nativeOutput = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-bridge-inventory-' + [guid]::NewGuid() + '.exe')
$nativeObject = [IO.Path]::ChangeExtension($nativeOutput, '.obj')

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
try {
    if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $nativeDirectory 'build.ps1') -Output $nativeOutput -Object $nativeObject
        if ($LASTEXITCODE -ne 0) { throw 'native endpoint inventory build failed' }
        $inventory = @(& $nativeOutput inventory 2>&1)
        if ($LASTEXITCODE -ne 0) { throw "native endpoint inventory failed: $($inventory -join "`n")" }
        $text = $inventory -join "`n"
        $render = [regex]::Match($text, ('render\[\d+\] name=' + [regex]::Escape($RenderFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
        $capture = [regex]::Match($text, ('capture\[\d+\] name=' + [regex]::Escape($CaptureFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
        if (-not $render.Success -or -not $capture.Success) { throw "matching bridge endpoints were not found: '$RenderFriendlyName' / '$CaptureFriendlyName'" }
        $RenderEndpointId = $render.Groups[1].Value.Trim()
        $CaptureEndpointId = $capture.Groups[1].Value.Trim()
    }

    $lines = @()
    for ($cycle = 1; $cycle -le $Cycles; $cycle++) {
        $output = @(& cargo run --quiet --manifest-path (Join-Path $workspace 'tools/m00-wasapi-probe/Cargo.toml') -- adapter-bridge $DurationMilliseconds $CaptureEndpointId $RenderEndpointId 2>&1)
        if ($LASTEXITCODE -ne 0) { throw "Rust adapter bridge failed on cycle ${cycle}: $($output -join "`n")" }
        $line = $output | Where-Object { $_ -match '^adapter_bridge ' } | Select-Object -Last 1
        if (-not $line) { throw "adapter bridge did not report telemetry on cycle ${cycle}: $($output -join "`n")" }
        $captured = [int]([regex]::Match($line, 'captured_frames=(\d+)').Groups[1].Value)
        $quanta = [int]([regex]::Match($line, 'processed_quanta=(\d+)').Groups[1].Value)
        $tapCalls = [int]([regex]::Match($line, 'tap_calls=(\d+)').Groups[1].Value)
        $tapNonFinite = [int]([regex]::Match($line, 'tap_non_finite_samples=(\d+)').Groups[1].Value)
        $rendered = [int]([regex]::Match($line, 'rendered_frames=(\d+)').Groups[1].Value)
        $dropped = [int]([regex]::Match($line, 'dropped_render_frames=(\d+)').Groups[1].Value)
        $recordingBytes = [long]([regex]::Match($line, 'recording_file_bytes=(\d+)').Groups[1].Value)
        $xruns = [int]([regex]::Match($line, 'scheduler_xruns=(\d+)').Groups[1].Value)
        $deadlineMisses = [int]([regex]::Match($line, 'scheduler_deadline_misses=(\d+)').Groups[1].Value)
        if ($captured -le 0 -or $quanta -le 0 -or $tapCalls -ne $quanta -or $tapNonFinite -ne 0 -or $rendered -le 0 -or $recordingBytes -le 128 -or $dropped -ne 0 -or $xruns -ne 0 -or $deadlineMisses -ne 0) {
            throw "adapter bridge reported invalid live telemetry on cycle ${cycle}: $line"
        }
        $lines += "cycle=${cycle} $line"
        $afterCycle = Get-MediaSnapshot
        if (Compare-Object -ReferenceObject $before -DifferenceObject $afterCycle) { throw "media-device identity/state changed during bridge cycle ${cycle}" }
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) { throw 'media-device identity/state changed during bridge acceptance' }
    Write-Output "M02 Rust adapter bridge passed: cycles=$Cycles"
    $lines | ForEach-Object { Write-Output $_ }
    Write-Output 'Scope: explicitly selected existing endpoints; every temporary stream/recording was stopped/removed and media-device state remained unchanged.'
}
finally {
    Remove-Item -LiteralPath $nativeOutput, $nativeObject -Force -ErrorAction SilentlyContinue
}
