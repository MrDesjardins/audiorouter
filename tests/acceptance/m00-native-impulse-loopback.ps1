param(
    [switch]$AllowLiveAudio,
    [int]$ImpulseCount = 1000,
    [double]$P95ThresholdMs = 250.0,
    [string]$RenderFriendlyName = 'Speakers (Focusrite USB Audio)',
    [string]$CaptureFriendlyName = 'Analogue 1 + 2 (Focusrite USB Audio)'
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing impulse-loopback acceptance without explicit -AllowLiveAudio' }
if ($ImpulseCount -lt 10 -or $ImpulseCount -gt 2000) { throw 'ImpulseCount must be between 10 and 2000' }

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m00-impulse-loopback-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [IO.Path]::ChangeExtension($output, '.obj')

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

if (Test-Path -LiteralPath $object) { throw "generated object already exists: $object" }
$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output)) { throw 'impulse-loopback probe build failed' }

    $saved = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $inventory = @(& $output inventory 2>&1); $inventoryExitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    if ($inventoryExitCode -ne 0) { throw "inventory failed with exit code $inventoryExitCode`n$($inventory -join "`n")" }
    $inventoryText = $inventory -join "`n"
    $renderMatch = [regex]::Match($inventoryText, ('render\[(\d+)\] name=' + [regex]::Escape($RenderFriendlyName)))
    $captureMatch = [regex]::Match($inventoryText, ('capture\[(\d+)\] name=' + [regex]::Escape($CaptureFriendlyName)))
    if (-not $renderMatch.Success -or -not $captureMatch.Success) {
        throw "requested endpoints were not both found: '$RenderFriendlyName' / '$CaptureFriendlyName'"
    }
    $renderIndex = [int]$renderMatch.Groups[1].Value
    $captureIndex = [int]$captureMatch.Groups[1].Value

    $saved = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $result = @(& $output impulse-loopback $ImpulseCount $renderIndex $captureIndex 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    $resultText = $result -join "`n"
    Write-Output $resultText

    $after = Get-MediaSnapshot
    if (Compare-Object $before $after) { throw 'media-device identity/state changed during impulse-loopback acceptance' }
    if ($exitCode -ne 0) { throw "impulse-loopback probe failed (exit_code=$exitCode)`n$resultText" }

    $p95Match = [regex]::Match($resultText, 'loopback_latency_p95_ms=(-?[\d.]+)')
    $minMatch = [regex]::Match($resultText, 'loopback_latency_min_ms=(-?[\d.]+)')
    $p50Match = [regex]::Match($resultText, 'loopback_latency_p50_ms=(-?[\d.]+)')
    $maxMatch = [regex]::Match($resultText, 'loopback_latency_max_ms=(-?[\d.]+)')
    $pairsMatch = [regex]::Match($resultText, 'loopback_pairs=(\d+)')
    $renderBufferMatch = [regex]::Match($resultText, 'loopback_render_buffer_frames=(\d+)')
    $captureBufferMatch = [regex]::Match($resultText, 'loopback_capture_buffer_frames=(\d+)')
    if (-not $p95Match.Success -or -not $pairsMatch.Success) { throw "impulse-loopback output missing expected latency fields`n$resultText" }

    $p95 = [double]$p95Match.Groups[1].Value
    $pairs = [int]$pairsMatch.Groups[1].Value
    Write-Output ("NFR-01 candidate: pairs={0}/{1} min={2}ms p50={3}ms p95={4}ms max={5}ms render_buffer={6} capture_buffer={7} threshold_p95<={8}ms" -f `
        $pairs, $ImpulseCount, $minMatch.Groups[1].Value, $p50Match.Groups[1].Value, $p95, $maxMatch.Groups[1].Value, `
        $renderBufferMatch.Groups[1].Value, $captureBufferMatch.Groups[1].Value, $P95ThresholdMs)

    if ($pairs -lt [Math]::Floor($ImpulseCount * 0.9)) {
        throw "only $pairs of $ImpulseCount impulses were paired; not enough evidence for a calibrated NFR-01 result"
    }
    if ($p95 -gt $P95ThresholdMs) {
        throw "NFR-01 gate failed: p95=$p95 ms exceeds the $P95ThresholdMs ms threshold"
    }
    Write-Output "NFR-01 wired physical loopback latency gate passed."
}
finally {
    $cleanupPaths = @($output, $temporaryObject, $object)
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        Remove-Item -LiteralPath $cleanupPaths -Force -ErrorAction SilentlyContinue
        if (-not (Test-Path -LiteralPath $object)) { break }
        Start-Sleep -Milliseconds 100
    }
}
