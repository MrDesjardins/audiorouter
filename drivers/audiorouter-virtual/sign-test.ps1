[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [string] $Package,
    [ValidatePattern('^[A-Fa-f0-9]{40}$')] [string] $CertificateThumbprint,
    [string] $TimestampUrl
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'package-tools.ps1')
$Package = Assert-DriverPackagePath $Package
$metadata = Read-DriverPackage $Package
$signtool = Find-DriverTool 'signtool.exe'
$inf2cat = Find-DriverTool 'Inf2Cat.exe'
$infverif = Find-DriverTool 'infverif.exe'
if ($TimestampUrl -and ($TimestampUrl -notmatch '^https?://' -or -not [Uri]::IsWellFormedUriString($TimestampUrl, [UriKind]::Absolute))) {
    throw 'TimestampUrl must be an absolute HTTP(S) URL.'
}
# Fail before certificate creation or signing if INF verification rejects it.
$null = Invoke-DriverTool $infverif @('/v', '/h', (Join-Path $Package 'audioroutervirtual.inf')) (Join-Path $Package 'infverif.log')
if ($CertificateThumbprint) {
    $certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$CertificateThumbprint"
} else {
    $certificate = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert |
        Where-Object { ($_.Subject -eq 'CN=AudioRouter Test Driver' -or $_.Subject -match '^CN="?WDKTestCert(?:[ ,"]|$)') -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(1) } |
        Sort-Object NotAfter -Descending | Select-Object -First 1
    if (-not $certificate) {
        $certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=AudioRouter Test Driver' `
            -CertStoreLocation Cert:\CurrentUser\My -KeyExportPolicy NonExportable -HashAlgorithm SHA256 `
            -NotAfter (Get-Date).AddYears(1)
    }
}
if (($certificate.Subject -ne 'CN=AudioRouter Test Driver' -and $certificate.Subject -notmatch '^CN="?WDKTestCert(?:[ ,"]|$)') -or -not $certificate.HasPrivateKey -or
    $certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -le (Get-Date) -or
    '1.3.6.1.5.5.7.3.3' -notin @($certificate.EnhancedKeyUsageList | ForEach-Object { [string]$_.ObjectId })) {
    throw 'Certificate must be a valid AudioRouter/WDK test code-signing certificate with a private key.'
}
# Invalidate the success marker before mutation; a failed rerun cannot leave
# metadata claiming that the current package is still successfully signed.
$metadata.signed = 'unsigned'
Write-DriverPackageMetadata $Package $metadata
$signArgs = @('sign', '/v', '/fd', 'sha256', '/s', 'My', '/sha1', $certificate.Thumbprint)
if ($TimestampUrl) { $signArgs += @('/tr', $TimestampUrl, '/td', 'sha256') }
$sys = Join-Path $Package 'audioroutervirtual.sys'
$cat = Join-Path $Package 'audioroutervirtual.cat'
# Embedding the SYS signature changes its bytes. Catalog creation MUST follow
# that change or its hash will describe an obsolete binary.
$null = Invoke-DriverTool $signtool ($signArgs + $sys) (Join-Path $Package 'sign-sys.log')
$null = Invoke-DriverTool $inf2cat @("/driver:$Package", '/os:10_X64', '/uselocaltime') (Join-Path $Package 'inf2cat.log')
$null = Invoke-DriverTool $signtool ($signArgs + $cat) (Join-Path $Package 'sign-cat.log')
$verification = @()
foreach ($name in @('audioroutervirtual.sys', 'audioroutervirtual.cat', 'audioroutervirtual.inf')) {
    $file = Join-Path $Package $name
    $verifyArgs = @('verify', '/pa', '/v')
    if ($name -ne 'audioroutervirtual.cat') { $verifyArgs += @('/c', $cat) }
    $result = Invoke-DriverTool $signtool ($verifyArgs + $file) (Join-Path $Package "verify-$name.log") -AllowFailure
    # Untrusted test root is expected; no root is imported on this host.
    # All other verification failures remain failures (including bad hashes).
    Assert-DriverVerificationResult $result $certificate.Thumbprint
    $verification += [ordered]@{ file = $name; exitCode = $result.ExitCode; untrustedTestRoot = ($result.ExitCode -ne 0); sha256 = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash }
}
# Verify both standalone signatures independently of catalog membership.
foreach ($file in @($sys, $cat)) {
    $embedded = Get-AuthenticodeSignature -LiteralPath $file
    if (-not $embedded.SignerCertificate -or $embedded.SignerCertificate.Thumbprint -ne $certificate.Thumbprint -or
        ($embedded.Status -ne 'Valid' -and $embedded.StatusMessage -notmatch 'certificate which is not trusted by the trust provider')) {
        throw 'Signature does not match the selected test certificate or has an unexpected verification failure.'
    }
}
Export-Certificate -Cert $certificate -FilePath (Join-Path $Package 'AudioRouterTest.cer') -Type CERT | Out-Null
[ordered]@{ certificateThumbprint = $certificate.Thumbprint; timestamped = [bool]$TimestampUrl; checks = $verification } |
    ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $Package 'signature-verification.json') -Encoding UTF8
$metadata.signed = 'test'
Write-DriverPackageMetadata $Package $metadata
Write-Host "Test-signed package: $Package (public CER only; host trust unchanged)."
