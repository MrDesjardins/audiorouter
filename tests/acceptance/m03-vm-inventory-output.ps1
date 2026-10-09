[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$source = Get-Content -LiteralPath (Join-Path $workspace 'tools/vm/vm-checks.ps1') -Raw
# Evaluate only the tool wrapper, never the VM script's driver operations.
$start = $source.IndexOf('function Invoke-Inventory(')
$end = $source.IndexOf('function Get-CableIds(', $start)
if ($start -lt 0 -or $end -le $start) { throw 'Inventory function boundary missing.' }
. ([scriptblock]::Create($source.Substring($start, $end - $start)))
$evidence = Join-Path $workspace ('target/inventory-output-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $evidence | Out-Null
$inventoryTool = Join-Path $evidence 'fixture.cmd'
@'
@echo off
echo fixture JSON report
echo fixture summary 1>&2
exit /b %2%
'@ | Set-Content -LiteralPath $inventoryTool -Encoding ASCII
foreach ($code in @(0, 1)) {
    $result = Invoke-Inventory $code "case-$code"
    if ($result.Code -ne $code -or
        -not $result.Text.Contains('fixture JSON report') -or
        -not $result.Text.Contains('fixture summary') -or
        -not (Test-Path -LiteralPath (Join-Path $evidence "case-$code.txt"))) {
        throw "Native inventory output/exit preservation failed for $code."
    }
    if ($ErrorActionPreference -ne 'Stop') { throw 'Error preference was not restored.' }
}
Write-Host "VM inventory output regression passed (native exit 0/1, stdout/stderr retained). Evidence: $evidence"
