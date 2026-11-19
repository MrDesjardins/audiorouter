$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-guards.ps1')
. (Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-recording.ps1')
function Reject([scriptblock]$Action, [string]$Expected) {
    try { & $Action } catch {
        if ($_.Exception.Message -like $Expected) { return }
        throw
    }
    throw "Expected failure: $Expected"
}
$snapshot = '74a7f354-c6c4-4be1-ac2d-81a734c08996'
Assert-LifecycleSnapshot @(('SnapshotUUID-1="{0}"' -f $snapshot)) $snapshot
Reject { Assert-LifecycleSnapshot @(('SnapshotDescription="{0}"' -f $snapshot)) $snapshot } '*does not belong*'
Assert-LifecycleGuestProfile $false $false $true
Reject { Assert-LifecycleGuestProfile $true $false $true } '*Administrators*'
Reject { Assert-LifecycleGuestProfile $false $true $true } '*already exists*'
Reject { Assert-LifecycleGuestProfile $false $false $false } '*hive is not loaded*'
$root = Join-Path $env:PUBLIC ('ar-stage-' + [guid]::NewGuid().ToString('N'))
& cmd.exe /c (New-LifecycleStagingCommand $root)
if ($LASTEXITCODE -ne 0) { throw 'Fresh staging failed' }
$marker = Join-Path $root 'owned.txt'
try {
    Set-Content -LiteralPath $marker -Value 'preserve'
    & cmd.exe /c (New-LifecycleStagingCommand $root)
    if ($LASTEXITCODE -ne 3 -or (Get-Content -LiteralPath $marker) -ne 'preserve') { throw 'Existing staging was not preserved' }
} finally {
    if (Test-Path -LiteralPath $marker) { Remove-Item -LiteralPath $marker }
    [IO.Directory]::Delete($root)
}
$versions = @(Get-LifecycleWebViewVersions @(
    @{name='WebView2 Runtime';pv='157.0.1.2'}, @{name='WebView2 Runtime';pv='0.0.0.0'},
    @{name='Edge';pv='158.0.1.2'}, @{name='WebView2 Runtime';pv='157.0.1.2'}
))
if (($versions -join ',') -ne '157.0.1.2') { throw 'Unexpected runtime inventory' }
Assert-LifecycleWebViewBaseline Present $versions
Assert-LifecycleWebViewBaseline Absent @()
Reject { Assert-LifecycleWebViewBaseline Present @() } '*expected WebView2 baseline*'
Reject { Assert-LifecycleWebViewBaseline Absent $versions } '*expected WebView2 baseline*'
$linkEvents = [Collections.Generic.List[string]]::new()
$transport = @{}
Invoke-LifecycleOfflineAttempt { $linkEvents.Add('off') } { throw 'offline fixture' } { $linkEvents.Add('on'); throw 'reconnect fixture' } $transport
$saved = ($transport | ConvertTo-Json -Depth 4) | ConvertFrom-Json
if (($linkEvents -join ',') -ne 'off,on' -or $saved.primaryFailure.phase -ne 'setup' -or
    $saved.primaryFailure.message -ne 'offline fixture' -or $saved.reconnectFailure.phase -ne 'reconnect' -or
    $saved.reconnectFailure.message -ne 'reconnect fixture') { throw 'Both failures must survive evidence serialization' }
$linkEvents.Clear()
$transport = @{}
Invoke-LifecycleOfflineAttempt { $linkEvents.Add('off') } { 'guest-report' } { $linkEvents.Add('on') } $transport
if ($transport.output[0] -ne 'guest-report' -or ($linkEvents -join ',') -ne 'off,on') { throw 'Unexpected offline result or link order' }
foreach ($case in @(
    @{ input = @(0,$false,$false,$false,$false,'absent',$false); expected = 'unexpected-success-exit' },
    @{ input = @(1,$false,$false,$false,$false,'absent',$false); expected = 'unattributed-installer-failure' },
    @{ input = @(1,$false,$false,$false,$false,'absent',$true); expected = 'confirmed-bootstrap-refusal' },
    @{ input = @(0,$false,$false,$false,$false,'absent',$true); expected = 'unexpected-success-exit' },
    @{ input = @(0,$false,$true,$false,$false,'absent',$false); expected = 'partial-install' },
    @{ input = @(1,$false,$false,$true,$false,'unverified',$false); expected = 'partial-install' },
    @{ input = @(1,$false,$false,$false,$true,'absent',$false); expected = 'partial-install' },
    @{ input = @(1,$true,$false,$false,$false,'absent',$false); expected = 'unexpected-offline-success' },
    @{ input = @(0,$false,$true,$true,$false,'usable',$false); expected = 'unexpected-offline-success' }
)) {
    $arguments = $case.input
    $outcome = Get-LifecycleOfflineOutcome @arguments
    if ($outcome -ne $case.expected) { throw "Expected $($case.expected), got $outcome" }
    $retry = [Collections.Generic.List[string]]::new()
    $report = @{ status = $outcome; retryEligible = ($outcome -in @('confirmed-bootstrap-refusal','unattributed-installer-failure')) }
    if ($report.retryEligible) {
        Invoke-LifecycleOnlineRetry $report { $retry.Add('same-guest') }
        if ($retry.Count -ne 1) { throw 'Eligible clean guest did not retry' }
    } else {
        Reject { Invoke-LifecycleOnlineRetry $report { $retry.Add('same-guest') } } '*cannot be safely retried*'
        if ($retry.Count -ne 0) { throw 'Unsafe retry ran' }
    }
}
$frames = [Collections.Generic.Queue[object]]::new()
function Frame([string]$Text, [bool]$End) { $frames.Enqueue(@{ text=$Text; end=$End }) }
$receive = {
    param($buffer, $token)
    $frame = $frames.Dequeue()
    $bytes = [Text.Encoding]::UTF8.GetBytes($frame.text)
    [Array]::Copy($bytes, $buffer, $bytes.Length)
    @{ Count=$bytes.Length; EndOfMessage=$frame.end; MessageType=[Net.WebSockets.WebSocketMessageType]::Text }
}
$none = [Threading.CancellationToken]::None
Frame '{"method":"event"}' $true
Frame '{"id":1,"result":{"result":' $false
Frame '{"value":{"id":"expected","result":42}}}}' $true
if ((Read-QualificationRpc $receive $none 'fixture' 'expected') -ne 42) { throw 'Fragmented RPC response was lost' }
Frame '{"id":1,"result":{"result":{"value":{"id":"expected","error":{"message":"denied"}}}}}' $true
Reject { Read-QualificationRpc $receive $none 'fixture' 'expected' } '*denied*'
Frame '{"id":1,"result":{"result":{"value":{"id":"other","result":42}}}}' $true
Reject { Read-QualificationRpc $receive $none 'fixture' 'expected' } '*RPC fixture failed*'
$deadline = [Threading.CancellationTokenSource]::new()
try {
    $deadline.Cancel()
    Reject { Read-QualificationRpc { throw 'Receive must not run' } $deadline.Token 'fixture' 'expected' } '*canceled*'
} finally { $deadline.Dispose() }
$cleaned = [Collections.Generic.List[string]]::new()
Complete-QualificationCleanup @({throw 'fixture failure'}, {$cleaned.Add('disposed')}) 'primary'
if ($cleaned.Count -ne 1) { throw 'Cleanup stopped after first failure' }
Reject { Complete-QualificationCleanup @({throw 'fixture failure'}) $null } '*fixture failure*'
$originalProfile = $env:WEBVIEW2_USER_DATA_FOLDER
$originalArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
try {
    $env:WEBVIEW2_USER_DATA_FOLDER = 'fixture-profile'
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = 'fixture-arguments'
    Reject { New-QualificationRecording (Join-Path $root 'absent.exe') $root } '*cannot find*'
    if ($env:WEBVIEW2_USER_DATA_FOLDER -ne 'fixture-profile' -or
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ne 'fixture-arguments') {
        throw 'Failed shell launch left modified WebView environment'
    }
} finally {
    $env:WEBVIEW2_USER_DATA_FOLDER = $originalProfile
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $originalArguments
}
Write-Output 'Synthetic lifecycle guards and recorder transport passed.'
