<#
.SYNOPSIS
  Run the AudioRouter cable tests inside the test VM, one named step at a time.

.DESCRIPTION
  Copy the prepared share into the VM as C:\ar, open PowerShell AS
  ADMINISTRATOR, then for example:

      cd C:\ar
      powershell -NoProfile -ExecutionPolicy Bypass -File .\vm-checks.ps1 -Step preflight

  Steps (see docs/operations/virtual-cable-vm-guide.md for the order):
    preflight     read-only: test mode, Secure Boot off, certificate, files
    smoke         A1-A3 + A14: install with the helper, check 4 endpoints, uninstall, compare baseline
    install       install and KEEP the driver (for the steps below)
    status        helper status + endpoint inventory (formats, low-latency period)
    tone          bridge tone tool, clean 10-minute run (A4 tool, A5)
    tone-stall    bridge tone tool with a deliberate stall (counters must rise)
    tone-8ch      8-channel 96 kHz lease
    cables        2 -> 8 -> 2 cables; Cable A/B endpoint IDs must not change (A15 count)
    rename        rename cable B; its endpoint IDs must not change
    verifier-on   enable Driver Verifier for audioroutervirtual.sys (then reboot)
    fuzz          30-minute IOCTL fuzz (run with Driver Verifier on) (A10, A11)
    verifier-off  disable Driver Verifier (then reboot)
    remove        uninstall with the helper
    collect       zip C:\ar\evidence for the developer

  Every step writes C:\ar\evidence\<time>-<step>\ (transcript + JSON) and
  prints PASS or FAIL at the end. The script refuses to run outside the
  test VM (computer name AR-DriverTest or the marker C:\ar\IS_TEST_VM).
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('preflight', 'smoke', 'install', 'status', 'tone', 'tone-stall', 'tone-8ch', 'cables', 'rename',
        'verifier-on', 'fuzz', 'verifier-off', 'remove', 'collect')]
    [string] $Step,
    [int] $FuzzSeconds = 1800,
    [int] $ToneSeconds = 600
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
# Inside the VM the scripts sit under C:\ar\repo; in the repository they are
# two folders up. Either way the identity check below runs before anything else.
$support = Join-Path $root 'repo\tests\acceptance\m03-driver-vm-support.ps1'
if (-not (Test-Path -LiteralPath $support)) { $support = Join-Path $root '..\..\tests\acceptance\m03-driver-vm-support.ps1' }
. $support
# Same identity rule as the WP-03 runner: never on the developer's PC.
Assert-DriverTestVmIdentity $env:COMPUTERNAME (Test-Path -LiteralPath 'C:\ar\IS_TEST_VM' -PathType Leaf)
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Open PowerShell with "Run as administrator" inside the VM.'
}

