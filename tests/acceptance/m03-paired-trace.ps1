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
    Assert ($Path -ceq (Join-Path $pairDirectory 'measure-done.json') -and $Run -ceq $pairId -and $TimeoutSeconds -eq 45) 'measurement completion wait is scoped and bounded'
    & $Guard
    return @{ Run = $Run }
}
function Invoke-CallbackBoundary { param($Directory,$Run) & $Run }
Invoke-CallbackBoundary -Directory $hostEvidence -Run $callbackExpression.ScriptBlock.GetScriptBlock()
Assert ((Read-PairedTraceJson (Join-Path $pairDirectory 'host-started.json')).Run -ceq $pairId) 'start marker publishes in shared run directory'
Assert ((Read-PairedTraceJson (Join-Path $pairDirectory 'measure-go.json')).Run -ceq $pairId) 'measurement marker keeps run identity'
Assert (($script:clockCalls -join ',') -ceq 'before,after') 'both clock stages executed'
Write-Host "$checks paired protocol checks pass. Evidence: $evidence"
