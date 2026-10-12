<# Run the currently implemented WP-09b case in the disposable driver VM.
   This is a diagnostic subset of A16, never a complete A16 qualification. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('float32-48k')][string] $Case,
    [ValidateRange(1, 8)][int] $Channels = 2,
    [ValidateRange(5, 60)][int] $Seconds = 20
)
$ErrorActionPreference = 'Stop'
$bundle = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
if ($env:COMPUTERNAME -ine 'AR-DriverTest' -or $bundle -notlike 'C:\ar\*') {
    throw 'Run inside AR-DriverTest from the copied bundle under C:\ar.'
}
. (Join-Path $bundle 'paired-trace-support.ps1')
Assert-PairedTraceBundle $bundle
$runner = Join-Path $bundle 'run-cable-bitexact.ps1'
if (-not (Test-Path -LiteralPath $runner -PathType Leaf)) { throw 'The bundle has no bit-exact product-route runner.' }
Write-Host "WP-09b diagnostic subset: $Case, $Channels channel(s), 48 kHz float32 only. This does not pass A16."
& (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') `
    -NoProfile -ExecutionPolicy Bypass -File $runner -Channels $Channels -Seconds $Seconds
if ($LASTEXITCODE -ne 0) { throw "Float32 bit-exact diagnostic failed (exit $LASTEXITCODE)." }
Write-Host 'Diagnostic case completed. A16 remains incomplete until every required rate, format, channel count, resampling and crosstalk case is implemented and run.'
