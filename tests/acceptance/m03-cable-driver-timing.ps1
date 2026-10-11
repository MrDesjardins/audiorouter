<# Host-only; process and archive boundaries are fake. #>
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
. (Join-Path $workspace 'tools\vm\paired-trace-support.ps1')
$root = Join-Path $workspace ('target\driver-timing-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$checks = 0
function Assert([bool]$Ok,[string]$Name) { if (-not $Ok) { throw "FAIL: $Name" }; $script:checks++ }
$tokens=$null; $errors=$null
$runner=Join-Path $workspace 'tools\vm\run-cable-driver-timing.ps1'
$ast=[Management.Automation.Language.Parser]::ParseFile($runner,[ref]$tokens,[ref]$errors)
Assert (-not $errors) 'PowerShell syntax'
$source = Get-Content -LiteralPath $runner -Raw
foreach ($guard in @('AR-DriverTest','IS_TEST_VM','Assert-PairedTraceAdministrator','Assert-PairedTraceBundle','timingDiagnostics','2GB')) {
    Assert ($source.Contains($guard)) "guard $guard"
}
$body=$ast.Find({param($n) $n -is [Management.Automation.Language.TryStatementAst] -and $n.Parent -is [Management.Automation.Language.NamedBlockAst]},$true)
$execute=[scriptblock]::Create($body.Extent.Text)
foreach ($case in @('pass','baseline-fail','enabled-fail','enabled-timeout','archive-missing','identity-missing')) {
    $evidence=Join-Path $root "$case\evidence"; $bundle=Join-Path $root "$case\bundle"; $peer=Join-Path $root "$case\peer"
    New-Item -ItemType Directory -Path $evidence,$bundle,$peer | Out-Null
    $run=$case; $failure=$null; $children=@(); $script:called=0
    function Invoke-DriverVmProcess {
        param($Executable,$Arguments,$Stdout,$Stderr,$TimeoutSeconds)
        $script:called++
        if ($script:called -eq 1) {
            Assert ($TimeoutSeconds -eq 120) 'baseline watchdog'
            Assert (($Arguments -join ' ') -match '-Starts 1 -Impulses 1000 -TraceRoute -PeriodMode low-latency') 'baseline exact measurement'
            $inner='starts-engine-0123456789abcdef0123456789abcdef.zip'
        } else {
            Assert ($Executable -eq (Join-Path $bundle 'tools\m03-timing-trace.exe') -and $TimeoutSeconds -eq 150) 'enabled helper watchdog'
            Assert ($Arguments[0] -eq $bundle -and $Arguments[1] -eq (Join-Path $evidence 'enabled')) 'exact helper paths'
            $inner='starts-engine-1123456789abcdef0123456789abcdef.zip'
            Set-Content -LiteralPath (Join-Path $evidence 'enabled\driver-timing.csv') -Value 'fake records'
        }
        $measurement = if ($script:called -eq 1) { $Stdout } else { Join-Path $evidence 'enabled\measurement.txt' }
        Set-Content -LiteralPath $measurement -Value $(if ($case -eq 'identity-missing') { 'no archive' } else { "Current-run reports copied to Z:\bundle\$inner" })
        Set-Content -LiteralPath $Stdout -Value (Get-Content -LiteralPath $measurement)
        Set-Content -LiteralPath $Stderr -Value 'fake stderr retained'
        if ($case -ne 'archive-missing') { Set-Content -LiteralPath (Join-Path $bundle $inner) -Value 'fake current archive' }
        return @{Code=$(if ($case -eq 'baseline-fail' -and $script:called -eq 1) {1} elseif ($case -eq 'enabled-fail' -and $script:called -eq 2) {1} elseif ($case -eq 'enabled-timeout' -and $script:called -eq 2) {124} else {0})}
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
    Assert ($script:called -eq $(if ($case -eq 'baseline-fail') {1} else {2})) 'failed baseline starts no enabled measurement'
    if ($case -ne 'baseline-fail') { Assert (Test-Path (Join-Path $evidence 'enabled\driver-timing.csv')) 'raw records retained' }
}
Write-Host "$checks driver timing wrapper checks passed; no driver/audio opened. Evidence: $root"
