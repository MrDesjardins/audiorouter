param(
    [switch]$Build,
    [string]$RenderEndpointId = ''
)

$ErrorActionPreference = 'Stop'

# This launcher is deliberately disposable. It never changes Windows default
# devices, endpoint volume/mute, startup registration, driver state, or the
# caller's environment. The shell receives exact IDs from the current
# read-only inventory and uses a temporary database with an operator grant.
$scriptRoot = (Resolve-Path $PSScriptRoot).Path
$repositoryRoot = Join-Path $scriptRoot '..'
if (Test-Path -LiteralPath (Join-Path $repositoryRoot 'Cargo.toml') -PathType Leaf) {
    $workspace = (Resolve-Path $repositoryRoot).Path
    $cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
    $shell = Join-Path $workspace 'src-tauri/target/debug/audiorouter-shell.exe'
} else {
    # Release preparation places this script next to the already-built shell
    # and CLI. Tauri embeds the built UI in the shell, so no source checkout is
    # needed for this mode.
    $workspace = $scriptRoot
    $cli = Join-Path $workspace 'audiorouter-cli.exe'
    $shell = Join-Path $workspace 'audiorouter-shell.exe'
}
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
        if (-not (Test-Path -LiteralPath (Join-Path $workspace 'Cargo.toml') -PathType Leaf)) {
            throw '-Build requires running the launcher from the repository checkout'
        }
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
    if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) {
        $render = @($inventory | Where-Object {
            $_.state -eq 'active' -and
            $_.direction -eq 'render' -and
            $_.name -eq 'CABLE Input (VB-Audio Virtual Cable)'
        })
    } else {
        $render = @($inventory | Where-Object {
            $_.state -eq 'active' -and
            $_.direction -eq 'render' -and
            $_.id -eq $RenderEndpointId
        })
    }
    if ($capture.Count -ne 1 -or $render.Count -ne 1) {
        $renderDescription = if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) { 'VB-Cable render' } else { "render ID '$RenderEndpointId'" }
        throw "refusing to launch: expected exactly one active VB-Cable capture and $renderDescription (found capture=$($capture.Count), render=$($render.Count))"
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
    if ([string]::IsNullOrWhiteSpace($RenderEndpointId)) {
        Write-Output 'Use Select VB-Cable capture or Select VB-Cable loopback pair, then Prepare native endpoints, Plan changes, and Start session.'
    } else {
        Write-Output 'The render endpoint was explicitly selected by ID; review the graph, then Prepare native endpoints, Plan changes, and Start session.'
    }
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
