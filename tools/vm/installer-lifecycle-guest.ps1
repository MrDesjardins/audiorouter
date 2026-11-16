[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateHash,
    [Parameter(Mandatory = $true)][string]$PreviousHash,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$PreviousVersion
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
function Step([string]$Name, [scriptblock]$Action) {
    try { & $Action; $steps.Add(@{ name = $Name; status = 'passed' }) }
    catch { $steps.Add(@{ name = $Name; status = 'failed'; error = $_.Exception.Message }); throw }
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
function Session {
    $value = (Run $cli @('--json','session','get','session-fixture','--database',$db)) | ConvertFrom-Json
    if ($value.id -ne 'session-fixture' -or $value.name -ne 'Fixture') { throw 'Persisted session did not match fixture' }
}
function Recording([string]$Expected) {
    if (-not (Test-Path -LiteralPath $recording -PathType Leaf) -or (Get-FileHash -LiteralPath $recording -Algorithm SHA256).Hash -ne $Expected) { throw 'Recording changed or disappeared' }
}
function Registration([bool]$Expected) {
    $roots = @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKCU:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall')
    $entries = @(foreach ($key in $roots) { if (Test-Path $key) { Get-ChildItem $key | ForEach-Object { Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue } | Where-Object { $_.DisplayName -eq 'AudioRouter' } } })
    if (($entries.Count -gt 0) -ne $Expected) { throw "Expected registration=$Expected, found $($entries.Count)" }
}
try {
    Step 'preflight' {
        $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = [Security.Principal.WindowsPrincipal]::new($identity)
        if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -or (whoami /groups | Out-String) -match 'S-1-5-32-544') { throw 'Guest account must not belong to Administrators' }
        if (Test-Path -LiteralPath $app) { throw 'Guest profile is not fresh: AudioRouter directory already exists' }
        if ((Get-FileHash $candidate -Algorithm SHA256).Hash -ne $CandidateHash -or (Get-FileHash $previous -Algorithm SHA256).Hash -ne $PreviousHash) { throw 'Installer hash mismatch' }
        if (-not (Test-Path -LiteralPath ('Registry::HKEY_USERS\' + $identity.User.Value))) { throw 'Guest user registry hive is not loaded; sign in interactively before running' }
        Registration $false
    }
    Step 'fresh-install' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true }
    Step 'seed-data' {
        [void](Run $cli @('--json','import',$fixture,'--database',$db)); Session
        # One second of generated mono PCM silence, not private audio.
        $bytes = [byte[]]::new(44 + 96000)
        $stream = [IO.MemoryStream]::new($bytes); $writer = [IO.BinaryWriter]::new($stream)
        $writer.Write([Text.Encoding]::ASCII.GetBytes('RIFF')); $writer.Write([int](36 + 96000)); $writer.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt ')); $writer.Write([int]16); $writer.Write([int16]1); $writer.Write([int16]1); $writer.Write([int]48000); $writer.Write([int]96000); $writer.Write([int16]2); $writer.Write([int16]16); $writer.Write([Text.Encoding]::ASCII.GetBytes('data')); $writer.Write([int]96000); $writer.Dispose()
        [IO.File]::WriteAllBytes($recording, $bytes)
        $script:recordingHash = (Get-FileHash $recording -Algorithm SHA256).Hash
    }
    Step 'repair' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; Session; Recording $recordingHash }
    Step 'rollback' { [void](Run $previous @('/S')); Version $PreviousVersion; Registration $true; Session; Recording $recordingHash }
    Step 'upgrade' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; Session; Recording $recordingHash }
    Step 'uninstall' {
        [void](Run (Join-Path $app 'uninstall.exe') @('/S'))
        if (Test-Path -LiteralPath $shell) { throw 'Shell remains after uninstall' }
        if (-not (Test-Path -LiteralPath $db)) { throw 'Database missing after uninstall' }
        Recording $recordingHash; Registration $false
    }
    Step 'reinstall' { [void](Run $candidate @('/S')); Version $CandidateVersion; Registration $true; Session; Recording $recordingHash }
    @{ status = 'passed'; steps = @($steps); recordingSha256 = $recordingHash } | ConvertTo-Json -Depth 8 -Compress
} catch {
    @{ status = 'failed'; steps = @($steps); error = $_.Exception.Message } | ConvertTo-Json -Depth 8 -Compress
    exit 1
}
