[CmdletBinding()]
param([string] $Package)
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $workspace 'drivers/audiorouter-virtual/package-tools.ps1')
$script:checks = 0
function Check {
    param([bool] $Condition, [string] $Description)
    if (-not $Condition) { throw $Description }
    $script:checks++
}
function Reject {
    param([scriptblock] $Action, [string] $Description)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    Check $rejected $Description
}
Check ((ConvertTo-DriverVersion '0.1.0') -eq '0.1.0.0') 'Valid version conversion failed.'
Check ((ConvertTo-DriverVersion '65534.65534.65534') -eq '65534.65534.65534.0') 'Upper version boundary failed.'
foreach ($bad in @('', '1.2', '1.2.3.4', '1.2.3-beta', '01.2.3', '-1.2.3', '65535.0.0', '9999999999999.0.0', '1.2.3&exit')) {
    Reject { ConvertTo-DriverVersion $bad } "Accepted invalid version: $bad"
}
$thumbprint = 'A' * 40
$rootWarning = "SHA1 hash: $thumbprint`nNumber of errors: 1`nSignTool Error: A certificate chain processed, but terminated in a root`n`tcertificate which is not trusted by the trust provider."
foreach ($ending in @('', "`nSignTool Error: File not valid: C:\test\driver.sys")) {
    Assert-DriverVerificationResult ([pscustomobject]@{ ExitCode = 1; Output = $rootWarning + $ending }) $thumbprint
    $script:checks++
}
foreach ($failure in @(
    [pscustomobject]@{ ExitCode = 2; Output = $rootWarning },
    [pscustomobject]@{ ExitCode = 1; Output = $rootWarning.Replace($thumbprint, ('B' * 40)) },
    [pscustomobject]@{ ExitCode = 1; Output = $rootWarning.Replace('Number of errors: 1', 'Number of errors: 2') },
    [pscustomobject]@{ ExitCode = 1; Output = "$rootWarning`nSignTool Error: Hash mismatch" },
    [pscustomobject]@{ ExitCode = 1; Output = 'Unknown failure' }
)) { Reject { Assert-DriverVerificationResult $failure $thumbprint } 'Accepted unexpected verification failure.' }
# Mock-free package/integrity tests against the real WDK-built package.
if ($Package) {
    $Package = Assert-DriverPackagePath $Package
    $metadata = Read-DriverPackage $Package
    Check ($metadata.signed -eq 'test') 'The real package was not verified as test-signed.'
    $certificate = New-Object Security.Cryptography.X509Certificates.X509Certificate2 (Join-Path $Package 'AudioRouterTest.cer')
    Check (-not $certificate.HasPrivateKey) 'Export included a private key.'
    $verification = Get-Content (Join-Path $Package 'signature-verification.json') -Raw | ConvertFrom-Json
    Check ($verification.certificateThumbprint -eq $certificate.Thumbprint) 'Verification report certificate mismatch.'
    foreach ($item in $verification.checks) {
        Check ((Get-FileHash (Join-Path $Package $item.file) -Algorithm SHA256).Hash -eq $item.sha256) 'Verification report contains a stale artifact hash.'
    }
    $build = Join-Path $workspace 'drivers/audiorouter-virtual/build.ps1'
    $powershellExe = Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe'
    $refused = Invoke-DriverTool $powershellExe @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $build, '-Output', $Package) '' -AllowFailure
    Check ($refused.ExitCode -ne 0 -and $refused.Output -match 'Output must be an empty directory') 'Build accepted existing package output.'
    foreach ($item in $verification.checks) {
        Check ((Get-FileHash (Join-Path $Package $item.file) -Algorithm SHA256).Hash -eq $item.sha256) 'Refused build changed the existing package.'
    }
    $signtool = Find-DriverTool 'signtool.exe'
    $cat = Join-Path $Package 'audioroutervirtual.cat'
    $originalInf = Join-Path $Package 'audioroutervirtual.inf'
    $originalSys = Join-Path $Package 'audioroutervirtual.sys'
    foreach ($file in @($originalInf, $originalSys)) {
        $result = Invoke-DriverTool $signtool @('verify', '/pa', '/v', '/c', $cat, $file) '' -AllowFailure
        Assert-DriverVerificationResult $result $certificate.Thumbprint
        $script:checks++
    }
    $testRoot = Join-Path $workspace ('target/driver-package-test-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    # Retain disposable test evidence; no recursive cleanup.
    foreach ($file in @($originalInf, $originalSys)) {
        $copy = Join-Path $testRoot ([IO.Path]::GetFileName($file))
        Copy-Item -LiteralPath $file -Destination $copy
        $bytes = [IO.File]::ReadAllBytes($copy)
        # Change SYS code, outside Authenticode's certificate/checksum exclusions.
        $offset = if ($file -eq $originalInf) { 0 } else { 4096 }
        $bytes[$offset] = $bytes[$offset] -bxor 1
        [IO.File]::WriteAllBytes($copy, $bytes)
        $result = Invoke-DriverTool $signtool @('verify', '/pa', '/v', '/c', $cat, $copy) (Join-Path $testRoot (([IO.Path]::GetFileName($file)) + '.log')) -AllowFailure
        Reject { Assert-DriverVerificationResult $result $certificate.Thumbprint } "Accepted tampered artifact: $copy"
    }
    Copy-Item -LiteralPath (Join-Path $Package 'LICENSE-MS-PL.txt') -Destination $testRoot
    Copy-Item -LiteralPath $originalInf -Destination (Join-Path $testRoot 'audioroutervirtual.inf') -Force
    $metadata.version = '0.1.99'
    Write-DriverPackageMetadata $testRoot $metadata
    Reject { Read-DriverPackage $testRoot } 'Accepted metadata/INF version mismatch.'
    Write-Host "Tamper evidence: $testRoot"
}
Write-Host "M03 driver package checks passed: $script:checks (no driver load or host trust changes)."
