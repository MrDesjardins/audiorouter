[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Vm,
    [Parameter(Mandatory = $true)][string]$Username,
    [Parameter(Mandatory = $true)][string]$PasswordFile,
    [Parameter(Mandatory = $true)][string]$Candidate,
    [Parameter(Mandatory = $true)][string]$CandidateHash,
    [Parameter(Mandatory = $true)][string]$CandidateVersion,
    [Parameter(Mandatory = $true)][string]$Previous,
    [Parameter(Mandatory = $true)][string]$PreviousHash,
    [Parameter(Mandatory = $true)][string]$PreviousVersion,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$vbox = Join-Path $env:ProgramFiles 'Oracle\VirtualBox\VBoxManage.exe'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw 'Evidence directory already exists' }
if (-not (Test-Path -LiteralPath (Split-Path -Parent $output) -PathType Container)) { throw 'Evidence parent does not exist' }
if ($Username -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,19}$') { throw 'Invalid guest username' }
foreach ($hash in @($CandidateHash, $PreviousHash)) { if ($hash -notmatch '^[a-fA-F0-9]{64}$') { throw 'Expected SHA-256 hashes' } }
foreach ($version in @($CandidateVersion, $PreviousVersion)) { if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected numeric product version' } }
foreach ($path in @($PasswordFile, $Candidate, $Previous, $vbox)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing input: $path" } }
if ((Get-FileHash $Candidate -Algorithm SHA256).Hash -ne $CandidateHash -or (Get-FileHash $Previous -Algorithm SHA256).Hash -ne $PreviousHash) { throw 'Host installer hash mismatch' }
New-Item -ItemType Directory -Path $output | Out-Null
$resultPath = Join-Path $output 'result.json'
$result = [ordered]@{ status = 'failed'; vm = $Vm; username = $Username; candidateHash = $CandidateHash; previousHash = $PreviousHash; candidateVersion = $CandidateVersion; previousVersion = $PreviousVersion; guest = $null; error = $null }
try {
    $auth = @('--username', $Username, '--passwordfile', (Resolve-Path -LiteralPath $PasswordFile).Path)
    $argsRun = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '30000', '--exe', 'C:\Windows\System32\cmd.exe', '--wait-stdout', '--', '/c', 'echo %USERPROFILE%')
    $profileOutput = @(& $vbox @argsRun 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "Guest profile lookup failed: $profileOutput" }
    $profile = ($profileOutput | Out-String).Trim()
    if ($profile -notmatch '^C:\\Users\\[A-Za-z0-9_.-]+$') { throw "Unexpected guest profile: $profile" }
    $guestDir = "$profile\ar-release-qualification"
    $mkdir = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '30000', '--exe', 'C:\Windows\System32\cmd.exe', '--wait-stdout', '--', '/c', "if exist $guestDir (exit /b 3) else (mkdir $guestDir)")
    & $vbox @mkdir
    if ($LASTEXITCODE -ne 0) { throw 'Guest staging directory exists or could not be created' }
    $copies = @(@($Candidate, "$guestDir\candidate.exe"), @($Previous, "$guestDir\previous.exe"), @((Join-Path $repo 'tests\fixtures\valid-session.json'), "$guestDir\session.json"), @((Join-Path $PSScriptRoot 'installer-lifecycle-guest.ps1'), "$guestDir\run.ps1"))
    foreach ($copy in $copies) {
        $copyArgs = @('guestcontrol', $Vm, 'copyto') + $auth + @($copy[0], $copy[1])
        & $vbox @copyArgs
        if ($LASTEXITCODE -ne 0) { throw "Guest copy failed: $($copy[1])" }
    }
    $run = @('guestcontrol', $Vm, 'run') + $auth + @('--profile', '--timeout', '180000', '--exe', 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe', '--wait-stdout', '--wait-stderr', '--', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "$guestDir\run.ps1", '-CandidateHash', $CandidateHash, '-PreviousHash', $PreviousHash, '-CandidateVersion', $CandidateVersion, '-PreviousVersion', $PreviousVersion)
    $guestOutput = @(& $vbox @run 2>&1)
    $guestExit = $LASTEXITCODE
    $guestText = $guestOutput | Out-String
    [IO.File]::WriteAllText((Join-Path $output 'guest-output.txt'), $guestText)
    try { $result.guest = $guestText | ConvertFrom-Json } catch { throw "Guest did not produce valid JSON (exit $guestExit)" }
    if ($guestExit -ne 0 -or $result.guest.status -ne 'passed') { throw "Guest lifecycle failed (exit $guestExit)" }
    $result.status = 'passed'
} catch {
    $result.error = $_.Exception.Message
} finally {
    [IO.File]::WriteAllText($resultPath, ($result | ConvertTo-Json -Depth 12))
}
Write-Output "Lifecycle $($result.status): $resultPath"
if ($result.status -ne 'passed') { exit 1 }
