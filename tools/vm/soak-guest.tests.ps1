<#
  Host check of the real soak-guest.ps1 loop. A copy runs in a temporary folder
  with its VM paths replaced, stub audio tools and Windows' own "Null" driver in
  place of the cable. No VM, cable or audio device is used. PS 5.1:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\soak-guest.tests.ps1
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'soak-support.ps1')
$script:failures = 0
function Assert-That([string] $Name, [bool] $Condition, [string] $Detail = '') {
    if ($Condition) { Write-Host "PASS $Name" } else { Write-Host "FAIL $Name $Detail" -ForegroundColor Red; $script:failures++ }
}
$temp = Join-Path ([IO.Path]::GetTempPath()) ('soak-guest-' + [Guid]::NewGuid().ToString('N'))
$ar = Join-Path $temp 'ar'
$z = Join-Path $temp 'z'
$bundle = Join-Path $ar 'bundle'
$soak = Join-Path $z 'soak'
New-Item -ItemType Directory -Force -Path "$bundle\driver", "$bundle\tools", "$z\bundle", $soak, "$temp\work" | Out-Null

$text = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'soak-guest.ps1'))
$text = $text.Replace('C:\ar', $ar).Replace('Z:\', "$z\").Replace("Name='AudioRouterVirtual'", "Name='Null'").Replace('m00-probe.exe', 'm00-probe.cmd')
$text = $text.Replace('Global\AudioRouterSoakGuest', 'Local\AudioRouterSoakGuestTest').Replace("'m03_bridge_tone', 'm00-probe'", "'no-such-process'")
$text = $text.Replace('"$env:SystemRoot\Minidump\*.dmp", "$env:SystemRoot\MEMORY.DMP"', "`"$temp\work\*.dmp`"")
[IO.File]::WriteAllText("$bundle\soak-guest.ps1", $text)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'soak-support.ps1'), (Join-Path $PSScriptRoot 'round2-support.ps1') -Destination $bundle
Copy-Item -LiteralPath "$env:SystemRoot\System32\drivers\null.sys" -Destination "$bundle\driver\audioroutervirtual.sys"
'{"driverVersion":"0.2.5.0"}' | Set-Content -LiteralPath "$bundle\driver\package.json" -Encoding ASCII
'function Assert-PairedTraceAdministrator { }; function Assert-PairedTraceBundle { }' | Set-Content -LiteralPath "$bundle\paired-trace-support.ps1" -Encoding ASCII
'exit 0' | Set-Content -LiteralPath "$bundle\quiet-guest.ps1" -Encoding ASCII
$lines = 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H' | ForEach-Object { "@echo render[0] name=AudioRouter Cable $_ Input (AudioRouter Virtual Cable) id=x" }
$lines | Set-Content -LiteralPath "$bundle\tools\m00-probe.cmd" -Encoding ASCII
# Stub test: mode.txt chooses clean, corrupted, crash or a long run; it writes
# the same evidence folder and zip as the real one.
@"
param([int] `$Minutes, [int] `$RouteCount)
`$mode = (Get-Content -LiteralPath '$temp\work\mode.txt').Trim()
Start-Sleep -Seconds `$(if (`$mode -eq 'long') { 120 } else { 4 })
`$run = 'continuity-' + [Guid]::NewGuid().ToString('N')
New-Item -ItemType Directory -Path "$ar\evidence\`$run" | Out-Null
if (`$mode -eq 'crash') { exit 1 }
`$bad = if (`$mode -eq 'corrupted') { 1 } else { 0 }
`$rows = @(1..`$RouteCount | ForEach-Object { @{ Pair = `$_; ProbeCode = 0; RouteCode = 0; Emitted = `$Minutes * 6000; Lost = 0; Corrupted = `$bad } })
@{ Passed = (`$bad -eq 0); Verdict = 'clean'; Error = ''; Results = `$rows } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath "$ar\evidence\`$run\result.json" -Encoding UTF8
'zip' | Set-Content -LiteralPath "$bundle\`$run.zip"
'zip' | Set-Content -LiteralPath "$z\bundle\`$run.zip"
"@ | Set-Content -LiteralPath "$bundle\run-cable-continuity.ps1" -Encoding ASCII

function Set-Mode([string] $Mode) { Set-Content -LiteralPath "$temp\work\mode.txt" -Value $Mode -Encoding ASCII }
function Set-Control([string] $State, [int] $Session) {
    Write-SoakJson "$soak\control.json" @{ state = $State; session = $Session; minutes = 20; routes = 2; loadFile = 'load.csv' }
}
function Get-Records { return @(Get-ChildItem -Path "$soak\records\*.json" -ErrorAction SilentlyContinue | Sort-Object Name | ForEach-Object { Read-SoakJson $_.FullName }) }
# Starts the sampler stand-in, one row per second, so each short segment has rows.
$sampler = { param($Path) while ($true) { [IO.File]::AppendAllText($Path, ('{0},5,3,2,12000,0' -f [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) + "`r`n"); Start-Sleep -Seconds 1 } }
function Wait-For([scriptblock] $Test, [int] $Seconds = 40) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.Elapsed.TotalSeconds -lt $Seconds) { if (& $Test) { return $true }; Start-Sleep -Milliseconds 250 }
    return $false
}
$job = $null
$guest = $null
$name = $env:COMPUTERNAME
try {
    Set-Mode 'clean'
    Set-Control 'pause' 1
    $job = Start-Job -ScriptBlock $sampler -ArgumentList "$soak\load.csv"
    $env:COMPUTERNAME = 'AR-DriverTest'
    $guest = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -WindowStyle Hidden -PassThru `
        -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$bundle\soak-guest.ps1`"" -RedirectStandardOutput "$temp\work\guest.txt" -RedirectStandardError "$temp\work\guest.err"
    $null = $guest.Handle
    $env:COMPUTERNAME = $name
    Assert-That 'waits while the PC says pause' ((Wait-For { (Read-SoakJson "$soak\guest.json").state -eq 'waiting' }) -and @(Get-Records).Count -eq 0)
    Set-Control 'run' 2
    Assert-That 'a clean segment counts 20 minutes' ((Wait-For { @(Get-Records | Where-Object { $_.outcome -eq 'valid' -and $_.validSeconds -eq 1200 -and $_.session -eq 2 }).Count -ge 1 })) (Get-Records | ConvertTo-Json -Depth 3)
    Assert-That 'raw files of a clean segment are removed' ((Wait-For { @(Get-ChildItem -Path "$ar\evidence", "$z\bundle").Count -eq 0 } 20))

    Set-Mode 'long'
    $before = @(Get-Records).Count
    Assert-That 'a long segment starts' (Wait-For { $g = Read-SoakJson "$soak\guest.json"; $g.state -eq 'running' -and $g.segment -gt $before })
    Start-Sleep -Seconds 7
    $before = @(Get-Records).Count
    Set-Mode 'clean'
    Set-Control 'run' 3   # saved and resumed: the guest sees only a new session
    Assert-That 'a resumed segment is dropped' ((Wait-For { @(Get-Records | Where-Object { $_.reason -eq 'paused' -and $_.validSeconds -eq 0 }).Count -ge 1 }))
    Assert-That 'the next segment counts again' (Wait-For { @(Get-Records | Where-Object { $_.outcome -eq 'valid' -and $_.session -eq 3 }).Count -ge 1 })

    Set-Mode 'corrupted'
    Assert-That 'corrupted audio fails and keeps the zip' ((Wait-For { @(Get-Records | Where-Object { $_.outcome -eq 'fail' }).Count -ge 1 }) -and (Wait-For { @(Get-ChildItem "$soak\evidence\*.zip").Count -ge 1 } 10))
    'x' | Set-Content -LiteralPath "$temp\work\new.dmp"
    Assert-That 'a new crash dump is recorded as a failure' (Wait-For { @(Get-Records | Where-Object { $_.reason -like 'Windows crash dump*' -and $_.sysSha256 }).Count -eq 1 })
    Set-Mode 'crash'
    Assert-That 'a test without a result goes to review' (Wait-For { @(Get-Records | Where-Object { $_.outcome -eq 'review' -and $_.reason -like 'no result*' }).Count -ge 1 })
    Set-Control 'stop' 3
    Assert-That 'stop is acknowledged' (Wait-For { (Read-SoakJson "$soak\guest.json").state -eq 'stopped' } 60)
    $score = @(Get-SoakScore -Records (Get-Records))
    Assert-That 'the score shows a rehearsal without a claim' ($score.Count -eq 1 -and $score[0].Rehearsal -and $score[0].Hours -gt 0 -and $score[0].Claim -like 'none*')
} finally {
    $env:COMPUTERNAME = $name
    if ($guest -and -not $guest.HasExited) { & taskkill.exe /PID $guest.Id /T /F | Out-Null }
    if ($job) { Stop-Job $job; Remove-Job $job -Force }
    Start-Sleep -Seconds 1
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
if ($script:failures -gt 0) { Write-Host "$($script:failures) check(s) failed." -ForegroundColor Red; exit 1 }
Write-Host 'All guest runner checks passed.' -ForegroundColor Green
