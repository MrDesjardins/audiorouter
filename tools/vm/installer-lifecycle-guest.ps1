[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateHash,
    [Parameter(Mandatory = $true)][string]$PreviousHash,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$PreviousVersion,
    [switch]$ProductionRecording,
    [ValidateSet('Any','Present','Absent')][string]$WebViewBaseline = 'Any',
    [switch]$ExpectOfflineBootstrapFailure,
    [switch]$RetryAfterOfflineBootstrap
)
$ErrorActionPreference = 'Stop'
$steps = [Collections.Generic.List[object]]::new()
$root = Join-Path $env:USERPROFILE 'ar-release-qualification'
$app = Join-Path $env:LOCALAPPDATA 'AudioRouter'
$db = Join-Path $app 'state.sqlite'
$cli = Join-Path $app 'audiorouter-cli.exe'
$shell = Join-Path $app 'audiorouter-shell.exe'
$recording = Join-Path $app 'qualification-recording.wav'
$candidate = Join-Path $root 'candidate.exe'
$previous = Join-Path $root 'previous.exe'
$fixture = Join-Path $root 'session.json'
$retryMarker = Join-Path $root 'offline-bootstrap-retry.txt'
. (Join-Path $root 'guards.ps1')
if ($ProductionRecording) { . (Join-Path $root 'recording.ps1') }
function Step([string]$Name, [scriptblock]$Action) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $step = @{ name = $Name; status = 'failed'; startedUtc = [DateTime]::UtcNow.ToString('o') }
    try {
        & $Action
        $step.status = 'passed'
    } catch {
        $step.error = $_.Exception.Message
        throw
    } finally {
        $step.endedUtc = [DateTime]::UtcNow.ToString('o')
        $step.durationMs = $timer.ElapsedMilliseconds
        $steps.Add($step)
    }
}
function Run([string]$Program, [string[]]$Arguments) {
    if ($Program -eq $candidate -or $Program -eq $previous -or $Program -eq (Join-Path $app 'uninstall.exe')) {
        $process = Start-Process -FilePath $Program -ArgumentList $Arguments -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "Installer process failed ($($process.ExitCode)): $Program" }
        return ''
    }
    $output = & $Program @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Process failed ($LASTEXITCODE): $Program : $output" }
    return ($output | Out-String)
}
function Version([string]$Expected) {
    $actual = (Get-Item -LiteralPath $shell).VersionInfo.ProductVersion
    if ($actual -ne $Expected) { throw "Expected $Expected, got $actual" }
}
$companions = @(
    'audiorouter-cli.exe',
    'audiorouter-plugin-worker.exe',
    'audiorouter-plugin-worker-x86.exe',
    'audiorouter-vst3-worker.exe',
    'audiorouter-vst3-worker-x86.exe'
)
function CandidateCompanions {
    foreach ($name in $companions) {
        if (-not (Test-Path -LiteralPath (Join-Path $app $name) -PathType Leaf)) {
            throw "Candidate companion missing: $name"
        }
    }
}
function Session {
    $value = (Run $cli @('--json','session','get','session-fixture','--database',$db)) | ConvertFrom-Json
    if ($value.id -ne 'session-fixture' -or $value.name -ne 'Fixture') { throw 'Persisted session did not match fixture' }
}
function Recording([string]$Expected) {
    if (-not (Test-Path -LiteralPath $recording -PathType Leaf) -or (Get-FileHash -LiteralPath $recording -Algorithm SHA256).Hash -ne $Expected) { throw 'Recording changed or disappeared' }
    if ($ProductionRecording) { $null = Measure-QualificationWav $recording }
}
function RegistrationCount {
    $roots = @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKCU:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall')
    $entries = @(foreach ($key in $roots) { if (Test-Path $key) { Get-ChildItem $key | ForEach-Object { Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue } | Where-Object { $_.DisplayName -eq 'AudioRouter' } } })
    return $entries.Count
}
function Registration([bool]$Expected) {
    $count = RegistrationCount
    if (($count -gt 0) -ne $Expected) { throw "Expected registration=$Expected, found $count" }
}
function WebView2Versions {
    $keys = @(
        'HKLM:\Software\Microsoft\EdgeUpdate\Clients',
        'HKLM:\Software\WOW6432Node\Microsoft\EdgeUpdate\Clients',
        'HKCU:\Software\Microsoft\EdgeUpdate\Clients'
    )
    $entries = @(foreach ($key in $keys) {
        if (Test-Path -LiteralPath $key) {
            Get-ChildItem -LiteralPath $key | ForEach-Object {
                Get-ItemProperty -LiteralPath $_.PSPath
            }
        }
    })
    Get-LifecycleWebViewVersions $entries
}
try {
    Step 'preflight' {
        $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = [Security.Principal.WindowsPrincipal]::new($identity)
        $admin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -or (whoami /groups | Out-String) -match 'S-1-5-32-544'
        $retryOwned = $RetryAfterOfflineBootstrap -and (Test-Path -LiteralPath $retryMarker -PathType Leaf) -and
            ((Get-Content -LiteralPath $retryMarker -Raw).Trim() -ceq $CandidateHash) -and
            -not (Test-Path -LiteralPath $app) -and (RegistrationCount) -eq 0
        if ($RetryAfterOfflineBootstrap -and -not $retryOwned) { throw 'Retry does not match a clean, owned offline setup attempt' }
        Assert-LifecycleGuestProfile $admin (Test-Path -LiteralPath $app) (Test-Path -LiteralPath ('Registry::HKEY_USERS\' + $identity.User.Value))
        if ((Get-FileHash $candidate -Algorithm SHA256).Hash -ne $CandidateHash -or (Get-FileHash $previous -Algorithm SHA256).Hash -ne $PreviousHash) { throw 'Installer hash mismatch' }
        Registration $false
    }
    $webViewBefore = @(WebView2Versions)
    Step 'webview-baseline' {
        Assert-LifecycleWebViewBaseline $WebViewBaseline $webViewBefore
    }
    if ($ExpectOfflineBootstrapFailure) {
        if ($WebViewBaseline -ne 'Absent' -or $RetryAfterOfflineBootstrap -or -not $ProductionRecording) { throw 'Invalid offline bootstrap scenario' }
        Step 'offline-bootstrap' {
            $setup = Start-Process -FilePath $candidate -ArgumentList '/S' -Wait -PassThru
            try { $script:offlineExit = $setup.ExitCode } finally { $setup.Dispose() }
            $script:webViewAfter = @(WebView2Versions)
            $registrationPresent = (RegistrationCount) -gt 0
            $filesPresent = Test-Path -LiteralPath $app
            $databasePresent = Test-Path -LiteralPath $db
            $shellProbe = 'absent'
            $shellProbeError = $null
            if (Test-Path -LiteralPath $shell -PathType Leaf) {
                try { $shellProbe = New-QualificationRecording $shell $root -ProbeOnly }
                catch {
                    $shellProbe = 'unverified'
                    $shellProbeError = $_.Exception.Message
                }
            }
            # The bundled NSIS installer does not expose a verified,
            # bootstrap-specific failure diagnostic through this harness.
            $status = Get-LifecycleOfflineOutcome $offlineExit ($webViewAfter.Count -gt 0) $registrationPresent $filesPresent $databasePresent $shellProbe $false
            $retryEligible = $status -in @('confirmed-bootstrap-refusal','unattributed-installer-failure')
            if ($retryEligible) { [IO.File]::WriteAllText($retryMarker, $CandidateHash) }
            $script:offlineReport = @{
                status = $status; steps = $steps; installerExitCode = $offlineExit
                webViewBefore = $webViewBefore; webViewAfter = $webViewAfter
                registrationPresent = $registrationPresent; filesPresent = $filesPresent
                databasePresent = $databasePresent; shellProbe = $shellProbe
                shellProbeError = $shellProbeError; retryEligible = $retryEligible
            }
        }
        $offlineReport | ConvertTo-Json -Depth 8 -Compress
        exit 0
    }
    Step 'fresh-install' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; CandidateCompanions }
    $webViewAfter = @(WebView2Versions)
    Step 'webview-after-setup' {
        if ($WebViewBaseline -ne 'Any' -and $webViewAfter.Count -eq 0) { throw 'WebView2 Runtime missing after successful setup exit' }
    }
    Step 'seed-data' {
        [void](Run $cli @('--json','import',$fixture,'--database',$db)); Session
        if ($ProductionRecording) {
            [void](Run $cli @('--json','import',(Join-Path $root 'recorder-session.json'),'--database',$db))
            $script:recording = New-QualificationRecording $shell $root
            $script:recordingMetrics = Measure-QualificationWav $recording
            $script:recordingHash = $recordingMetrics.sha256
            return
        }
        # One second of generated mono PCM silence, not private audio.
        $bytes = [byte[]]::new(44 + 96000)
        $stream = [IO.MemoryStream]::new($bytes); $writer = [IO.BinaryWriter]::new($stream)
        $writer.Write([Text.Encoding]::ASCII.GetBytes('RIFF')); $writer.Write([int](36 + 96000)); $writer.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt ')); $writer.Write([int]16); $writer.Write([int16]1); $writer.Write([int16]1); $writer.Write([int]48000); $writer.Write([int]96000); $writer.Write([int16]2); $writer.Write([int16]16); $writer.Write([Text.Encoding]::ASCII.GetBytes('data')); $writer.Write([int]96000); $writer.Dispose()
        [IO.File]::WriteAllBytes($recording, $bytes)
        $script:recordingHash = (Get-FileHash $recording -Algorithm SHA256).Hash
    }
    Step 'repair' {
        $companion = Join-Path $app 'audiorouter-plugin-worker.exe'
        $expectedHash = (Get-FileHash -LiteralPath $companion -Algorithm SHA256).Hash
        Remove-Item -LiteralPath $companion
        [void](Run $candidate @('/S'))
        Version $CandidateVersion
        Registration $true
        CandidateCompanions
        if ((Get-FileHash -LiteralPath $companion -Algorithm SHA256).Hash -ne $expectedHash) { throw 'Repair restored different companion bytes' }
        Session
        Recording $recordingHash
    }
    Step 'rollback' { [void](Run $previous @('/S')); Version $PreviousVersion; Registration $true; Session; Recording $recordingHash }
    Step 'upgrade' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; CandidateCompanions; Session; Recording $recordingHash }
    Step 'uninstall' {
        [void](Run (Join-Path $app 'uninstall.exe') @('/S'))
        if (Test-Path -LiteralPath $shell) { throw 'Shell remains after uninstall' }
        if (-not (Test-Path -LiteralPath $db)) { throw 'Database missing after uninstall' }
        Recording $recordingHash; Registration $false
    }
    Step 'reinstall' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; CandidateCompanions; Session; Recording $recordingHash }
    @{ status = 'passed'; steps = @($steps); recordingSha256 = $recordingHash; productionRecording = [bool]$ProductionRecording; recordingMetrics = $recordingMetrics; webViewBefore = $webViewBefore; webViewAfter = $webViewAfter } | ConvertTo-Json -Depth 8 -Compress
} catch {
    $failure = @{ status = 'failed'; steps = @($steps); error = $_.Exception.Message; webViewBefore = $webViewBefore; webViewAfter = $webViewAfter }
    try { $failure.webViewAfter = @(WebView2Versions) }
    catch { $failure.webViewInventoryError = $_.Exception.Message }
    $failure | ConvertTo-Json -Depth 8 -Compress
    exit 1
}
