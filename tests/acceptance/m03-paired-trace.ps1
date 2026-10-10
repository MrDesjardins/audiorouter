<# Host-safe protocol checks; no WPR or audio/driver invocation. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
$evidence = Join-Path $workspace ('target\paired-protocol-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; $evidence" }
    $script:checks++
}
$run = [Guid]::NewGuid().ToString('N')
$message = Join-Path $evidence 'message.json'
Write-PairedTraceJson $message @{ Run = $run; Number = 42 }
Assert ((Read-PairedTraceJson $message).Number -eq 42) 'complete atomic message'
$failed = $false
try { Write-PairedTraceJson $message @{ Run = $run; Number = 17 } } catch { $failed = $true }
Assert $failed 'existing protocol message refused'
Assert ((Read-PairedTraceJson $message).Number -eq 42) 'previous message preserved'
Assert (@(Get-ChildItem $evidence -Filter '*.tmp').Count -eq 0) 'failed publish removes own temporary file'
$failed = $false
try { $null = Wait-PairedTraceMessage $message 'wrong-run' 1 } catch { $failed = $true }
Assert $failed 'wrong run rejected'
$failed = $false
try { $null = Wait-PairedTraceMessage (Join-Path $evidence 'missing.json') $run 1 } catch { $failed = $true }
Assert $failed 'missing peer times out'
$abort = Join-Path $evidence 'abort.json'
Write-PairedTraceJson $abort @{ Run = $run; Error = 'fake failure' }
$failed = $false
try { $null = Wait-PairedTraceMessage (Join-Path $evidence 'missing.json') $run 1 $abort } catch { $failed = $_.ToString().Contains('fake failure') }
Assert $failed 'peer failure propagated'
$large = Join-Path $evidence 'large.json'
[IO.File]::WriteAllText($large, ('x' * 16385))
$failed = $false
try { $null = Read-PairedTraceJson $large } catch { $failed = $true }
Assert $failed 'oversized message refused'

# A separate PowerShell worker exercises real shared-file request/reply flow.
# Both clocks here are on one machine; this is no cross-machine accuracy claim.
$job = Start-Job -ScriptBlock {
    param($Support, $Directory, $Run)
    . $Support
    Reply-PairedTraceClockSamples $Directory $Run 'before'
    Reply-PairedTraceClockSamples $Directory $Run 'after'
} -ArgumentList (Join-Path $workspace 'tools\vm\paired-trace-support.ps1'),$evidence,$run
try {
    Save-PairedTraceClockSamples $evidence $run 'before' $evidence
    Save-PairedTraceClockSamples $evidence $run 'after' $evidence
    $null = Wait-Job $job -Timeout 10
    if ($job.State -ne 'Completed') { throw 'Protocol worker did not finish.' }
    Receive-Job $job -ErrorAction Stop | Out-Null
    foreach ($stage in @('before','after')) {
        $samples = @(Import-Csv (Join-Path $evidence "$stage-clock.csv"))
        Assert ($samples.Count -eq 8) "$stage complete sample count"
        foreach ($sample in $samples) {
            Assert ([double]$sample.GuestMinusHostLowerMs -le 0 -and [double]$sample.GuestMinusHostUpperMs -ge 0) 'same-machine zero offset inside bracket'
            Assert ([double]$sample.RoundTripMs -gt 0 -and [long]$sample.GuestQpcFrequency -gt 0) 'positive uncertainty and clock frequency recorded'
            Assert ([decimal]$sample.GuestMinusHostQpcLowerMs -le 0 -and [decimal]$sample.GuestMinusHostQpcUpperMs -ge 0) 'same-machine normalized QPC offset inside bounds'
        }
    }
} finally { Stop-Job $job; Remove-Job $job }

