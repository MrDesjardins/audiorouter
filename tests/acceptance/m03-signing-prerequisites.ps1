[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$kitsRoot = 'C:\Program Files (x86)\Windows Kits\10'
$kit = Get-ChildItem -LiteralPath (Join-Path $kitsRoot 'bin') -Directory |
    Where-Object { $_.Name -match '^10\.0\.28000\.0$' } |
    Select-Object -First 1
if (-not $kit) {
    throw 'The installed 28000 Windows SDK signing-tool directory was not found.'
}
$signtool = Join-Path $kit.FullName 'x64\signtool.exe'
if (-not (Test-Path -LiteralPath $signtool -PathType Leaf)) {
    throw "WDK signtool.exe is missing: $signtool"
}

$secureBoot = try {
    [bool](Confirm-SecureBootUEFI -ErrorAction Stop)
} catch {
    'unavailable'
}
$guard = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard -ErrorAction SilentlyContinue
if (-not $guard) {
    throw 'Win32_DeviceGuard is unavailable; cannot record the VBS prerequisite.'
}
$vbsStatus = [int]$guard.VirtualizationBasedSecurityStatus

foreach ($platform in @('x64', 'ARM64')) {
    $package = Join-Path $workspace "drivers\audiorouter-virtual\$platform\Release\package"
    $driver = Join-Path $package 'AudioRouterVirtual.sys'
    $catalog = Get-ChildItem -LiteralPath $package -File -ErrorAction Stop |
        Where-Object { $_.Name -ieq 'audioroutervirtual.cat' } | Select-Object -First 1
    if (-not (Test-Path -LiteralPath $driver -PathType Leaf) -or -not $catalog) {
        throw "The $platform driver package is missing the prototype binary or catalog."
    }
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $verification = @(& $signtool verify /kp /c $catalog.FullName $driver 2>&1)
        $verificationExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
    $verificationText = $verification -join "`n"
    if ($verificationExitCode -eq 0 -or $verificationText -notmatch 'No signature found') {
        throw "The $platform prototype signature state was not the expected unsigned result: $verificationText"
    }
    Write-Output "${platform}: prototype .sys is unsigned; signtool verification rejected it as expected."
}

Write-Output "M03 signing prerequisites recorded: SecureBoot=$secureBoot VBSStatus=$vbsStatus signtool=$signtool"
Write-Output 'Scope: read-only prerequisite and prototype-signature verification; no signing, installation, boot-policy, or audio-configuration action.'
