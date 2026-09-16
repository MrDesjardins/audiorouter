param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$build = Join-Path $repositoryRoot 'tools\m03-swdevice-probe\build.ps1'
$token = [guid]::NewGuid().ToString('N')
$output = Join-Path ([IO.Path]::GetTempPath()) "audiorouter-swdevice-$token.exe"
$object = [IO.Path]::ChangeExtension($output, '.obj')
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $build -Output $output -Object $object
    if ($LASTEXITCODE -ne 0) { throw "software-device probe build failed with exit code $LASTEXITCODE" }
    $result = @(& $output 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "software-device dry-run failed with exit code $LASTEXITCODE" }
    $text = $result -join "`n"
    if ($text -notmatch 'dry-run: enumerator=AudioRouter parent=HTREE\\ROOT\\0 hardwareId=SWD\\AudioRouterVirtual instance=dry-run-bus') {
        throw "software-device dry-run did not report the expected bounded plan: $text"
    }
    Write-Output 'M03 Software Device API probe acceptance passed'
    Write-Output 'Scope: native compile and no-side-effect dry-run only; no software device, driver, endpoint, or machine audio configuration was created.'
}
finally {
    foreach ($path in @($output, $object)) {
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }
    }
}
