# Shared build/signing helpers. Dot-sourcing this file has no side effects.
function ConvertTo-DriverVersion {
    param([string] $Version)
    if ($Version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        throw 'Version must be X.Y.Z (three decimal components, no prerelease).'
    }
    foreach ($part in $Version.Split('.')) {
        if ($part.Length -gt 5 -or [int]$part -gt 65534) {
            throw 'Driver version components must be between 0 and 65534.'
        }
    }
    return "$Version.0"
}

function Assert-DriverPackagePath {
    param([string] $Path)
    $fullPath = [IO.Path]::GetFullPath($Path)
    $cursor = $fullPath
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw 'Driver package paths must not traverse a reparse point.'
            }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }
        $cursor = $parent
    }
    return $fullPath
}

function Find-DriverTool {
    param([string] $Name)
    $kitRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
    $versions = @(Get-ChildItem -LiteralPath (Join-Path $kitRoot 'bin') -Directory |
        Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending)
    foreach ($version in $versions) {
        foreach ($relative in @("bin\$($version.Name)\x64", "bin\$($version.Name)\x86", "Tools\$($version.Name)\x64")) {
            $candidate = Join-Path (Join-Path $kitRoot $relative) $Name
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
        }
    }
    throw "WDK tool $Name was not found. Install the matching SDK/WDK."
}

function Invoke-DriverTool {
    param([string] $Tool, [string[]] $Arguments, [string] $Log, [switch] $AllowFailure)
    # PS 5.1 converts redirected native stderr into terminating ErrorRecords
    # under Stop. Capture it, restore the preference, then decide by exit code.
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $lines = @(& $Tool @Arguments 2>&1 | ForEach-Object { "$_" })
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    if ($Log) { $lines | Set-Content -LiteralPath $Log -Encoding UTF8 }
    if ($code -ne 0 -and -not $AllowFailure) {
        throw "$([IO.Path]::GetFileName($Tool)) failed ($code). See $Log`n$($lines -join "`n")"
    }
    return [pscustomobject]@{ ExitCode = $code; Output = ($lines -join "`n") }
}

function Read-DriverPackage {
    param([string] $Package)
    $root = Assert-DriverPackagePath $Package
    foreach ($name in @('audioroutervirtual.inf', 'audioroutervirtual.sys', 'LICENSE-MS-PL.txt', 'package.json')) {
        $file = Join-Path $root $name
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Package is missing $name" }
        $null = Assert-DriverPackagePath $file
    }
    $metadataPath = Join-Path $root 'package.json'
    if ((Get-Item -LiteralPath $metadataPath).Length -gt 16384) { throw 'Package metadata exceeds 16 KiB.' }
    $metadata = Get-Content -LiteralPath $metadataPath -Raw | ConvertFrom-Json
    $driverVersion = ConvertTo-DriverVersion $metadata.version
    if ($metadata.platform -ne 'x64') { throw 'Test signing currently supports only x64 packages.' }
    if ($metadata.signed -notin @('unsigned', 'test')) { throw 'Refusing to test-sign a release or unknown package.' }
    $inf = Get-Content -LiteralPath (Join-Path $root 'audioroutervirtual.inf') -Raw
    if ($inf -notmatch '(?im)^Provider\s*=\s*%ProviderName%\s*$' -or
        $inf -notmatch '(?im)^ProviderName\s*=\s*"AudioRouter Project"\s*$' -or
        $inf -notmatch '(?im)^CatalogFile\s*=\s*AudioRouterVirtual\.cat\s*$' -or
        $inf -notmatch ('(?im)^DriverVer\s*=\s*\d{2}/\d{2}/\d{4},\s*' + [regex]::Escape($driverVersion) + '\s*$')) {
        throw 'INF identity/version does not match the AudioRouter package metadata.'
    }
    foreach ($name in @('audioroutervirtual.cat', 'AudioRouterTest.cer', 'signature-verification.json')) {
        $null = Assert-DriverPackagePath (Join-Path $root $name)
    }
    return $metadata
}

function Write-DriverPackageMetadata {
    param([string] $Package, $Metadata)
    $Metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $Package 'package.json') -Encoding UTF8
}

function Assert-DriverVerificationResult {
    param($Result, [string] $Thumbprint)
    if ($Result.ExitCode -eq 0) { return }
    # SignTool omits HRESULTs on this WDK. Accept only its single, specific
    # untrusted-root diagnostic, with the expected certificate in the chain.
    # Unknown/localized diagnostics fail closed rather than hiding a bad hash.
    $errors = @([regex]::Matches($Result.Output, '(?m)^SignTool Error:.*$') | ForEach-Object { $_.Value.Trim() })
    if ($Result.ExitCode -ne 1 -or $Result.Output -notmatch [regex]::Escape($Thumbprint) -or
        $Result.Output -notmatch 'certificate which is not trusted by the trust provider' -or
        $Result.Output -notmatch 'Number of errors:\s*1\b' -or $errors.Count -notin @(1, 2) -or
        $errors[0] -ne 'SignTool Error: A certificate chain processed, but terminated in a root' -or
        ($errors.Count -eq 2 -and $errors[1] -notmatch '^SignTool Error: File not valid: ')) {
        throw 'Signature/catalog verification failed; only an untrusted test root is expected.'
    }
}
