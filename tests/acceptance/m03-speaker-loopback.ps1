<# Host-safe wrapper checks; fake process/GUI boundaries, no endpoint access. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
. (Join-Path $workspace 'tests\acceptance\m03-driver-vm-support.ps1')
. (Join-Path $workspace 'tools\vm\portable-tool-support.ps1')
$root = Join-Path $workspace ('target\speaker-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:checks = 0
function Assert([bool] $Value, [string] $Name) {
    if (-not $Value) { throw "FAIL: $Name; evidence $root" }
    $script:checks++
}
$wrapper = Join-Path $workspace 'tools\vm\run-speaker-loopback.ps1'
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($wrapper,[ref]$tokens,[ref]$errors)
Assert (-not $errors) 'wrapper PowerShell syntax'
$null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $workspace 'tools\vm\prepare-speaker-loopback.ps1'),[ref]$tokens,[ref]$errors)
Assert (-not $errors) 'preparer PowerShell syntax'
foreach ($name in @('Assert-SpeakerReady','Invoke-SpeakerRecording')) {
    $definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
    Assert ([bool]$definition) "$name exists"
    . ([scriptblock]::Create($definition.Extent.Text))
}
function New-Ready {
    return @{ mode='guest-speaker-loopback'; endpointName='Speakers (High Definition Audio Device)';
        endpointId='speaker-id'; rate=44100; channels=2; seconds=30; qualification=$false; bits=32; wavTag=3 }
}
foreach ($field in @('mode','endpointName','endpointId','rate','channels','seconds','qualification','bits','wavTag')) {
    $ready = New-Ready
    $ready.Remove($field)
    $rejected = $false
    try { Assert-SpeakerReady ([pscustomobject]$ready) } catch { $rejected = $true }
    Assert $rejected "readiness missing $field rejected"
}
Assert-SpeakerReady ([pscustomobject](New-Ready))
$pcm = New-Ready; $pcm.bits=16; $pcm.wavTag=1
Assert-SpeakerReady ([pscustomobject]$pcm)
function Start-Process {
    param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru,$RedirectStandardOutput,$RedirectStandardError)
    Assert ($WindowStyle -eq 'Hidden' -and $PassThru) 'owned helper starts hidden'
    Assert ($ArgumentList -match '^"speaker-record" ') 'only speaker mode requested'
    $ready = New-Ready
    if ($script:case -eq 'invalid-ready') { $ready.rate = 48000 }
    if ($script:case -notin @('early-exit','readiness-timeout')) {
        Write-PairedTraceJson (Join-Path $script:capture 'ready.json') $ready
    }
    if ($script:case -ne 'missing-wav') { [IO.File]::WriteAllBytes((Join-Path $script:capture 'speakers.wav'),[byte[]]@(0,1)) }
    $process = [pscustomobject]@{ Handle=1; Id=4321; HasExited=($script:case -eq 'early-exit'); ExitCode=0 }
    if ($script:case -eq 'early-exit') { $process.ExitCode=-1073741515 }
    $process | Add-Member ScriptMethod WaitForExit {
        param($Milliseconds)
        if ($script:case -eq 'watchdog' -and -not $this.HasExited) { return $false }
        $this.HasExited=$true
        if ($script:case -eq 'exit-failure') { $this.ExitCode=1 }
        return $true
    }
    $process | Add-Member ScriptMethod Kill { $script:killed=$true; $this.HasExited=$true; $this.ExitCode=-1 }
    $process | Add-Member ScriptMethod Dispose { $script:disposed=$true }
    return $process
}
function Invoke-Item {
    param($LiteralPath)
    $script:playbacks++
    Assert (Test-Path -LiteralPath (Join-Path $script:capture 'ready.json')) 'playback waits for readiness'
    if ($script:case -eq 'playback-failure') { throw 'fixture playback failed' }
}
foreach ($script:case in @('pass','invalid-ready','early-exit','exit-failure','watchdog','playback-failure','missing-wav','readiness-timeout')) {
    $evidence = Join-Path $root $script:case
    New-Item -ItemType Directory -Path $evidence | Out-Null
    $script:capture = Join-Path $evidence 'capture'
    $script:killed=$false; $script:disposed=$false; $script:playbacks=0
    $failure = $null
    try { Invoke-SpeakerRecording 'fixture.exe' $evidence 'fixture.wav' } catch { $failure=[string]$_ }
    Assert (([bool]$failure) -eq ($script:case -ne 'pass')) "$script:case preserves outcome ($failure)"
    Assert $script:disposed "$script:case disposes only owned process"
    Assert (Test-Path -LiteralPath (Join-Path $evidence 'recorder-process.json')) "$script:case saves exit metadata"
    $exit = Get-Content -LiteralPath (Join-Path $evidence 'recorder-process.json') -Raw | ConvertFrom-Json
    Assert (-not $exit.Qualification) "$script:case is not qualification"
    if ($script:case -in @('early-exit','invalid-ready','readiness-timeout')) {
        Assert ($script:playbacks -eq 0) "$script:case opens no reference"
    } else { Assert ($script:playbacks -eq 1) "$script:case opens reference once" }
    if ($script:case -eq 'early-exit') { Assert ($exit.ExitCodeHex -eq '0xC0000135') 'loader code preserved' }
    if ($script:case -in @('watchdog','playback-failure','invalid-ready','readiness-timeout')) {
        Assert $script:killed "$script:case terminates its owned recorder"
    }
}
Remove-Item -LiteralPath 'Function:\Start-Process','Function:\Invoke-Item'
$binary = Join-Path $workspace 'target\vm-direct-audio\release\examples\m03_direct_audio.exe'
$imports = @(Assert-PortableVmTool $binary)
Assert ($imports.Count -gt 0) 'static helper imports inspected'
# This real subprocess rejects the host before any enumeration/open operation.
$hostGuard = Invoke-DriverVmProcess -Executable $binary -Arguments @('speaker-record',$root) `
    -Stdout (Join-Path $root 'host-guard.txt') -Stderr (Join-Path $root 'host-guard-stderr.txt') -TimeoutSeconds 5
Assert ($hostGuard.Code -eq 1) 'actual speaker mode refuses host'
Assert ((Get-Content -LiteralPath (Join-Path $root 'host-guard-stderr.txt') -Raw) -match 'restricted to AR-DriverTest; no endpoint was opened') 'host refusal precedes audio'
Write-Host "Speaker loopback offline checks: $script:checks PASS. Evidence: $root"
