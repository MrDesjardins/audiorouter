[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$generator = Join-Path $workspace 'tools/m03-inf-gen/generate.ps1'
$cableList = Join-Path $workspace 'tools/m03-inf-gen/cables.json'
$output = Join-Path $workspace 'target/m03-inf-gen-tests'
New-Item -ItemType Directory -Path $output -Force | Out-Null
$first = Join-Path $output 'first.inx'
$second = Join-Path $output 'second.inx'
& $generator -OutputInf $first | Out-Null
& $generator -OutputInf $second | Out-Null
if ((Get-FileHash -LiteralPath $first -Algorithm SHA256).Hash -ne
    (Get-FileHash -LiteralPath $second -Algorithm SHA256).Hash) {
    throw 'Cable INF generation is not deterministic.'
}
$generated = Get-Content -LiteralPath $first -Raw
if (([regex]::Matches($generated, '(?m)^AddInterface=')).Count -ne 80) {
    throw 'Generated INF must contain 80 cable interfaces.'
}
foreach ($expected in @(
        'WaveCableARender', 'TopologyCableACapture',
        'WaveCableHRender', 'TopologyCableHCapture',
        'AudioRouter Cable A Input', 'AudioRouter Cable A Output',
        'AudioRouter Cable H Input', 'AudioRouter Cable H Output')) {
    if (-not $generated.Contains($expected)) { throw "Generated INF omitted $expected." }
}
$invalid = Join-Path $output 'invalid-cables.json'
$bad = Get-Content -LiteralPath $cableList -Raw | ConvertFrom-Json
$bad.cables[2].letter = 'Z'
$bad | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $invalid -Encoding UTF8
$rejected = $false
try { & $generator -CableList $invalid -OutputInf (Join-Path $output 'bad.inx') | Out-Null }
catch { $rejected = $true }
if (-not $rejected) { throw 'Non-contiguous cable identities must be rejected.' }
& $generator -Check | Out-Null
Write-Output 'M03 cable INF generator acceptance passed (80 interfaces, deterministic output, invalid list rejected).'
