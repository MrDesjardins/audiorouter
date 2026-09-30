param(
    [string]$TargetDirectory = 'target/deterministic-audio',
    [string]$ArtifactDirectory = 'target/deterministic-audio-evidence',
    [switch]$NativeFixtures,
    [string]$Vst2Fixture = '',
    [string]$Vst3Fixture = '',
    [string]$Vst3Worker = ''
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$previousLocation = Get-Location
$names = @('AUDIOROUTER_SIGNAL_ARTIFACTS', 'AUDIOROUTER_VST2_FIXTURE', 'AUDIOROUTER_VST3_FIXTURE', 'AUDIOROUTER_VST3_NATIVE_WORKER')
$previousEnvironment = @{}
foreach ($name in $names) { $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
$results = [System.Collections.Generic.List[object]]::new()

function Invoke-SignalCheck {
    param([string]$Name, [string[]]$CargoArguments)
    $log = Join-Path $artifactRoot "$Name.log"
    Write-Host "Running $Name (log: $log)"
    # Windows PowerShell wraps cargo's ordinary stderr progress as errors.
    # Judge native commands by their exit code, not the output stream.
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & cargo @CargoArguments > $log 2>&1
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    $results.Add([ordered]@{ name = $Name; exitCode = $code; log = $log; arguments = $CargoArguments })
    Write-Host "$Name exit code: $code"
}

try {
    Set-Location -LiteralPath $repositoryRoot
    Get-Command cargo -ErrorAction Stop | Out-Null
    $artifactRoot = [IO.Path]::GetFullPath($ArtifactDirectory)
    New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
    $env:AUDIOROUTER_SIGNAL_ARTIFACTS = $artifactRoot
    Invoke-SignalCheck 'built-in-signals' @('test', '-p', 'audiorouter-engine', '--test', 'deterministic_audio', '--release', '--locked', '--target-dir', $TargetDirectory, '--', '--nocapture')
    Invoke-SignalCheck 'dsp-engine-regressions' @('test', '-p', 'audiorouter-dsp', '-p', 'audiorouter-engine', '--lib', '--release', '--locked', '--target-dir', $TargetDirectory)
    # Explicitly recheck the former failing gate as well as running it in the
    # ordinary suite; it is no longer ignored.
    Invoke-SignalCheck 'dehum-preservation-gate' @('test', '-p', 'audiorouter-engine', '--test', 'deterministic_audio', '--release', '--locked', '--target-dir', $TargetDirectory, 'dehum_eight_harmonics_preserves_wanted_band_within_five_percent', '--', '--nocapture')
    if ($NativeFixtures) {
        foreach ($path in @($Vst2Fixture, $Vst3Fixture, $Vst3Worker)) {
            if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Native fixture file unavailable: $path" }
        }
        $env:AUDIOROUTER_VST2_FIXTURE = [IO.Path]::GetFullPath($Vst2Fixture)
        $env:AUDIOROUTER_VST3_FIXTURE = [IO.Path]::GetFullPath($Vst3Fixture)
        $env:AUDIOROUTER_VST3_NATIVE_WORKER = [IO.Path]::GetFullPath($Vst3Worker)
        Invoke-SignalCheck 'native-plugin-signals' @('test', '-p', 'audiorouter-plugin-host', '--test', 'deterministic_audio', '--features', 'test-fixtures', '--release', '--locked', '--target-dir', $TargetDirectory, '--', '--ignored', '--nocapture')
    }
    $failed = @($results | Where-Object { $_.exitCode -ne 0 })
    $summary = [ordered]@{ generatedAtUtc = [DateTime]::UtcNow.ToString('o'); platform = [Environment]::OSVersion.VersionString; qualified = ($failed.Count -eq 0); nativeFixturesRequested = [bool]$NativeFixtures; checks = $results.ToArray() }
    $summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $artifactRoot 'qualification.json') -Encoding UTF8
    if ($failed.Count -ne 0) { throw "Audio qualification failed: $($failed.name -join ', '). See $artifactRoot/qualification.json" }
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previousEnvironment[$name], 'Process') }
    Set-Location -LiteralPath $previousLocation.Path
}
