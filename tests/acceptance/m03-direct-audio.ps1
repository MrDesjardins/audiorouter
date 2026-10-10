<# Host-safe checks of production orchestration with fake process boundaries. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
$root = Join-Path $workspace ('target\direct-audio-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\run-direct-audio.ps1'), [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell wrapper syntax'
$null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\prepare-direct-audio-update.ps1'), [ref]$tokens, [ref]$errors)
Assert (-not $errors) 'Windows PowerShell package preparer syntax'
$body = $ast.Find({ param($node) $node -is [Management.Automation.Language.TryStatementAst] -and $node.Extent.Text.Contains('Automatic 30-second test:') }, $true)
$execution = [scriptblock]::Create($body.Extent.Text)

function Test-Orchestration([string] $Case) {
    $evidence = Join-Path $root $Case
    $capture = Join-Path $evidence 'capture'
    $bundle = Join-Path $evidence 'bundle'
    $peer = Join-Path $evidence 'peer'
    New-Item -ItemType Directory -Path $capture,$bundle,$peer | Out-Null
    $run = $Case
    $powershell = 'fake-powershell'
    $probe = 'fake-probe'
    $process = $null; $toneEvidence = $null; $failure = $null; $codes = @{}
    $script:calls = @(); $script:inventoryCalls = 0; $script:started = 0; $script:disposed = $false; $script:killed = $false
    function Start-Sleep { param($Milliseconds) }
    function Start-Process {
        param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru,$RedirectStandardOutput,$RedirectStandardError)
        Assert ($FilePath -eq $probe -and $WindowStyle -eq 'Hidden') 'owned probe only, hidden window'
        $script:started++
        Write-PairedTraceJson (Join-Path $capture 'ready.json') @{ rate=48000; channels=2; bits=32; format='IEEE_FLOAT' }
        Write-PairedTraceJson (Join-Path $capture 'recording.json') @{ packets=@(@{fileFrame=1000;frames=480;flags=$(if ($Case -eq 'packet-error') {1} else {0})}) }
        $fake = [pscustomobject]@{Handle=1;HasExited=$false;ExitCode=$(if ($Case -eq 'recorder-failure') {1} else {0})}
        $fake | Add-Member ScriptMethod WaitForExit { param($Milliseconds) $this.HasExited=$true; return $true }
        $fake | Add-Member ScriptMethod Dispose { $script:disposed=$true }
        $fake | Add-Member ScriptMethod Kill { $this.HasExited=$true; $script:killed=$true }
        return $fake
    }
    function Invoke-DriverVmProcess {
        param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
        $script:calls += [pscustomobject]@{Executable=$Executable;Arguments=$Arguments;Timeout=$TimeoutSeconds}
        Set-Content -LiteralPath $Stdout -Value 'fake process output'
        Set-Content -LiteralPath $Stderr -Value ''
        if ($Arguments -contains 'status') { return @{Code=$(if ($Case -eq 'status-failure') {1} else {0})} }
        if ($Arguments -contains 'tone') {
            Assert ($TimeoutSeconds -eq 90 -and $Arguments[-1] -eq '30') '30-second tone and 90-second watchdog'
            if ($Case -eq 'tone-exception') { throw 'fake native boundary failure' }
            return @{Code=$(if ($Case -eq 'tone-failure') {1} else {0})}
        }
        Assert ($Arguments[0] -eq 'analyze' -and $TimeoutSeconds -eq 20) 'bounded offline analysis'
        Write-PairedTraceJson $Arguments[-1] @{ passed=($Case -ne 'signal-failure'); fitStartFrame=480;fitEndFrame=1440000 }
        return @{Code=$(if ($Case -eq 'signal-failure') {1} else {0})}
    }
    function Get-ChildItem {
        param($LiteralPath,[switch]$Directory,$Filter)
        Assert ($LiteralPath -eq 'C:\ar\evidence' -and $Filter -eq '*-tone') 'current tone inventory only'
        $script:inventoryCalls++
        if ($script:inventoryCalls -eq 1) { return [pscustomobject]@{FullName='old-tone'} }
        $fresh = Join-Path $evidence 'new-tone'
        New-Item -ItemType Directory -Path $fresh | Out-Null
        return @([pscustomobject]@{FullName='old-tone'},[pscustomobject]@{FullName=$fresh})
    }
    function Compress-Archive {
        param($LiteralPath,$DestinationPath)
        Assert ($LiteralPath -contains $evidence -and $LiteralPath -notcontains 'old-tone') 'archive excludes previous runs'
        Set-Content -LiteralPath $DestinationPath -Value 'fake archive'
    }
    . $execution
    Assert ((-not [bool]$failure) -eq ($Case -eq 'pass')) "$Case outcome preserved"
    Assert (Test-Path -LiteralPath (Join-Path $peer "$run.zip")) "$Case evidence copied"
    if ($Case -eq 'status-failure') { Assert ($script:started -eq 0) 'status failure starts no audio' }
    else {
        Assert $script:disposed "$Case owned recorder disposed"
        Assert (Test-Path -LiteralPath (Join-Path $capture 'stop')) "$Case recorder stop requested"
    }
    if ($Case -in @('pass','tone-failure','signal-failure','packet-error')) {
        Assert ($script:calls.Count -eq 4) "$Case both recorded paths analyzed"
    }
    if ($Case -eq 'tone-exception') { Assert ([bool]$toneEvidence) 'throw still retains current tone evidence' }
}
foreach ($case in @('pass','status-failure','recorder-failure','tone-failure','signal-failure','packet-error','tone-exception')) { Test-Orchestration $case }
if ($env:COMPUTERNAME -ine 'AR-DriverTest') {
    $binary = Join-Path $workspace 'target\release\examples\m03_direct_audio.exe'
    $guard = Invoke-DriverVmProcess -Executable $binary -Arguments @('record',$root) -Stdout (Join-Path $root 'guard.txt') -Stderr (Join-Path $root 'guard-stderr.txt') -TimeoutSeconds 5
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'guard-stderr.txt') -Raw) -match 'no endpoint was opened') 'real binary refuses host recording before any audio API'
    $guard = Invoke-DriverVmProcess -Executable (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
        -Arguments @('-NoProfile','-ExecutionPolicy','Bypass','-File',(Join-Path $workspace 'tools\vm\run-direct-audio.ps1')) `
        -Stdout (Join-Path $root 'wrapper-guard.txt') -Stderr (Join-Path $root 'wrapper-guard-stderr.txt') -TimeoutSeconds 5
    Assert ($guard.Code -eq 1 -and (Get-Content -LiteralPath (Join-Path $root 'wrapper-guard-stderr.txt') -Raw) -match 'Run inside AR-DriverTest') 'real wrapper refuses host before native process startup'
}
Write-Host "$checks direct-audio orchestration checks passed. No audio stream or driver tool was opened. Evidence: $root"
