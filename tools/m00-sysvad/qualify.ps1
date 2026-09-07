param(
    [switch]$KeepCheckout
)

$ErrorActionPreference = 'Stop'
$samplesRepository = 'https://github.com/microsoft/Windows-driver-samples.git'
$samplesCommit = '197ba2156a60e2b76fcd4820bae594223e91a1e9'
$checkout = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-sysvad-' + [guid]::NewGuid().ToString('N'))
$wrapper = Join-Path $PSScriptRoot '..\..\tests\acceptance\m00-sysvad-build.ps1'

try {
    Write-Output "Creating disposable SysVAD checkout at $checkout"
    & git clone --depth 1 $samplesRepository $checkout
    if ($LASTEXITCODE -ne 0) { throw "Windows driver samples clone failed with exit code $LASTEXITCODE" }
    $actualCommit = (& git -C $checkout rev-parse HEAD).Trim()
    if ($actualCommit -ne $samplesCommit) {
        throw "Windows driver samples revision mismatch: expected $samplesCommit, found $actualCommit"
    }
    Write-Output "Using Windows driver samples revision $actualCommit"

    & git -C $checkout submodule update --init --depth 1 wil
    if ($LASTEXITCODE -ne 0) { throw "WIL submodule checkout failed with exit code $LASTEXITCODE" }

    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $wrapper -SourceRoot $checkout
    if ($LASTEXITCODE -ne 0) { throw "SysVAD qualification failed with exit code $LASTEXITCODE" }
    Write-Output 'Scope: disposable reference checkout only; no driver installation, loading, signing-mode change, or machine audio configuration action.'
}
finally {
    if (-not $KeepCheckout -and (Test-Path -LiteralPath $checkout)) {
        Remove-Item -LiteralPath $checkout -Recurse -Force -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $checkout) {
            Write-Warning "Disposable checkout could not be fully removed: $checkout"
        } else {
            Write-Output 'Disposable SysVAD checkout removed.'
        }
    } elseif ($KeepCheckout) {
        Write-Output "Kept disposable checkout for inspection: $checkout"
    }
}
