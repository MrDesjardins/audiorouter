param(
    [switch]$AllowLiveAudio,
    [int]$DurationMilliseconds = 100
)

$ErrorActionPreference = 'Stop'

if (-not $AllowLiveAudio) {
    throw 'Refusing live audio acceptance without explicit -AllowLiveAudio'
}
if ($DurationMilliseconds -lt 50 -or $DurationMilliseconds -gt 1000) {
    throw 'DurationMilliseconds must be between 50 and 1000'
}

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$object = Join-Path $probeDirectory 'main.obj'
$output = Join-Path ([System.IO.Path]::GetTempPath()) ("audiorouter-m00-live-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [System.IO.Path]::ChangeExtension($output, '.obj')

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
    throw "refusing native live acceptance because generated object already exists: $object"
}

$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'native live acceptance probe build failed'
    }

    $inventory = Invoke-Probe @()
    if ($inventory.ExitCode -ne 0 -or ($inventory.Output -join "`n") -notmatch 'capture_endpoint_count=(\d+)') {
        throw "native endpoint inventory failed`n$($inventory.Output -join "`n")"
    }
    $captureCount = [int]([regex]::Match(($inventory.Output -join "`n"), 'capture_endpoint_count=(\d+)').Groups[1].Value)
    if ($captureCount -le 0) { throw 'native inventory reported no capture endpoints' }

    for ($index = 0; $index -lt $captureCount; $index++) {
        $capture = Invoke-Probe @('capture', $index, $DurationMilliseconds)
        if ($capture.ExitCode -ne 0 -or ($capture.Output -join "`n") -notmatch 'capture_start=0x0' -or ($capture.Output -join "`n") -notmatch 'capture_stop=0x0' -or ($capture.Output -join "`n") -notmatch 'capture_reset=0x0') {
            throw "capture endpoint $index failed`n$($capture.Output -join "`n")"
        }
    }

    $renderCount = 0
    $occupiedRenders = 0
    while ($true) {
        $render = Invoke-Probe @('render', $renderCount, $DurationMilliseconds)
        $renderText = $render.Output -join "`n"
        if ($renderText -match 'render_index_out_of_range=(\d+) count=(\d+)') {
            $reportedCount = [int]([regex]::Match($renderText, 'count=(\d+)').Groups[1].Value)
            if ($reportedCount -ne $renderCount) { throw "render inventory count changed during acceptance" }
            break
        }
        if ($renderText -match 'render_initialize=0x8889000a') {
            if ($renderText -notmatch 'render_item=0x0' -or $renderText -notmatch 'render_activate=0x0') {
                throw "render endpoint $renderCount had an unexpected ownership failure`n$renderText"
            }
            $occupiedRenders++
        } elseif ($render.ExitCode -ne 0 -or $renderText -notmatch 'render_start=0x0' -or $renderText -notmatch 'render_stop=0x0' -or $renderText -notmatch 'render_reset=0x0') {
            throw "render endpoint $renderCount failed`n$renderText"
        }
        $renderCount++
    }
    if ($renderCount -le 0) { throw 'native inventory reported no render endpoints' }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during native live acceptance'
    }
    Write-Output "M00 native live acceptance passed: capture_endpoints=$captureCount render_endpoints=$renderCount occupied_render_endpoints=$occupiedRenders duration_ms=$DurationMilliseconds"
    Write-Output 'Scope: bounded shared capture and silent render lifecycle only; defaults, volume, mute, privacy, drivers, signing, and startup configuration unchanged.'
}
finally {
    Remove-Item -LiteralPath $output, $object, $temporaryObject -Force -ErrorAction SilentlyContinue
}