foreach ($name in @('run-paired-scheduling-host.ps1','run-paired-scheduling-guest.ps1','paired-trace-support.ps1','prepare-paired-trace-update.ps1')) {
    $tokens = $null; $errors = $null
    $null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace "tools\vm\$name"), [ref]$tokens, [ref]$errors)
    Assert (-not $errors) "$name Windows PowerShell syntax"
}
# Exercise the production callback under the recorder's actual parameter names.
# PowerShell dynamic scope would otherwise replace a caller's $run/$directory.
$hostAst = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\run-paired-scheduling-host.ps1'), [ref]$tokens, [ref]$errors)
$callbackExpression = $hostAst.Find({ param($node)
    $node -is [Management.Automation.Language.ScriptBlockExpressionAst] -and $node.Extent.Text -match 'host-started.json'
}, $true)
$pairDirectory = Join-Path $evidence 'callback-protocol'
$hostEvidence = Join-Path $evidence 'callback-trace'
$pairId = [Guid]::NewGuid().ToString('N')
$measureTimeout = 45
$maxHostTraceBytes = 1GB
$minHostTraceFree = 2GB
$pairOutcome = [pscustomobject]@{ GuestExitCode = $null }
$fakeToneExit = 0
New-Item -ItemType Directory -Path $pairDirectory,$hostEvidence | Out-Null
$script:clockCalls = @()
function Save-PairedTraceClockSamples {
    param($Directory,$Run,$Stage,$Evidence,$Guard)
    Assert ($Directory -ceq $pairDirectory -and $Run -ceq $pairId -and $Evidence -ceq $hostEvidence) 'clock callback retains caller identity and directory'
    & $Guard
    $script:clockCalls += $Stage
}
function Wait-PairedTraceMessage {
    param($Path,$Run,$TimeoutSeconds,$AbortPath,$PollMilliseconds,$Guard)
    Assert ($Path -ceq (Join-Path $pairDirectory 'measure-done.json') -and $Run -ceq $pairId -and $TimeoutSeconds -eq $measureTimeout) 'measurement completion wait is scoped and bounded'
    & $Guard
    return @{ Run = $Run; ExitCode = $fakeToneExit }
}
function Invoke-CallbackBoundary { param($Directory,$Run) & $Run }
Invoke-CallbackBoundary -Directory $hostEvidence -Run $callbackExpression.ScriptBlock.GetScriptBlock()
Assert ((Read-PairedTraceJson (Join-Path $pairDirectory 'host-started.json')).Run -ceq $pairId) 'start marker publishes in shared run directory'
Assert ((Read-PairedTraceJson (Join-Path $pairDirectory 'measure-go.json')).Run -ceq $pairId) 'measurement marker keeps run identity'
Assert (($script:clockCalls -join ',') -ceq 'before,after') 'both clock stages executed'
Assert ($pairOutcome.GuestExitCode -eq 0) 'probe completion returned from recorder scope'

$pairDirectory = Join-Path $evidence 'tone-callback-protocol'
New-Item -ItemType Directory -Path $pairDirectory | Out-Null
$pairId = [Guid]::NewGuid().ToString('N')
$measureTimeout = 540
$maxHostTraceBytes = 6GB
$minHostTraceFree = 8GB
$fakeToneExit = 1
$script:clockCalls = @()
Invoke-CallbackBoundary -Directory $hostEvidence -Run $callbackExpression.ScriptBlock.GetScriptBlock()
Assert ($pairOutcome.GuestExitCode -eq 1) 'failed tone acceptance survives successful trace callback'
Assert (($script:clockCalls -join ',') -ceq 'before,after') 'failed acceptance still gets post-run clock brackets'

