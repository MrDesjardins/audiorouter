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
    if (-not $line -or $line -notmatch 'route=true' -or $line -notmatch 'capture_frames=(\d+)' -or $line -notmatch 'scheduler_frames=(\d+)' -or $line -notmatch 'routed_frames=(\d+)') { throw "adapter route did not report bounded routed frames`n$routeText" }
    $captureFrames = [int]([regex]::Match($line, 'capture_frames=(\d+)').Groups[1].Value)
    $schedulerFrames = [int]([regex]::Match($line, 'scheduler_frames=(\d+)').Groups[1].Value)
    $routedFrames = [int]([regex]::Match($line, 'routed_frames=(\d+)').Groups[1].Value)
    if ($captureFrames -le 0 -or $schedulerFrames -le 0 -or $routedFrames -le 0) { throw "adapter route reported invalid frame counts: $line" }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) { throw 'media-device identity/state changed during adapter route acceptance' }
    Write-Output ("M02 Rust adapter route passed: render='{0}' capture='{1}' capture_frames={2} scheduler_frames={3} routed_frames={4}" -f $renderLabel, $captureLabel, $captureFrames, $schedulerFrames, $routedFrames)
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
