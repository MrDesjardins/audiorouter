param(
    [switch]$Build
)

$ErrorActionPreference = 'Stop'

# This launcher is deliberately disposable. It never changes Windows default
# devices, endpoint volume/mute, startup registration, driver state, or the
# caller's environment. The shell receives exact IDs from the current
# read-only inventory and uses a temporary database with an operator grant.
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
$shell = Join-Path $workspace 'src-tauri/target/debug/audiorouter-shell.exe'
$tempRoot = Join-Path $env:TEMP ('audiorouter-vb-cable-desktop-' + [guid]::NewGuid().ToString('N'))
$database = Join-Path $tempRoot 'state.sqlite'
$params = Join-Path $tempRoot 'authorize.json'
$shellProcess = $null
$savedEnvironment = @{}

function Invoke-Checked([string] $FilePath, [string[]] $ArgumentList) {
    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

try {
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

    if ($Build) {
        Invoke-Checked 'npm.cmd' @('run', 'build', '--prefix', (Join-Path $workspace 'ui'))
        Invoke-Checked 'cargo.exe' @('build', '--manifest-path', (Join-Path $workspace 'src-tauri/Cargo.toml'))
    }
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) {
        throw "missing CLI binary: $cli (run with -Build)"
    }
    if (-not (Test-Path -LiteralPath $shell -PathType Leaf)) {
        throw "missing desktop shell: $shell (run with -Build)"
    }

    $inventoryText = @(& $cli devices list --json)
    if ($LASTEXITCODE -ne 0) {
        throw "read-only endpoint inventory failed: $($inventoryText -join "`n")"
    }
    $inventory = ($inventoryText -join "`n") | ConvertFrom-Json
    $capture = @($inventory | Where-Object {
        $_.state -eq 'active' -and
        $_.direction -eq 'capture' -and
        $_.name -eq 'CABLE Output (VB-Audio Virtual Cable)'
    })
    $render = @($inventory | Where-Object {
        $_.state -eq 'active' -and
        $_.direction -eq 'render' -and
        $_.name -eq 'CABLE Input (VB-Audio Virtual Cable)'
    })
    if ($capture.Count -ne 1 -or $render.Count -ne 1) {
        throw "refusing to launch: expected exactly one active VB-Cable capture and render endpoint (found capture=$($capture.Count), render=$($render.Count))"
    }

    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $key = 'vb-cable-desktop-' + [guid]::NewGuid().ToString('N')
    Set-Content -LiteralPath $params -NoNewline -Encoding ascii -Value (
        '{"clientId":"' + $sid + '","role":"operator","idempotencyKey":"' + $key + '"}'
    )
    Invoke-Checked $cli @('api', 'call', 'clients.authorize', $params, '--database', $database, '--json')

    foreach ($name in @(
        'AUDIOROUTER_DATABASE',
        'AUDIOROUTER_ALLOW_DEVICE_ADMIN',
        'AUDIOROUTER_CAPTURE_ENDPOINT_ID',
        'AUDIOROUTER_RENDER_ENDPOINT_ID'
    )) {
        $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    }
    $env:AUDIOROUTER_DATABASE = $database
    $env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = '1'
    $env:AUDIOROUTER_CAPTURE_ENDPOINT_ID = [string]$capture[0].id
    $env:AUDIOROUTER_RENDER_ENDPOINT_ID = [string]$render[0].id

    Write-Output "Launching disposable AudioRouter desktop with:"
    Write-Output "  capture: $($capture[0].name)"
    Write-Output "  render:  $($render[0].name)"
    Write-Output 'Use Select VB-Cable capture or Select VB-Cable loopback pair, then Prepare native endpoints, Plan changes, and Start session.'
    Write-Output 'Use the tray Quit and stop audio action when finished, or press Ctrl+C here.'
    $shellProcess = Start-Process -FilePath $shell -WorkingDirectory $workspace -PassThru
    $shellProcess.WaitForExit()
}
finally {
    if ($null -ne $shellProcess) {
        $shellProcess.Refresh()
        if (-not $shellProcess.HasExited) {
            & taskkill.exe /PID $shellProcess.Id /T /F *> $null
        }
        $shellProcess.Dispose()
    }
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
