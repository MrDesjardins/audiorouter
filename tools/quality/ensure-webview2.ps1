param(
    [Parameter(Mandatory = $true)][string]$Output,
    [switch]$InstallMissing
)
$ErrorActionPreference = 'Stop'

# Edge Stable is not a WebView2 runtime. Microsoft's documented detection keys:
# https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
function Get-RuntimeVersions {
    $client = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    $machine = if ([Environment]::Is64BitOperatingSystem) { 'HKLM:\SOFTWARE\WOW6432Node' } else { 'HKLM:\SOFTWARE' }
    foreach ($key in @("$machine\Microsoft\EdgeUpdate\Clients\$client", "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$client")) {
        if (Test-Path -LiteralPath $key) {
            $value = (Get-ItemProperty -LiteralPath $key -Name pv -ErrorAction SilentlyContinue).pv
            $version = $null
            if ([version]::TryParse($value, [ref]$version) -and $version -gt [version]'0.0.0.0') {
                [pscustomobject]@{ key = $key; version = $value }
            }
        }
    }
}

$report = [ordered]@{
    before = @(Get-RuntimeVersions)
    after = @()
    userInteractive = [Environment]::UserInteractive
    sessionId = (Get-Process -Id $PID).SessionId
    installed = $false
    bootstrapperSha256 = $null
    error = $null
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Output) | Out-Null
try {
    if ($report.before.Count -eq 0) {
        if (-not $InstallMissing -or $env:GITHUB_ACTIONS -ne 'true') {
            throw 'WebView2 runtime is missing; automatic installation is restricted to disposable GitHub Actions runners.'
        }
        $setup = Join-Path $env:RUNNER_TEMP 'MicrosoftEdgeWebview2Setup.exe'
        Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $setup -TimeoutSec 120
        $signature = Get-AuthenticodeSignature -LiteralPath $setup
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch '(?:^|, )O=Microsoft Corporation(?:,|$)') {
            throw 'WebView2 bootstrapper must have a valid Microsoft signature.'
        }
        $report.bootstrapperSha256 = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash
        $installer = Start-Process -FilePath $setup -ArgumentList '/silent', '/install' -WindowStyle Hidden -PassThru
        # Acquire the handle before waiting so PowerShell retains the exit code.
        $handle = $installer.Handle
        if (-not $installer.WaitForExit(180000)) {
            Stop-Process -Id $installer.Id -Force -ErrorAction SilentlyContinue
            throw 'WebView2 installation timed out.'
        }
        if ($installer.ExitCode -ne 0) { throw "WebView2 installer exited $($installer.ExitCode)." }
        $report.installed = $true
    }
    $report.after = @(Get-RuntimeVersions)
    if ($report.after.Count -eq 0) { throw 'WebView2 runtime registration is still missing after setup.' }
    Write-Host "WebView2 runtime: $($report.after.version -join ', ')"
} catch {
    $report.error = $_.Exception.Message
    throw
} finally {
    $report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Output -Encoding UTF8
}
