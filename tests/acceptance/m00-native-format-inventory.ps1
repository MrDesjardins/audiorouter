param()

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$probeDirectory = Join-Path $workspace 'tools/m00-native-wasapi-probe'
$buildScript = Join-Path $probeDirectory 'build.ps1'
$output = Join-Path ([IO.Path]::GetTempPath()) ("audiorouter-m00-format-{0}.exe" -f ([guid]::NewGuid()))
$temporaryObject = [IO.Path]::ChangeExtension($output, '.obj')

function Get-MediaSnapshot {
    @(Get-PnpDevice -Class Media -PresentOnly | ForEach-Object {
        '{0}|{1}|{2}|{3}' -f $_.Status, $_.Class, $_.FriendlyName, $_.InstanceId
    } | Sort-Object)
}

$before = Get-MediaSnapshot
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $buildScript -Output $output -Object $temporaryObject
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output -PathType Leaf)) {
        throw 'native endpoint format probe build failed'
    }
    $lines = @(& $output inventory-formats 2>&1)
    $exitCode = $LASTEXITCODE
    $text = $lines -join "`n"
    if ($exitCode -ne 0) { throw "native endpoint format inventory failed`n$text" }
    $endpointLines = @($lines | Where-Object { $_ -match '^(render|capture)\[\d+\] name=' })
    $formatLines = @($lines | Where-Object { $_ -match '^(render|capture)\[\d+\] mix_format=activate_or_format=0x0' })
    $rateLines = @($lines | Where-Object { $_ -match '^rate=\d+ channels=\d+ bits=\d+ ' })
    if ($endpointLines.Count -lt 1 -or $formatLines.Count -ne $endpointLines.Count -or
        $rateLines.Count -ne $endpointLines.Count) {
        throw "format inventory was incomplete: endpoints=$($endpointLines.Count) formats=$($formatLines.Count) rates=$($rateLines.Count)`n$text"
    }
    $after = Get-MediaSnapshot
    if (Compare-Object -ReferenceObject $before -DifferenceObject $after) {
        throw 'media-device identity/state changed during format inventory'
    }
    Write-Output "M00 native format inventory acceptance passed: endpoints=$($endpointLines.Count)"
    $rateLines | Sort-Object -Unique | ForEach-Object { Write-Output "  $_" }
    Write-Output 'Scope: read-only endpoint activation and GetMixFormat metadata; no audio stream, driver, or machine configuration action.'
}
finally {
    Remove-Item -LiteralPath $output, $temporaryObject -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $output) { throw "native format executable cleanup failed: $output" }
    if (Test-Path -LiteralPath $temporaryObject) { throw "native format object cleanup failed: $temporaryObject" }
}
