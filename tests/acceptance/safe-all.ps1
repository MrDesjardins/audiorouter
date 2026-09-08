param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$acceptanceRoot = Join-Path $repositoryRoot 'tests\acceptance'
$steps = @(
    @{ Name = 'M00 native compile'; Script = Join-Path $acceptanceRoot 'm00-native-build.ps1' },
    @{ Name = 'M00 native format inventory'; Script = Join-Path $acceptanceRoot 'm00-native-format-inventory.ps1' },
    @{ Name = 'M00 pinned SysVAD qualification'; Script = Join-Path $repositoryRoot 'tools\m00-sysvad\qualify.ps1' },
    @{ Name = 'M01 CLI'; Script = Join-Path $acceptanceRoot 'm01-cli.ps1' },
    @{ Name = 'M04 DSP and recording'; Script = Join-Path $acceptanceRoot 'm04-dsp-recording.ps1' },
    @{ Name = 'M05 UI'; Script = Join-Path $acceptanceRoot 'm05-ui.ps1' },
    @{ Name = 'M06 SDK installer'; Script = Join-Path $acceptanceRoot 'm06-sdk-installer.ps1' },
    @{ Name = 'M06 VST3 SDK'; Script = Join-Path $acceptanceRoot 'm06-vst3-sdk.ps1' },
    @{ Name = 'M07 headless'; Script = Join-Path $acceptanceRoot 'm07-headless.ps1' },
    @{ Name = 'M08 release'; Script = Join-Path $acceptanceRoot 'm08-release.ps1' },
    @{ Name = 'Documentation'; Script = Join-Path $acceptanceRoot 'docs.ps1' }
)

foreach ($step in $steps) {
    Write-Output "--- $($step.Name) ---"
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $step.Script
    if ($LASTEXITCODE -ne 0) {
        throw "$($step.Name) failed with exit code $LASTEXITCODE"
    }
}

Write-Output 'Safe acceptance chain passed.'
Write-Output 'Scope: compile/portable/SDK/reference-driver qualification only; live audio, driver installation, signing-mode changes, plugin registration, startup registration, and machine audio configuration are excluded.'
