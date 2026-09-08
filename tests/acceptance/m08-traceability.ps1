param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$deliveryPath = Join-Path $repositoryRoot 'docs\spec\15-delivery.md'
$specDirectory = Join-Path $repositoryRoot 'docs\spec'

$deliveryLines = Get-Content -LiteralPath $deliveryPath
$inTraceability = $false
$traceRanges = @()
foreach ($line in $deliveryLines) {
    if ($line -eq '## Requirement traceability') { $inTraceability = $true; continue }
    if ($inTraceability -and $line -match '^## ') { break }
    if (-not $inTraceability -or $line -notmatch '^\|') { continue }
    $cell = ($line -split '\|')[1]
    foreach ($match in [regex]::Matches($cell, '(?<family>[A-Z]+)-(?<start>\d+)(?:\s*[^A-Z0-9|]+\s*(?<end>\d+))?')) {
        $start = [int]$match.Groups['start'].Value
        $end = if ($match.Groups['end'].Success) { [int]$match.Groups['end'].Value } else { $start }
        $traceRanges += [pscustomobject]@{ Family = $match.Groups['family'].Value; Start = $start; End = $end }
    }
}

$normativeIds = @()
foreach ($file in Get-ChildItem -LiteralPath $specDirectory -Filter '*.md' | Where-Object Name -ne '15-delivery.md') {
    $text = Get-Content -LiteralPath $file.FullName -Raw
    foreach ($match in [regex]::Matches($text, '\*\*(?<family>[A-Z]+)-(?<number>\d+)\s')) {
        $normativeIds += [pscustomobject]@{ Family = $match.Groups['family'].Value; Number = [int]$match.Groups['number'].Value }
    }
}
$normativeIds = $normativeIds | Sort-Object Family, Number -Unique
$missing = @($normativeIds | Where-Object {
    $id = $_
    -not ($traceRanges | Where-Object { $_.Family -eq $id.Family -and $id.Number -ge $_.Start -and $id.Number -le $_.End })
})
if ($missing.Count -gt 0) {
    throw "Requirement traceability is missing $($missing.Count) normative IDs: $($missing | ForEach-Object { \"$($_.Family)-$($_.Number.ToString('00'))\" } -join ', ')"
}

Write-Output "M08 traceability acceptance passed: $($normativeIds.Count) normative requirement IDs covered by the delivery map"
Write-Output 'Scope: documentation coverage only; this does not mark implementation, hardware, driver, signing, or release gates complete.'
