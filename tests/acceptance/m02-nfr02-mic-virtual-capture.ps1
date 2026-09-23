param(
    [switch]$AllowLiveAudio,
    [int]$ImpulseCount = 500,
    [double]$P95ThresholdMs = 160.0,
    [int]$RouteDurationMilliseconds = 8000,
    [string]$RenderFriendlyName = 'Speakers (Focusrite USB Audio)',
    [string]$MicCaptureFriendlyName = 'Analogue 1 + 2 (Focusrite USB Audio)',
    [string]$VirtualRenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$VirtualCaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)'
)

$ErrorActionPreference = 'Stop'
if (-not $AllowLiveAudio) { throw 'Refusing NFR-02 acceptance without explicit -AllowLiveAudio' }
if ($ImpulseCount -lt 10 -or $ImpulseCount -gt 500) { throw 'ImpulseCount must be between 10 and 500 to fit the route duration bound' }

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$nativeProbeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $nativeProbeDirectory 'build.ps1'
$nativeObject = Join-Path $nativeProbeDirectory 'main.obj'
$nativeOutput = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m02-nfr02-{0}.exe" -f ([guid]::NewGuid()))
$nativeTemporaryObject = [IO.Path]::ChangeExtension($nativeOutput, '.obj')
$routeExe = Join-Path $workspace 'tools/m00-wasapi-probe/target/debug/m00-wasapi-probe.exe'
$routeLog = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m02-nfr02-route-{0}.log" -f ([guid]::NewGuid()))

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

if (Test-Path -LiteralPath $nativeObject) { throw "generated object already exists: $nativeObject" }
$before = Get-MediaSnapshot
$routeProcess = $null
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $nativeOutput
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $nativeOutput)) { throw 'native probe build failed' }
    & cargo build --quiet --manifest-path (Join-Path $workspace 'tools/m00-wasapi-probe/Cargo.toml')
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $routeExe)) { throw 'Rust adapter probe build failed' }

    $saved = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $inventory = @(& $nativeOutput inventory 2>&1)
    $ErrorActionPreference = $saved
    $inventoryText = $inventory -join "`n"
    $renderMatch = [regex]::Match($inventoryText, ('render\[(\d+)\] name=' + [regex]::Escape($RenderFriendlyName)))
    $micCaptureMatch = [regex]::Match($inventoryText, ('capture\[(\d+)\] name=' + [regex]::Escape($MicCaptureFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
    $virtualRenderMatch = [regex]::Match($inventoryText, ('render\[(\d+)\] name=' + [regex]::Escape($VirtualRenderFriendlyName) + ' id=(.+)$'), [Text.RegularExpressions.RegexOptions]::Multiline)
    $virtualCaptureMatch = [regex]::Match($inventoryText, ('capture\[(\d+)\] name=' + [regex]::Escape($VirtualCaptureFriendlyName)))
    if (-not $renderMatch.Success -or -not $micCaptureMatch.Success -or -not $virtualRenderMatch.Success -or -not $virtualCaptureMatch.Success) {
        throw "requested endpoints were not all found`n$inventoryText"
    }
    $renderIndex = [int]$renderMatch.Groups[1].Value
    $micCaptureIndex = [int]$micCaptureMatch.Groups[1].Value
    $micCaptureId = $micCaptureMatch.Groups[2].Value.Trim()
    $virtualRenderId = $virtualRenderMatch.Groups[2].Value.Trim()
    $virtualCaptureIndex = [int]$virtualCaptureMatch.Groups[1].Value

    $routeProcess = Start-Process -FilePath $routeExe -ArgumentList @('adapter-control-route', "$RouteDurationMilliseconds", $micCaptureId, $virtualRenderId) `
        -RedirectStandardOutput $routeLog -RedirectStandardError "$routeLog.err" -PassThru -NoNewWindow
    Start-Sleep -Milliseconds 500

    $saved = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $result = @(& $nativeOutput capture-loopback $ImpulseCount $renderIndex $micCaptureIndex $virtualCaptureIndex 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $saved
    $resultText = $result -join "`n"
    Write-Output $resultText

    $routeExited = $routeProcess.WaitForExit(($RouteDurationMilliseconds + 3000))
    if ($routeExited) { $routeProcess.Refresh() }
    $routeOutputText = (Get-Content -LiteralPath $routeLog -ErrorAction SilentlyContinue) -join "`n"
    $routeErrorText = (Get-Content -LiteralPath "$routeLog.err" -ErrorAction SilentlyContinue) -join "`n"
    Write-Output "--- route output ---`n$routeOutputText"
    if (-not $routeExited) { throw "AudioRouter control-owned route did not exit in time`n$routeErrorText" }
    if ($routeOutputText -notmatch 'route=true') {
        throw "AudioRouter control-owned route failed`n$routeOutputText`n$routeErrorText"
    }

    $after = Get-MediaSnapshot
    if (Compare-Object $before $after) { throw 'media-device identity/state changed during NFR-02 acceptance' }
    if ($exitCode -ne 0) { throw "capture-loopback probe failed (exit_code=$exitCode)`n$resultText" }

    $p95Match = [regex]::Match($resultText, 'nfr02_latency_p95_ms=(-?[\d.]+)')
    $pairsMatch = [regex]::Match($resultText, 'nfr02_pairs=(\d+)')
    if (-not $p95Match.Success -or -not $pairsMatch.Success) { throw "capture-loopback output missing expected latency fields`n$resultText" }
    $p95 = [double]$p95Match.Groups[1].Value
    $pairs = [int]$pairsMatch.Groups[1].Value
    if ($pairs -lt [Math]::Floor($ImpulseCount * 0.9)) {
        throw "only $pairs of $ImpulseCount impulses were paired; not enough evidence for a calibrated NFR-02 result"
    }
    Write-Output ("NFR-02 candidate: pairs={0}/{1} p95={2}ms threshold_p95<={3}ms" -f $pairs, $ImpulseCount, $p95, $P95ThresholdMs)
    if ($p95 -gt $P95ThresholdMs) {
        throw "NFR-02 gate failed: p95=$p95 ms exceeds the $P95ThresholdMs ms threshold"
    }
    Write-Output "NFR-02 mic-to-virtual-capture latency gate passed."
}
finally {
    if ($routeProcess -and -not $routeProcess.HasExited) {
        Stop-Process -Id $routeProcess.Id -Force -ErrorAction SilentlyContinue
    }
    $cleanupPaths = @($nativeOutput, $nativeTemporaryObject, $nativeObject, $routeLog, "$routeLog.err")
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        Remove-Item -LiteralPath $cleanupPaths -Force -ErrorAction SilentlyContinue
        if (-not (Test-Path -LiteralPath $nativeObject)) { break }
        Start-Sleep -Milliseconds 100
    }
}