# Run the production guest status/tone blocks with a fake native boundary.
# No guest identity spoofing, driver call or file operation under C:\ar occurs.
$guestAst = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\run-paired-scheduling-guest.ps1'), [ref]$tokens, [ref]$errors)
$toneIf = $guestAst.Find({ param($node) $node -is [Management.Automation.Language.IfStatementAst] -and $node.Extent.Text -match '\$beforeTone\s*=' }, $true)
$statusIf = $guestAst.Find({ param($node) $node -is [Management.Automation.Language.IfStatementAst] -and $node.Extent.Text -match '\$status\s*= Invoke-DriverVmProcess' }, $true)
$bundle = Join-Path $evidence 'fake-bundle'
$guestEvidence = Join-Path $evidence 'fake-guest'
New-Item -ItemType Directory -Path $guestEvidence | Out-Null
$ToneSeconds = 30
$script:nativeCalls = @()
$script:inventoryCalls = 0
function Invoke-DriverVmProcess {
    param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
    $script:nativeCalls += [pscustomobject]@{ Executable=$Executable; Arguments=$Arguments; Timeout=$TimeoutSeconds }
    Set-Content -LiteralPath $Stdout -Value 'fake tone failure' -Encoding UTF8
    Set-Content -LiteralPath $Stderr -Value '' -Encoding UTF8
    return @{ Code=1; TimedOut=$false }
}
function Get-ChildItem {
    param($LiteralPath,[switch]$Directory,$Filter)
    if ($LiteralPath -ne 'C:\ar\evidence' -or $Filter -ne '*-tone') { throw 'Unexpected fake inventory boundary.' }
    $script:inventoryCalls++
    if ($script:inventoryCalls -eq 1) { return [pscustomobject]@{FullName='C:\ar\evidence\old-tone'} }
    return @([pscustomobject]@{FullName='C:\ar\evidence\old-tone'},[pscustomobject]@{FullName='C:\ar\evidence\new-tone'})
}
$toneBlockText = $toneIf.Clauses[0].Item2.Extent.Text
$toneBlock = [scriptblock]::Create($toneBlockText.Substring(1,$toneBlockText.Length - 2))
. $toneBlock
Assert ($script:nativeCalls.Count -eq 1) 'guest tone executes only its one owned native boundary'
Assert (($script:nativeCalls[0].Arguments -join ' ') -match '-Step tone -ToneSeconds 30 -TraceScheduling') 'guest forwards exact duration and opt-in trace'
Assert ($script:nativeCalls[0].Timeout -eq 240) 'guest outer process watchdog bounded'
Assert ($toneExit -eq 1 -and $toneEvidence -eq 'C:\ar\evidence\new-tone') 'failed tone retains its current evidence only'
Assert (Test-Path -LiteralPath (Join-Path $guestEvidence 'tone-process.json')) 'guest process failure saved'
$ToneSeconds = 300
$script:nativeCalls = @()
$script:inventoryCalls = 0
. $toneBlock
Assert (($script:nativeCalls[0].Arguments -join ' ') -match '-Step tone -ToneSeconds 300 -TraceScheduling') 'later long diagnostic forwards matching duration'
Assert ($script:nativeCalls[0].Timeout -eq 510) 'later long diagnostic retains bounded outer watchdog'
$script:nativeCalls = @()
$statusFailed = $false
$statusBlockText = $statusIf.Clauses[0].Item2.Extent.Text
$statusBlock = [scriptblock]::Create($statusBlockText.Substring(1,$statusBlockText.Length - 2))
try { . $statusBlock } catch { $statusFailed = $_.ToString().Contains('status failed') }
Assert $statusFailed 'status failure stops progression before joining host'
Assert ($script:nativeCalls.Count -eq 1 -and $script:nativeCalls[0].Timeout -eq 60 -and ($script:nativeCalls[0].Arguments -join ' ') -match '-Step status$') 'status check is read-only and bounded'
# Extract the actual offer guard: no startup/admin/VM/device boundary is invoked.
$offerIf = $guestAst.Find({ param($node) $node -is [Management.Automation.Language.IfStatementAst] -and $node.Extent.Text -match 'No matching active host offer' }, $true)
$offerGuard = [scriptblock]::Create($offerIf.Extent.Text)
$Phase = 'Tone'
$ToneSeconds = 30
foreach ($badOffer in @($null, @{Run='invalid';Phase='Tone';ToneSeconds=30},
    @{Run=$pairId;Phase='PairProbe';ToneSeconds=30}, @{Run=$pairId;Phase='Tone';ToneSeconds=300})) {
    $offer = $badOffer
    $rejected = $false
    try { . $offerGuard } catch { $rejected = $_.ToString().Contains('No matching active host offer') }
    Assert $rejected 'missing/invalid run or mismatched phase/duration refused'
}
$offer = @{Run=$pairId;Phase='Tone';ToneSeconds=30}
. $offerGuard
Assert $true 'matching tone offer accepted'
Write-Host "$checks paired protocol checks pass. Evidence: $evidence"
