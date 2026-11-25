$ErrorActionPreference = 'Stop'
$wrapper = Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-unattended.ps1'
$root = Join-Path $env:TEMP ('ar-unattended-guards-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$passwordFile = Join-Path $root 'guest-password.txt'
$candidate = Join-Path $root 'candidate.exe'
$previous = Join-Path $root 'previous.exe'
Set-Content -LiteralPath $passwordFile -Value 'fixture-only'
Set-Content -LiteralPath $candidate -Value 'candidate fixture'
Set-Content -LiteralPath $previous -Value 'previous fixture'
$hash = 'a' * 64
$snapshot = '45d20e3c-8066-4896-9e28-a206f9d88723'
$driverVm = 'c889da39-b7d2-4800-b656-35319aeba2f5'
$counter = 0
function Invoke-Wrapper([hashtable]$Overrides, [string]$EvidenceDirectory) {
    $values = @{
        VmId = '074bd5af-a936-4e25-a2c7-d8fbd632b631'
        SnapshotId = $snapshot
        Username = 'arqualfresh'
        PasswordFile = $passwordFile
        Candidate = $candidate
        CandidateHash = $hash
        CandidateVersion = '0.0.32'
        Previous = $previous
        PreviousHash = $hash
        PreviousVersion = '0.0.29'
        OutputDirectory = $EvidenceDirectory
        RestoreDisposableSnapshot = $true
        WebViewBaseline = 'Present'
    }
    foreach ($key in $Overrides.Keys) { $values[$key] = $Overrides[$key] }
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $wrapper)
    foreach ($key in @('VmId', 'SnapshotId', 'Username', 'PasswordFile', 'Candidate', 'CandidateHash',
        'CandidateVersion', 'Previous', 'PreviousHash', 'PreviousVersion', 'OutputDirectory')) {
        $arguments += @("-$key", [string]$values[$key])
    }
    if ($values.RestoreDisposableSnapshot) { $arguments += '-RestoreDisposableSnapshot' }
    if ($values.ProductionRecording) { $arguments += '-ProductionRecording' }
    if ($values.InjectGuestControlTimeout) { $arguments += '-InjectGuestControlTimeout' }
    if ($values.OfflineBootstrapRetry) { $arguments += '-OfflineBootstrapRetry' }
    $arguments += @('-WebViewBaseline', [string]$values.WebViewBaseline)
    # A pre-try refusal writes to stderr; Stop would turn that into a terminating error.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = (& powershell.exe @arguments 2>&1 | Out-String)
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    return @{ exit = $code; output = $output }
}
function New-EvidenceDirectory {
    $script:counter++
    return Join-Path $root ('evidence-{0}' -f $script:counter)
}
function Expect-Refused([hashtable]$Overrides, [string]$Pattern, [string]$Label) {
    $output = New-EvidenceDirectory
    $run = Invoke-Wrapper $Overrides $output
    if ($run.exit -ne 1) { throw "$Label did not refuse (exit $($run.exit)): $($run.output)" }
    $resultPath = Join-Path $output 'result.json'
    if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) { throw "$Label left no result.json evidence" }
    $result = Get-Content -LiteralPath $resultPath -Raw | ConvertFrom-Json
    if ($result.status -ne 'failed' -or $result.error -notlike "*$Pattern*") {
        throw ("{0} recorded '{1}: {2}' instead of the refusal" -f $Label, $result.status, $result.error)
    }
    if (@($result.steps).Count -ne 0) { throw "$Label recorded VM steps despite a preflight refusal" }
    if (Test-Path -LiteralPath (Join-Path $output 'steps.jsonl')) { throw "$Label launched a VM command despite a preflight refusal" }
}
try {
    Expect-Refused @{ VmId = 'not-a-uuid' } '*VM and snapshot must be exact UUIDs*' 'Invalid VM UUID'
    Expect-Refused @{ SnapshotId = 'not-a-uuid' } '*VM and snapshot must be exact UUIDs*' 'Invalid snapshot UUID'
    Expect-Refused @{ VmId = $driverVm } '*driver test VM is excluded*' 'Driver test VM'
    Expect-Refused @{ Username = '-bad-user' } '*Invalid guest username*' 'Invalid guest username'
    Expect-Refused @{ Username = 'a user with spaces' } '*Invalid guest username*' 'Guest username with spaces'
    Expect-Refused @{ CandidateVersion = '0.0' } '*Invalid product version*' 'Invalid candidate version'
    Expect-Refused @{ PreviousVersion = '0.0.29-beta' } '*Invalid product version*' 'Invalid previous version'
    Expect-Refused @{ WebViewBaseline = 'Present'; OfflineBootstrapRetry = $true; ProductionRecording = $true } '*requires the absent runtime baseline*' 'Offline retry on a runtime-present baseline'
    Expect-Refused @{ WebViewBaseline = 'Absent'; OfflineBootstrapRetry = $true } '*requires the absent runtime baseline*' 'Offline retry without installed-shell verification'

    $existing = New-EvidenceDirectory
    New-Item -ItemType Directory -Path $existing | Out-Null
    $run = Invoke-Wrapper @{} $existing
    if ($run.exit -ne 1 -or $run.output -notlike '*Evidence directory must be new*') {
        throw "Existing evidence directory was not refused (exit $($run.exit)): $($run.output)"
    }
    if (Test-Path -LiteralPath (Join-Path $existing 'result.json')) { throw 'Existing evidence directory was modified' }

    $missingParent = Join-Path $root ('missing-parent-{0}/evidence' -f ([guid]::NewGuid().ToString('N')))
    $run = Invoke-Wrapper @{} $missingParent
    if ($run.exit -ne 1 -or $run.output -notlike '*Evidence directory must be new*') {
        throw "Missing evidence parent was not refused (exit $($run.exit)): $($run.output)"
    }
    if (Test-Path -LiteralPath $missingParent) { throw 'Refused evidence path was created' }

    # RQ-02 needs a reached marker, not just a bounded timeout. The run result
    # must carry that state so a missing marker cannot read as a clean timeout.
    $wrapperText = [IO.File]::ReadAllText($wrapper)
    if ($wrapperText -notmatch 'timeoutMarkerReached' -or
        $wrapperText -notmatch 'Guest timeout marker was not verified') {
        throw 'Unattended result must record whether the injected timeout marker was reached'
    }
    Write-Output 'Unattended lifecycle preflight guards passed.'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}