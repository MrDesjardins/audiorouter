$ErrorActionPreference = 'Stop'

$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$tempRoot = Join-Path $env:TEMP ("audiorouter-shell-rpc-" + [guid]::NewGuid().ToString('N'))
$database = Join-Path $tempRoot 'state.sqlite'
$params = Join-Path $tempRoot 'authorize.json'
$marker = Join-Path $tempRoot 'frontend-response.json'
$pipe = "\\.\pipe\audiorouter-shell-rpc-" + [guid]::NewGuid().ToString('N')
$processes = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()

function Start-IsolatedProcess([string] $file, [string[]] $arguments, [hashtable] $environment) {
    $info = [System.Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $file
    $info.WorkingDirectory = $workspace
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Arguments = (($arguments | ForEach-Object { '"' + $_.Replace('"', '\\"') + '"' }) -join ' ')
    foreach ($entry in $environment.GetEnumerator()) { $info.Environment[$entry.Key] = $entry.Value }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $info
    if (-not $process.Start()) { throw "could not start $file" }
    [void] $processes.Add($process)
    return $process
}

try {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'M07 frontend-owned shell acceptance requires an elevated PowerShell session because the native shell WebView setup is administrator-only on this machine'
    }
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $cli = Join-Path $workspace 'target/debug/audiorouter-cli.exe'
    $shell = Join-Path $workspace 'src-tauri/target/debug/audiorouter-shell.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf)) { throw "missing CLI binary: $cli" }
    if (-not (Test-Path -LiteralPath $shell -PathType Leaf)) { throw "missing shell binary: $shell" }

    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    Set-Content -LiteralPath $params -NoNewline -Encoding ascii -Value ('{"clientId":"' + $sid + '","role":"observer","idempotencyKey":"shell-rpc-' + [guid]::NewGuid().ToString('N') + '"}')
    $enroll = & $cli api call clients.authorize $params --database $database --json 2>&1
    if ($LASTEXITCODE -ne 0) { throw "client enrollment failed: $enroll" }

    $backend = Start-IsolatedProcess $cli @('backend', 'serve', '--database', $database, '--pipe', $pipe, '--connections', '1') @{}
    Start-Sleep -Milliseconds 750
    $frontend = Start-IsolatedProcess $shell @{} @{
        'AUDIOROUTER_CONTROL_PIPE' = $pipe
        'AUDIOROUTER_SHELL_PROBE_FILE' = $marker
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    while (-not (Test-Path -LiteralPath $marker -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) {
        if ($frontend.HasExited) { throw "shell exited before frontend probe marker was written" }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { throw 'frontend system.describe probe did not reach the native command' }
    $response = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
    if ($null -ne $response.error) { throw "frontend probe returned an RPC error: $($response.error.message)" }
    if ($response.result.protocolVersion.major -ne 1) { throw 'frontend probe returned an unexpected protocol version' }
    Write-Output 'M07 frontend-owned Tauri RPC acceptance passed'
    Write-Output 'Scope: WebView initialization -> Tauri command -> authenticated backend system.describe; no audio endpoint or persistent machine configuration.'
}
finally {
    foreach ($process in $processes) {
        if ($null -ne $process -and -not $process.HasExited) {
            & taskkill.exe /PID $process.Id /T /F *> $null
        }
        if ($null -ne $process) { $process.Dispose() }
    }
    if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue }
}