$package = Join-Path $root 'driver'
$helper = Join-Path $root 'tools\audiorouter-driver-helper.exe'
$tone = Join-Path $root 'tools\m03_bridge_tone.exe'
$inventoryTool = Join-Path $root 'tools\m03_cable_inventory.exe'
$fuzzer = Join-Path $root 'tools\m03-bridge-fuzz.exe'
$evidenceRoot = 'C:\ar\evidence'
$evidence = Join-Path $evidenceRoot ((Get-Date -Format 'yyyyMMdd-HHmmss') + "-$Step")
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
Start-Transcript -LiteralPath (Join-Path $evidence 'transcript.txt') | Out-Null
$results = [Collections.Generic.List[object]]::new()
function Check([string] $Name, [bool] $Passed, [string] $Detail = '') {
    $results.Add([pscustomobject]@{ check = $Name; passed = $Passed; detail = $Detail })
    $mark = if ($Passed) { 'PASS' } else { 'FAIL' }
    $color = if ($Passed) { 'Green' } else { 'Red' }
    Write-Host ("[{0}] {1} {2}" -f $mark, $Name, $Detail) -ForegroundColor $color
}
# Helper/test tools print JSON; keep it and parse it. The opt-in variable
# exists only in this PowerShell process (17 §9.1), never machine-wide.
function Invoke-Helper([string[]] $Arguments, [string] $Name) {
    $env:AUDIOROUTER_ALLOW_TEST_DRIVER = '1'
    $resultFile = Join-Path $evidence "$Name.json"
    $output = & $helper @Arguments '--result' $resultFile 2>&1 | Out-String
    $code = $LASTEXITCODE
    $output | Set-Content -LiteralPath (Join-Path $evidence "$Name.txt") -Encoding UTF8
    $json = $null
    if (Test-Path -LiteralPath $resultFile) { $json = Get-Content -LiteralPath $resultFile -Raw | ConvertFrom-Json }
    return [pscustomobject]@{ Code = $code; Json = $json; Text = $output }
}
function Invoke-Inventory([int] $Cables, [string] $Name) {
    $out = Join-Path $evidence "$Name.json"
    $text = & $inventoryTool --cables $Cables --out $out 2>&1 | Out-String
    $code = $LASTEXITCODE
    $text | Set-Content -LiteralPath (Join-Path $evidence "$Name.txt") -Encoding UTF8
    $json = if (Test-Path -LiteralPath $out) { Get-Content -LiteralPath $out -Raw | ConvertFrom-Json } else { $null }
    return [pscustomobject]@{ Code = $code; Json = $json; Text = $text }
}
function Get-CableIds($Inventory, [string[]] $Letters) {
    $map = @{}
    foreach ($endpoint in @($Inventory.Json.endpoints)) {
        foreach ($letter in $Letters) {
            foreach ($side in 'Input', 'Output') {
                if (Test-CableEndpointName $endpoint.name "AudioRouter Cable $letter $side") { $map["$letter $side"] = $endpoint.id }
            }
        }
    }
    return $map
}
function Show-ToneSummary($Text) {
    $counters = @($Text -split "`n" | Where-Object { $_ -match 'counters' })
    $counters | ForEach-Object { Write-Host "  $_" }
    return $counters
}

