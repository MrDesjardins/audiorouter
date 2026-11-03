# Shared by release preparation and its fixture checks. Keep this file ASCII.
function Get-CargoPackageNotices {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][object[]]$MetadataObjects)

    $packages = @{}
    foreach ($metadata in $MetadataObjects) {
        if ($null -eq $metadata.packages) { throw "Cargo metadata has no packages array" }
        foreach ($package in $metadata.packages) {
            if ([string]::IsNullOrWhiteSpace($package.name) -or [string]::IsNullOrWhiteSpace($package.version)) {
                throw "Cargo metadata package has no name or version"
            }
            $source = if ([string]::IsNullOrWhiteSpace($package.source)) { "workspace" } else { $package.source }
            $license = if ([string]::IsNullOrWhiteSpace($package.license)) { "license metadata unavailable" } else { $package.license }
            $key = "$($package.name)`n$($package.version)`n$source"
            if ($packages.ContainsKey($key) -and $packages[$key].license -ne $license) {
                throw "Conflicting license metadata for $($package.name) $($package.version)"
            }
            $packages[$key] = [PSCustomObject]@{
                name = $package.name
                version = $package.version
                source = $source
                license = $license
            }
        }
    }
    foreach ($package in @($packages.Values | Sort-Object name, version, source)) {
        "- $($package.name) $($package.version) - $($package.license) - $($package.source)"
    }
}
