[CmdletBinding()]
param(
    [string] $InputInf,
    [string] $CableList,
    [string] $OutputInf,
    [switch] $Check,
    [switch] $Update
)
$ErrorActionPreference = 'Stop'
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $InputInf) { $InputInf = Join-Path $scriptRoot '../../drivers/audiorouter-virtual/Source/Main/AudioRouterVirtual.inx' }
if (-not $CableList) { $CableList = Join-Path $scriptRoot 'cables.json' }
$topologyHeader = Join-Path $scriptRoot '../../drivers/audiorouter-virtual/Source/Filters/cabletopotable.h'

$cableData = Get-Content -LiteralPath $CableList -Raw | ConvertFrom-Json
if ($cableData.schemaVersion -ne 1 -or $cableData.cables.Count -ne 8) {
    throw 'Cable list must contain exactly eight entries and schemaVersion 1.'
}
for ($index = 0; $index -lt 8; $index++) {
    if ($cableData.cables[$index].index -ne $index -or
        $cableData.cables[$index].letter -cne [string][char](65 + $index)) {
        throw "Cable list must be contiguous A-H at index $index."
    }
}
$uniquePinNames = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($cable in $cableData.cables) {
    foreach ($property in @('renderPinNameGuid', 'capturePinCategoryGuid')) {
        $guid = [guid]::Empty
        if (-not [guid]::TryParse([string] $cable.$property, [ref] $guid) -or
            -not $uniquePinNames.Add($guid.ToString('D'))) {
            throw "Cable $($cable.letter) has an invalid or duplicate $property."
        }
    }
}