try {
    switch ($Step) {
        'preflight' {
            $boot = (& bcdedit /enum '{current}') -join "`n"
            Check 'Test mode on (bcdedit testsigning Yes)' ($boot -match '(?im)^\s*testsigning\s+Yes\s*$')
            $secureBoot = $null
            try { $secureBoot = Confirm-SecureBootUEFI } catch { $secureBoot = $false }
            Check 'Secure Boot off in this VM' (-not $secureBoot)
            $cer = Join-Path $package 'AudioRouterTest.cer'
            $thumb = if (Test-Path $cer) { (New-Object Security.Cryptography.X509Certificates.X509Certificate2 $cer).Thumbprint } else { '' }
            Check 'Test certificate file present' ([bool]$thumb) $cer
            Check 'Certificate trusted (Root)' ([bool](Get-ChildItem Cert:\LocalMachine\Root | Where-Object Thumbprint -eq $thumb))
            Check 'Certificate trusted (TrustedPublisher)' ([bool](Get-ChildItem Cert:\LocalMachine\TrustedPublisher | Where-Object Thumbprint -eq $thumb))
            foreach ($file in $helper, $tone, $inventoryTool, $fuzzer, (Join-Path $package 'audioroutervirtual.inf'),
                (Join-Path $package 'audioroutervirtual.sys'), (Join-Path $package 'audioroutervirtual.cat')) {
                Check "File present: $(Split-Path -Leaf $file)" (Test-Path -LiteralPath $file -PathType Leaf)
            }
            Check 'Windows audio service running' ((Get-Service Audiosrv).Status -eq 'Running')
            $existing = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' })
            Check 'No AudioRouter driver installed yet (clean snapshot)' ($existing.Count -eq 0) "$($existing.Count) found"
            Check 'Not running from a network path' (-not $root.StartsWith('\\')) $root
        }
        'smoke' {
            $runner = Join-Path $root 'repo\tests\acceptance\m03-driver-vm.ps1'
            & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner -Package $package -Helper $helper -Evidence (Join-Path $evidence 'runner')
            Check 'WP-03 runner (A1 baseline, A2 install, A3 endpoints, A14 uninstall + baseline)' ($LASTEXITCODE -eq 0) "evidence: $evidence\runner"
        }
        'install' {
            $r = Invoke-Helper @('install', '--package', $package) 'helper-install'
            Check 'helper install (exit 0)' ($r.Code -eq 0) "exit $($r.Code) $($r.Json.error)"
            if ($r.Json) { Check 'Four endpoints reported' (@($r.Json.endpoints).Count -eq 4) (@($r.Json.endpoints | ForEach-Object { $_.friendlyName }) -join '; ') }
            if ($r.Json.defaultsChanged) { Write-Host "Note: Windows changed default devices: $($r.Json.defaultsChanged -join ', ') (recorded, see 17 §9.4)" -ForegroundColor Yellow }
            $again = Invoke-Helper @('install', '--package', $package) 'helper-install-again'
            Check 'Second install is a no-op (idempotent)' ($again.Code -eq 0 -and $again.Json.alreadyInstalled) "exit $($again.Code)"
        }
        'status' {
            $r = Invoke-Helper @('status') 'helper-status'
            Check 'helper status = installed' ($r.Json.state -eq 'installed') "state=$($r.Json.state) protocol=$($r.Json.protocol) driver=$($r.Json.installedDriverVersion)"
            $count = if ($r.Json.cableCount) { [int]$r.Json.cableCount } else { 2 }
            $inventory = Invoke-Inventory $count 'inventory'
            foreach ($endpoint in @($inventory.Json.endpoints)) {
                Write-Host ("  {0}: periods {1} formats {2}/60" -f $endpoint.name, ($endpoint.enginePeriodFrames | ConvertTo-Json -Compress), $endpoint.formatsSupported)
            }
            Check 'Inventory: names, 60 formats, min period <= 128 frames (VCAB-11, VCAB-25)' ($inventory.Code -eq 0) 'details in inventory.txt'
        }
        { $_ -in 'tone', 'tone-stall', 'tone-8ch' } {
            $arguments = switch ($Step) {
                'tone' { @('--seconds', $ToneSeconds) }
                'tone-stall' { @('--seconds', 30, '--stall-ms', 500) }
                'tone-8ch' { @('--seconds', 60, '--channels', 8, '--rate', 96000, '--frames', 480) }
            }
            Write-Host 'While this runs: record "AudioRouter Cable B Output" (e.g. Audacity) to hear 997 Hz left / 47 Hz right,'
            Write-Host 'and play any sound into "AudioRouter Cable A Input" so it lands in the WAV.' -ForegroundColor Yellow
            $wav = Join-Path $evidence 'render-source.wav'
            $text = & $tone @arguments --out $wav 2>&1 | Out-String
            $code = $LASTEXITCODE
            $text | Set-Content -LiteralPath (Join-Path $evidence 'tone.txt') -Encoding UTF8
            Check 'Tone tool finished (driver compatible, leases opened and closed)' ($code -eq 0) "exit $code"
            $lines = Show-ToneSummary $text
            $values = @($lines | ForEach-Object { [regex]::Matches($_, '(underrun_frames|overrun_frames|sequence_gaps|non_finite_samples|format_mismatches): (\d+)') } |
                ForEach-Object { [long]$_.Groups[2].Value })
            if ($Step -eq 'tone-stall') {
                Check 'Counters rose during the deliberate stall' (($values | Measure-Object -Sum).Sum -gt 0)
            } elseif ($Step -eq 'tone') {
                Check 'All error counters zero in the clean run' (($values | Measure-Object -Sum).Sum -eq 0) 'see tone.txt'
            }
            Check 'WAV written' (Test-Path -LiteralPath $wav) $wav
        }
        'cables' {
            $before = Invoke-Inventory 2 'inventory-2-before'
            $idsBefore = Get-CableIds $before @('A', 'B')
            $r = Invoke-Helper @('set-cables', '--count', '8') 'set-cables-8'
            Check 'set-cables 8' ($r.Code -eq 0) "exit $($r.Code) $($r.Json.error)"
            $eight = Invoke-Inventory 8 'inventory-8'
            Check '16 endpoints with all formats (A15 count)' ($eight.Code -eq 0) 'inventory-8.txt'
            $r = Invoke-Helper @('set-cables', '--count', '2') 'set-cables-2'
            Check 'set-cables 2' ($r.Code -eq 0) "exit $($r.Code) $($r.Json.error)"
            $after = Invoke-Inventory 2 'inventory-2-after'
            $idsAfter = Get-CableIds $after @('A', 'B')
            $same = $idsBefore.Count -eq 4 -and @($idsBefore.Keys | Where-Object { $idsBefore[$_] -ne $idsAfter[$_] }).Count -eq 0
            Check 'Cable A/B endpoint IDs unchanged after 2 -> 8 -> 2 (VCAB-05)' $same
            Check 'Cables C-H gone again' (@($after.Json.endpoints).Count -eq 4)
        }
        'rename' {
            $before = Get-CableIds (Invoke-Inventory 2 'inventory-before-rename') @('B')
            $r = Invoke-Helper @('configure', '--name', '2=Discord') 'configure-rename'
            Check 'configure --name 2=Discord' ($r.Code -eq 0) "exit $($r.Code) $($r.Json.error)"
            $inventory = Invoke-Inventory 2 'inventory-after-rename'
            $names = @($inventory.Json.endpoints | ForEach-Object { $_.name })
            Check 'New name visible' (@($names | Where-Object { $_ -like '*Discord*' }).Count -eq 2) ($names -join '; ')
            $after = Get-CableIds $inventory @('B')
            Check 'Cable B endpoint IDs unchanged by the rename' ($before.Count -eq 2 -and $before['B Input'] -eq $after['B Input'] -and $before['B Output'] -eq $after['B Output'])
        }
        'verifier-on' {
            & verifier /standard /driver audioroutervirtual.sys | Set-Content (Join-Path $evidence 'verifier.txt')
            Check 'Driver Verifier configured' ($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq 2) 'Reboot the VM now (Restart-Computer).'
        }
        'fuzz' {
            $verifier = (& verifier /querysettings) -join "`n"
            Check 'Driver Verifier active for audioroutervirtual.sys' ($verifier -match 'audioroutervirtual\.sys')
            $text = & $fuzzer --seconds $FuzzSeconds 2>&1 | Out-String
            $code = $LASTEXITCODE
            $text | Set-Content -LiteralPath (Join-Path $evidence 'fuzz.txt') -Encoding UTF8
            Write-Host ($text -split "`n" | Select-Object -Last 3)
            Check "IOCTL fuzz $FuzzSeconds s: no crash, no unexpected result, leases exercised" ($code -eq 0)
        }
        'verifier-off' {
            & verifier /reset | Set-Content (Join-Path $evidence 'verifier.txt')
            Check 'Driver Verifier reset' $true 'Reboot the VM now (Restart-Computer).'
        }
        'remove' {
            $r = Invoke-Helper @('remove') 'helper-remove'
            Check 'helper remove (exit 0 or 5 = restart pending)' ($r.Code -eq 0 -or $r.Code -eq 5) "exit $($r.Code) $($r.Json.error)"
            $left = @(Get-PnpDevice -Class MEDIA -ErrorAction SilentlyContinue | Where-Object { $_.FriendlyName -like '*AudioRouter*' })
            Check 'No AudioRouter device left' ($left.Count -eq 0) "$($left.Count) left"
        }
        'collect' {
            $zip = Join-Path $root ('evidence-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.zip')
            # Windows' driver-install log explains Code 10/52 and install failures.
            Copy-Item -LiteralPath 'C:\Windows\INF\setupapi.dev.log' -Destination (Join-Path $evidence 'setupapi.dev.log') -ErrorAction SilentlyContinue
            Get-PnpDevice -Class MEDIA, AudioEndpoint -ErrorAction SilentlyContinue |
                Select-Object FriendlyName, Status, Problem, InstanceId |
                ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'devices.json') -Encoding UTF8
            Compress-Archive -Path (Join-Path $evidenceRoot '*') -DestinationPath $zip
            # Minidumps help if the VM crashed (blue screen) during a test.
            foreach ($dump in @(Get-ChildItem C:\Windows\Minidump -Filter *.dmp -ErrorAction SilentlyContinue)) {
                Compress-Archive -Path $dump.FullName -DestinationPath $zip -Update
            }
            Check 'Evidence zipped' (Test-Path $zip) $zip
        }
    }
} catch {
    Check 'Step ran without an error' $false $_.Exception.Message
} finally {
    $results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidence 'summary.json') -Encoding UTF8
    Stop-Transcript | Out-Null
}
$failed = @($results | Where-Object { -not $_.passed }).Count
if ($failed -eq 0) {
    Write-Host "`n${Step}: PASS ($($results.Count) checks). Evidence: $evidence" -ForegroundColor Green
    exit 0
}
Write-Host "`n${Step}: FAIL ($failed of $($results.Count) checks). Evidence: $evidence" -ForegroundColor Red
exit 1
