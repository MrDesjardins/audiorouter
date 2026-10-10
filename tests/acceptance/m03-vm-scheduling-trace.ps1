<# Host-safe ownership/lifecycle regression. All recorder calls are fake. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\vm-scheduling-trace.ps1')
$evidence = Join-Path $workspace ('target\vm-trace-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$fakeRecorder = Join-Path $evidence 'fake-recorder.exe'
Set-Content -LiteralPath $fakeRecorder -Value 'never executed' -Encoding ASCII
$checks = 0
function Assert([bool] $Passed, [string] $Name) {
    if (-not $Passed) { throw "FAIL: $Name; evidence $evidence" }
    $script:checks++
}
# Replace the production command boundary; do not invoke WPR on the host.
function Invoke-DriverVmTraceCommand {
    param($Recorder, [string[]] $Arguments, $Directory, $Name, $TimeoutSeconds = 30)
    $script:calls.Add([pscustomobject]@{ Arguments = $Arguments; Name = $Name; Timeout = $TimeoutSeconds })
    $text = ''; $code = 0
    switch ($Name) {
        'profiles' { $text = if ($script:scenario -eq 'missing-profile') { 'CPU cpu' } else { 'AudioRouterScheduling Guest scheduling' } }
        'initial-status' {
            $text = if ($script:scenario -eq 'busy') { 'WPR recording is in progress...' }
            elseif ($script:scenario -eq 'unknown-status') { 'unknown/localized status' }
            else { 'WPR is not recording' }
        }
        'start' {
            if ($script:scenario -in @('start-failure','start-cancel-failure','start-no-profiles')) { $code = -2147024846; $text = 'The request is not supported. Error code: 0x80070032' }
            elseif ($script:scenario -eq 'start-timeout') { $code = 124 }
        }
        'final-status' { if ($script:scenario -eq 'status-failure') { throw 'fake status query failure' } }
        'stop' {
            if ($script:scenario -in @('stop-failure','cancel-failure')) { $code = 1 }
            elseif ($script:scenario -ne 'missing-etl') { Set-Content -LiteralPath $Arguments[1] -Value 'fake ETL' -Encoding ASCII }
        }
        'cancel' {
            if ($script:scenario -in @('cancel-failure','start-cancel-failure')) { $code = 1 }
            elseif ($script:scenario -eq 'start-no-profiles') { $code = -984076288 }
        }
    }
    return [pscustomobject]@{ Code = $code; TimedOut = $false; Text = $text }
}
foreach ($scenarioName in @('success', 'tone-failure', 'busy', 'unknown-status', 'missing-profile', 'start-failure', 'start-timeout', 'start-cancel-failure', 'start-no-profiles', 'stop-failure', 'cancel-failure', 'status-failure', 'missing-etl')) {
    $script:scenario = $scenarioName
    $script:calls = [Collections.Generic.List[object]]::new()
    $script:runCount = 0
    $directory = Join-Path $evidence $scenarioName
    $failed = $false
    try {
        Invoke-DriverVmSchedulingRun -Recorder $fakeRecorder -Directory $directory -Run {
            $script:runCount++
            if ($script:scenario -eq 'tone-failure') { throw 'fake tone failure' }
        }
    } catch {
        $failed = $true
        if ($scenarioName -eq 'start-failure') { Assert ($_.ToString().Contains('0x80070032')) 'original recorder error reaches user output' }
    }
    Assert ($failed -eq ($scenarioName -notin @('success', 'status-failure'))) "$scenarioName preserves outcome"
    foreach ($call in @($script:calls | Where-Object Name -in @('start','stop','cancel','final-status'))) {
        Assert ($call.Arguments[-2] -eq '-instancename' -and $call.Arguments[-1] -match '^AudioRouterTone-[a-f0-9]{32}$') "$scenarioName never manipulates a global recorder"
    }
    foreach ($call in @($script:calls | Where-Object Name -eq 'start')) {
        Assert ($call.Arguments[1] -like '*AudioRouterScheduling.wprp!AudioRouterScheduling.Light') 'starts the custom profile, not GeneralProfile'
    }
    $profileQuery = @($script:calls | Where-Object Name -eq 'profiles')[0]
    Assert ($profileQuery.Arguments[1] -like '*AudioRouterScheduling.wprp') 'validates the actual custom profile'
    if ($scenarioName -in @('busy','unknown-status','missing-profile')) {
        Assert ($script:runCount -eq 0 -and @($script:calls | Where-Object Name -in @('start','stop','cancel')).Count -eq 0) "$scenarioName leaves existing recording untouched"
    } elseif ($scenarioName -in @('start-failure','start-timeout','start-cancel-failure','start-no-profiles')) {
        Assert ($script:runCount -eq 0) 'startup failure prevents tone'
        Assert (@($script:calls | Where-Object Name -eq 'cancel').Count -eq 1) 'partially started named instance is cleaned up'
        $summary = Get-Content -LiteralPath (Join-Path $directory 'trace-summary.json') -Raw | ConvertFrom-Json
        Assert ($summary.CleanupFailed -eq ($scenarioName -eq 'start-cancel-failure')) 'failed startup still reports failed cancellation; explicit no-profiles is clean'
    } else {
        Assert ($script:runCount -eq 1 -and @($script:calls | Where-Object Name -eq 'stop').Count -eq 1) "$scenarioName stops after callback even when it fails"
        if ($scenarioName -in @('stop-failure','cancel-failure','missing-etl')) {
            Assert (@($script:calls | Where-Object Name -eq 'cancel').Count -eq 1) "$scenarioName stops only its owned instance"
        } else {
            Assert (@($script:calls | Where-Object Name -eq 'cancel').Count -eq 0) "$scenarioName preserves saved recording"
        }
        $summary = Get-Content -LiteralPath (Join-Path $directory 'trace-summary.json') -Raw | ConvertFrom-Json
        Assert ($summary.RunFailed -eq ($scenarioName -eq 'tone-failure')) "$scenarioName keeps primary run failure"
        Assert ($summary.Saved -eq ($scenarioName -notin @('stop-failure','cancel-failure','missing-etl'))) "$scenarioName reports trace validity"
        Assert ($summary.ProfileSha256 -eq (Get-FileHash -LiteralPath (Join-Path $workspace 'tools\vm\AudioRouterScheduling.wprp')).Hash) 'records exact profile identity'
    }
}
# Exercise the real probe with the fake recorder, including actual bounded
# wait, start/save and zero calls into a tone process boundary.
$script:scenario = 'success'
$script:calls = [Collections.Generic.List[object]]::new()
$probeWatch = [Diagnostics.Stopwatch]::StartNew()
Invoke-DriverVmSchedulingProbe -Recorder $fakeRecorder -Directory (Join-Path $evidence 'probe')
Assert ($probeWatch.Elapsed.TotalSeconds -ge 2 -and $probeWatch.Elapsed.TotalSeconds -lt 15) 'probe callback is bounded to a short wait'
Assert (@($script:calls | Where-Object Name -eq 'start').Count -eq 1 -and @($script:calls | Where-Object Name -eq 'stop').Count -eq 1) 'probe exercises the same owned recorder lifecycle'
# Exercise the actual tone function with a fake native-process boundary.
# This catches PowerShell dynamic-scope/argument forwarding mistakes without
# running a recorder, tone executable or driver on this PC.
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\vm-checks.ps1'), [ref]$tokens, [ref]$errors)
if ($errors) { throw ($errors | Out-String) }
$toneFunction = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Invoke-ToneTool' }, $true)
. ([scriptblock]::Create($toneFunction.Extent.Text))
function Get-PSDrive { param($Name) return [pscustomobject]@{ Free = 3GB } }
function Test-Path {
    param($LiteralPath, $PathType)
    if ($LiteralPath -like '*\System32\wpr.exe') { return $true }
    return Microsoft.PowerShell.Management\Test-Path -LiteralPath $LiteralPath -PathType $PathType
}
function Invoke-DriverVmProcess {
    param($Executable, [string[]] $Arguments, $Stdout, $Stderr, $TimeoutSeconds)
    $code = 0; $text = ''
    switch ($Arguments[0]) {
        '-profiles' { $text = 'AudioRouterScheduling Guest scheduling' }
        '-status' { $text = 'WPR is not recording' }
        '-stop' {
            if ($script:scenario -eq 'integrated-stop-failure') { $code = 1 }
            else { Set-Content -LiteralPath $Arguments[1] -Value 'fake ETL' -Encoding ASCII }
        }
        '--seconds' {
            $script:nativeCalls++
            $text = $Arguments -join '|'
            if ($script:scenario -eq 'integrated-native-failure') { $code = 7 }
        }
    }
    Set-Content -LiteralPath $Stdout -Value $text -Encoding UTF8
    Set-Content -LiteralPath $Stderr -Value '' -Encoding UTF8
    return [pscustomobject]@{ Code = $code; TimedOut = $false; ProcessId = 0; ElapsedSeconds = 0 }
}
$TraceScheduling = $true
$root = Join-Path $workspace 'tools\vm'
$tone = 'never-executed-tone.exe'
foreach ($scenarioName in @('integrated-success','integrated-native-failure','integrated-stop-failure')) {
    $script:scenario = $scenarioName
    $script:nativeCalls = 0
    $integrationEvidence = Join-Path $evidence $scenarioName
    New-Item -ItemType Directory -Path $integrationEvidence | Out-Null
    # Invoke-ToneTool resolves its evidence folder from the script scope.
    $testEvidenceRoot = $evidence
    $evidence = $integrationEvidence
    try {
        $wav = Join-Path $evidence 'render-source.wav'
        $result = Invoke-ToneTool @('--seconds','30') $wav
        Assert ($script:nativeCalls -eq 1) "$scenarioName invokes only one fake tone"
        Assert ($result.Text.Contains("--seconds|30|--out|$wav")) "$scenarioName forwards native tone args rather than recorder args"
        $expectedCode = switch ($scenarioName) { 'integrated-success' { 0 } 'integrated-native-failure' { 7 } 'integrated-stop-failure' { 125 } }
        Assert ($result.Code -eq $expectedCode) "$scenarioName preserves native or trace failure code"
        if ($scenarioName -eq 'integrated-stop-failure') { Assert ($result.Text -match 'scheduling diagnostic failed') 'failed save cannot look like a clean native pass' }
    } finally { $evidence = $testEvidenceRoot }
}
# Guard the narrowed profile and probe wiring without executing a recorder.
[xml]$profileXml = Get-Content -LiteralPath (Join-Path $workspace 'tools\vm\AudioRouterScheduling.wprp') -Raw
$keywords = @($profileXml.WindowsPerformanceRecorder.Profiles.SystemProvider.Keywords.Keyword | ForEach-Object Value)
Assert (($keywords | Sort-Object) -join ',' -eq 'CSwitch,DPC,Interrupt,Loader,ProcessThread,ReadyThread,ThreadPriority') 'profile contains exactly the scheduling keywords'
Assert ($profileXml.WindowsPerformanceRecorder.Profiles.Profile.LoggingMode -eq 'File') 'profile saves the whole bounded observation'
$wrapper = Get-Content -LiteralPath (Join-Path $workspace 'tools\vm\run-packet-clock-review.ps1') -Raw
Assert ($wrapper.Contains("elseif (`$Phase -eq 'TraceProbe')") -and $wrapper.Contains('Invoke-DriverVmSchedulingProbe')) 'short recorder probe is wired'
Assert ($wrapper.IndexOf('AR-DriverTest') -lt $wrapper.IndexOf('Invoke-DriverVmSchedulingProbe')) 'VM identity guard precedes probe'
Assert ($wrapper.Contains('Compress-Archive -LiteralPath $probeDirectory')) 'probe archive excludes old audio recordings'
Write-Host "$checks host-only trace regressions pass. Evidence: $evidence"
