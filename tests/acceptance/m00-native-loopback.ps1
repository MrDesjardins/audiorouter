param(
    [switch]$AllowLiveAudio,
    [int]$CaptureDurationMilliseconds = 1000,
    [int]$ToneDurationMilliseconds = 1500,
    [string]$RenderFriendlyName = 'CABLE Input (VB-Audio Virtual Cable)',
    [string]$CaptureFriendlyName = 'CABLE Output (VB-Audio Virtual Cable)'
)

$ErrorActionPreference = 'Stop'

if (-not $AllowLiveAudio) {
    throw 'Refusing signal-path acceptance without explicit -AllowLiveAudio'
}
if ($CaptureDurationMilliseconds -lt 250 -or $CaptureDurationMilliseconds -gt 2000) {
    throw 'CaptureDurationMilliseconds must be between 250 and 2000'
}
if ($ToneDurationMilliseconds -lt $CaptureDurationMilliseconds -or $ToneDurationMilliseconds -gt 3000) {
    throw 'ToneDurationMilliseconds must be at least the capture duration and at most 3000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-loopback-{0}.exe" -f ([guid]::NewGuid()))
$captureLog = "$output.capture.log"
$toneLog = "$output.tone.log"

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

function Invoke-Probe([string[]]$Arguments) {
    $savedErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $result = @(& $output @Arguments 2>&1)
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $savedErrorActionPreference
    [pscustomobject]@{ Output = $result; ExitCode = $exitCode }
}

if (Test-Path -LiteralPath $object) {
    throw "refusing virtual loopback acceptance because generated object already exists: $object"
}

$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'signal-path acceptance probe build failed'
    }

    $inventory = Invoke-Probe @('inventory')
    $inventoryText = $inventory.Output -join "`n"
    if ($inventory.ExitCode -ne 0) { throw "native endpoint inventory failed`n$inventoryText" }

    $renderMatch = [regex]::Match(
        $inventoryText,
        ('render\[(\d+)\] name=' + [regex]::Escape($RenderFriendlyName)))
    $captureMatch = [regex]::Match(
        $inventoryText,
        ('capture\[(\d+)\] name=' + [regex]::Escape($CaptureFriendlyName)))
    if (-not $renderMatch.Success -or -not $captureMatch.Success) {
        throw "Requested render/capture endpoints were not both found: '$RenderFriendlyName' / '$CaptureFriendlyName'"
    }
    $renderIndex = [int]$renderMatch.Groups[1].Value
    $captureIndex = [int]$captureMatch.Groups[1].Value

    $capture = Start-Process -FilePath $output `
        -ArgumentList @('capture', $captureIndex, $CaptureDurationMilliseconds) `
        -RedirectStandardOutput $captureLog `
        -RedirectStandardError "$captureLog.err" -PassThru
    Start-Sleep -Milliseconds 150
    $tone = Start-Process -FilePath $output `
        -ArgumentList @('tone', $ToneDurationMilliseconds, $renderIndex) `
        -RedirectStandardOutput $toneLog `
        -RedirectStandardError "$toneLog.err" -PassThru
    Wait-Process -Id $capture.Id
    Wait-Process -Id $tone.Id

    $captureText = Get-Content -LiteralPath $captureLog -Raw
    $toneText = Get-Content -LiteralPath $toneLog -Raw
    if ($captureText -notmatch 'capture_start=0x0' -or
        $captureText -notmatch 'capture_stop=0x0' -or
        $captureText -notmatch 'capture_reset=0x0') {
        throw "signal-path capture lifecycle failed`n$captureText"
    }
    $nonzeroMatch = [regex]::Match($captureText, 'capture_nonzero_bytes=(\d+)')
    if (-not $nonzeroMatch.Success -or [int64]$nonzeroMatch.Groups[1].Value -le 0) {
        throw "signal-path capture contained no nonzero payload bytes`n$captureText"
    }
    if ($toneText -notmatch 'render_start=0x0' -or
        $toneText -notmatch 'render_stop=0x0' -or
        $toneText -notmatch 'render_reset=0x0' -or
        $toneText -notmatch 'render_tone_written=1') {
        throw "signal-path tone lifecycle failed`n$toneText"
    }

    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during signal-path acceptance'
    }
    Write-Output ("M00 native signal path passed: render='{0}' capture='{1}' render_index={2} capture_index={3} capture_nonzero_bytes={4} capture_duration_ms={5} tone_duration_ms={6}" -f `
        $RenderFriendlyName, $CaptureFriendlyName, $renderIndex, $captureIndex, $nonzeroMatch.Groups[1].Value,
        $CaptureDurationMilliseconds, $ToneDurationMilliseconds)
    Write-Output 'Scope: explicitly selected existing endpoints only; defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged.'
}
finally {
    Remove-Item -LiteralPath $output, $captureLog, "$captureLog.err", $toneLog, "$toneLog.err", $object -Force -ErrorAction SilentlyContinue
}