$inf = Get-Content -LiteralPath $InputInf -Raw
$header = Get-Content -LiteralPath $topologyHeader -Raw
$interfaceLines = [System.Collections.Generic.List[string]]::new()
$registrySections = [System.Collections.Generic.List[string]]::new()
$pinCategoryRegistry = [System.Collections.Generic.List[string]]::new()
$pinCategoryHeader = [System.Collections.Generic.List[string]]::new()
$stringLines = [System.Collections.Generic.List[string]]::new()
foreach ($cable in $cableData.cables) {
    $letter = $cable.letter
    foreach ($role in @('Render', 'Capture')) {
        $property = if ($role -eq 'Render') { 'renderPinNameGuid' } else { 'capturePinCategoryGuid' }
        $kind = if ($role -eq 'Render') { 'PinName' } else { 'PinCategory' }
        $registrationName = "Cable${letter}${role}${kind}"
        $guidText = [string] $cable.$property
        $guid = [guid]::Empty
        if (-not [guid]::TryParse($guidText, [ref] $guid)) {
            throw "Cable $letter has an invalid ${property}: $guidText"
        }
        $guidText = $guid.ToString('B').ToUpperInvariant()
        $pinCategoryRegistry.Add("HKR,MediaCategories\%GUID.$registrationName%,Name,,%Name.$registrationName%")
        $pinCategoryHeader.Add("static const GUID ${registrationName} = $($guid.ToString('X'));")
        $stringLines.Add("GUID.$registrationName=`"$guidText`"")
        $suffix = if ($role -eq 'Render') { 'Input' } else { 'Output' }
        $stringLines.Add("Name.$registrationName=`"AudioRouter Cable $letter $suffix`"")
    }
    foreach ($role in @('Render', 'Capture')) {
        $suffix = if ($role -eq 'Render') { 'Input' } else { 'Output' }
        $wave = "WaveCable${letter}${role}"
        $topology = "TopologyCable${letter}${role}"
        $waveSection = "AUDIOROUTERVIRTUAL.I.$wave"
        $topologySection = "AUDIOROUTERVIRTUAL.I.$topology"
        $friendly = "AudioRouter Cable $letter $suffix"
        $waveString = "AUDIOROUTERVIRTUAL.$wave.szPname"
        $topologyString = "AUDIOROUTERVIRTUAL.$topology.szPname"
        $interfaceLines.Add("AddInterface=%KSCATEGORY_AUDIO%, %KSNAME_$wave%, $waveSection")
        $interfaceLines.Add("AddInterface=%KSCATEGORY_REALTIME%, %KSNAME_$wave%, $waveSection")
        $category = if ($role -eq 'Render') { 'KSCATEGORY_RENDER' } else { 'KSCATEGORY_CAPTURE' }
        $interfaceLines.Add("AddInterface=%$category%, %KSNAME_$wave%, $waveSection")
        $interfaceLines.Add("AddInterface=%KSCATEGORY_AUDIO%, %KSNAME_$topology%, $topologySection")
        $interfaceLines.Add("AddInterface=%KSCATEGORY_TOPOLOGY%, %KSNAME_$topology%, $topologySection")
        foreach ($entry in @(@($wave, $waveSection, $waveString), @($topology, $topologySection, $topologyString))) {
            $nameString = $entry[2]
            $section = $entry[1]
            $name = $entry[0]
            $registrySections.Add("[$section]")
            $registrySections.Add("AddReg=$section.AddReg")
            $registrySections.Add("[$section.AddReg]")
            $registrySections.Add('HKR,,CLSID,,%Proxy.CLSID%')
            $registrySections.Add("HKR,,FriendlyName,,%$nameString%")
            if ($role -eq 'Render') {
                # Analog connectors preserve pin Names but are hidden by default.
                # Associate this override with the bridge category, enable only
                # the render flow (0x100), and keep capture policy unchanged.
                $registrySections.Add('HKR,EP\0,%PKEY_AudioEndpoint_Association%,,%KSNODETYPE_ANALOG_CONNECTOR%')
                $registrySections.Add('HKR,EP\0,%PKEY_AudioDevice_EnableEndpointByDefault%,0x00010001,0x00000101')
            } else {
                $registrySections.Add('HKR,EP\0,%PKEY_AudioEndpoint_Association%,,%KSNODETYPE_ANY%')
            }
            $registrySections.Add('HKR,EP\0,%PKEY_AudioEndpoint_Supports_EventDriven_Mode%,0x00010001,0x1')
            $stringLines.Add("KSNAME_$name=`"$name`"")
            $stringLines.Add("$nameString=`"$friendly`"")
        }
    }
}

function Replace-MarkedRegion([string] $Source, [string] $Start, [string] $End, [string[]] $Body) {
    $startAt = $Source.IndexOf($Start, [StringComparison]::Ordinal)
    $endAt = $Source.IndexOf($End, [StringComparison]::Ordinal)
    if ($startAt -lt 0 -or $endAt -le $startAt) { throw "Missing or reversed generated region markers: $Start / $End" }
    $bodyStart = $startAt + $Start.Length
    $replacement = "`r`n" + ($Body -join "`r`n") + "`r`n"
    return $Source.Substring(0, $bodyStart) + $replacement + $Source.Substring($endAt)
}

$inf = Replace-MarkedRegion $inf '; BEGIN GENERATED CABLE REGISTRY SECTIONS' '; END GENERATED CABLE REGISTRY SECTIONS' $registrySections.ToArray()
$inf = Replace-MarkedRegion $inf '; BEGIN GENERATED PIN NAME/CATEGORY REGISTRATION' '; END GENERATED PIN NAME/CATEGORY REGISTRATION' $pinCategoryRegistry.ToArray()
$inf = Replace-MarkedRegion $inf '; BEGIN GENERATED CABLE INTERFACES' '; END GENERATED CABLE INTERFACES' $interfaceLines.ToArray()
$inf = Replace-MarkedRegion $inf '; BEGIN GENERATED CABLE STRINGS' '; END GENERATED CABLE STRINGS' $stringLines.ToArray()
$header = Replace-MarkedRegion $header '// BEGIN GENERATED CABLE PIN NAME/CATEGORY GUIDS' '// END GENERATED CABLE PIN NAME/CATEGORY GUIDS' $pinCategoryHeader.ToArray()

if ($Check) {
    $actual = [IO.File]::ReadAllText((Resolve-Path $InputInf).Path)
    if ($actual -cne $inf) { throw 'INF is stale. Regenerate a preview, inspect it, then update the source.' }
    $actualHeader = [IO.File]::ReadAllText((Resolve-Path $topologyHeader).Path)
    if ($actualHeader -cne $header) { throw 'Topology pin category GUIDs are stale. Regenerate and update the source.' }
    Write-Output 'Cable INF sections and topology pin name/category GUIDs are current and deterministic.'
} elseif ($Update) {
    [IO.File]::WriteAllText((Resolve-Path $InputInf).Path, $inf, [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText((Resolve-Path $topologyHeader).Path, $header, [Text.UTF8Encoding]::new($false))
    Write-Output 'Updated cable INF sections and topology pin name/category GUIDs.'
} elseif ($OutputInf) {
    $directory = Split-Path -Parent $OutputInf
    if ($directory) { New-Item -ItemType Directory -Path $directory -Force | Out-Null }
    [IO.File]::WriteAllText($OutputInf, $inf, [Text.UTF8Encoding]::new($false))
    Write-Output "Generated INF preview: $OutputInf"
} else {
    throw 'Specify -OutputInf for a preview or -Check to verify the checked-in INF.'
}
