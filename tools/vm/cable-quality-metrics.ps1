function Read-CableQualityArchiveMetric {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string] $ArchivePath)

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        $entry = $archive.Entries | Where-Object { $_.FullName -match '(^|/)metrics\.json$' } | Select-Object -First 1
        if (-not $entry) { throw "Archive has no metrics.json: $ArchivePath" }
        $stream = $entry.Open()
        try {
            $reader = [IO.StreamReader]::new($stream)
            try { $metrics = ConvertFrom-Json -InputObject $reader.ReadToEnd() }
            finally { $reader.Dispose() }
        }
        finally { $stream.Dispose() }
    }
    finally { $archive.Dispose() }

    foreach ($candidate in @(
        @{ Name = 'Worst response deviation'; Key = 'worstDeviationDb'; Units = 'dB' },
        @{ Name = 'Worst THD+N'; Key = 'worstThdnDb'; Units = 'dB' },
        @{ Name = 'Maximum PCM error'; Key = 'maximumErrorLsb'; Units = 'LSB' },
        @{ Name = 'Maximum sample error'; Key = 'maxAbsoluteError'; Units = 'FS' },
        @{ Name = 'Peak leakage'; Key = 'peakDbfs'; Units = 'dBFS' }
    )) {
        $property = $metrics.PSObject.Properties[$candidate.Key]
        if ($null -ne $property -and $null -ne $property.Value) {
            return [pscustomobject]@{
                Name = $candidate.Name
                Value = [double]$property.Value
                Units = $candidate.Units
                AnalyzerPass = $metrics.pass
            }
        }
    }
    throw "No supported numeric measurement in archive: $ArchivePath"
}
