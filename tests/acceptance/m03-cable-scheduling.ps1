<# Host-only wrapper checks. Every native process/recorder boundary is fake. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
$root = Join-Path $workspace ('target\cable-scheduling-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$checks = 0
function Assert([bool]$Ok,[string]$Name) { if (-not $Ok) { throw "FAIL: $Name" }; $script:checks++ }
$tokens=$null; $errors=$null
$runner=Join-Path $workspace 'tools\vm\run-cable-scheduling.ps1'
$ast=[Management.Automation.Language.Parser]::ParseFile($runner,[ref]$tokens,[ref]$errors)
Assert (-not $errors) 'PowerShell syntax'
$body=$ast.Find({param($n) $n -is [Management.Automation.Language.TryStatementAst] -and $n.Parent -is [Management.Automation.Language.NamedBlockAst]},$true)
$execute=[scriptblock]::Create($body.Extent.Text)
foreach ($case in @('pass','busy','measurement-fail','measurement-timeout','archive-missing','identity-missing')) {
    $evidence=Join-Path $root "$case\evidence"; $bundle=Join-Path $root "$case\bundle"; $peer=Join-Path $root "$case\peer"
    New-Item -ItemType Directory -Path $evidence,$bundle,$peer | Out-Null
    $run=$case; $recorder='fake-wpr'; $powershell='fake-powershell'; $failure=$null; $script:cableSchedulingChild=$null; $script:called=0
    function Invoke-DriverVmSchedulingRun {
        param($Recorder,$Directory,$ProfilePath,[scriptblock]$Run)
        Assert ($Recorder -eq 'fake-wpr' -and $ProfilePath -eq (Join-Path $bundle 'AudioRouterScheduling.wprp')) 'exact owned profile'
        if ($case -eq 'busy') { throw 'existing recording preserved' }
        New-Item -ItemType Directory -Path $Directory | Out-Null
        try { & $Run } finally { Set-Content -LiteralPath (Join-Path $Directory 'scheduling.etl') -Value 'fake scheduler events' }
    }
    function Invoke-DriverVmProcess {
        param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
        $script:called++
        Assert ($Executable -eq 'fake-powershell' -and $TimeoutSeconds -eq 120) 'bounded redirected child'
        Assert (($Arguments -join ' ') -match '-Starts 1 -Impulses 1000 -TraceRoute -PeriodMode low-latency') 'one short low-latency trace'
        $inner='starts-engine-0123456789abcdef0123456789abcdef.zip'
        Set-Content -LiteralPath $Stdout -Value $(if ($case -eq 'identity-missing') { 'no archive' } else { "Current-run reports copied to Z:\bundle\$inner" })
        Set-Content -LiteralPath $Stderr -Value 'fake stderr retained'
        if ($case -ne 'archive-missing') { Set-Content -LiteralPath (Join-Path $bundle $inner) -Value 'fake measurement archive' }
        return @{Code=$(if ($case -eq 'measurement-fail') {1} elseif ($case -eq 'measurement-timeout') {124} else {0});TimedOut=($case -eq 'measurement-timeout')}
    }
    function Compress-Archive {
        param($LiteralPath,$DestinationPath)
        Assert ($LiteralPath -eq $evidence) 'only current run archived'
        Set-Content -LiteralPath $DestinationPath -Value 'fake outer archive'
    }
    . $execute
    $result=Get-Content (Join-Path $evidence 'result.json') -Raw | ConvertFrom-Json
    Assert ($result.Passed -eq ($case -eq 'pass')) 'failure cannot become success'
    Assert (Test-Path (Join-Path $peer "$run.zip")) 'failure evidence copied'
    Assert ($script:called -eq $(if ($case -eq 'busy') {0} else {1})) 'busy recorder starts no measurement'
    if ($case -ne 'busy') { Assert (Test-Path (Join-Path $evidence 'trace\scheduling.etl')) 'ETL preserved even on child failure' }
}
Write-Host "$checks focused scheduling wrapper checks passed; no WPR/audio started. Evidence: $root"
