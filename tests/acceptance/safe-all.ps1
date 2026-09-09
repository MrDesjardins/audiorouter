param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$acceptanceRoot = Join-Path $repositoryRoot 'tests\acceptance'
$steps = @(
    @{ Name = 'M00 toolchain compatibility'; Script = Join-Path $acceptanceRoot 'm00-toolchain.ps1' },
    @{ Name = 'M00 native compile'; Script = Join-Path $acceptanceRoot 'm00-native-build.ps1' },
    @{ Name = 'M00 native format inventory'; Script = Join-Path $acceptanceRoot 'm00-native-format-inventory.ps1' },
    @{ Name = 'M00 pinned SysVAD qualification'; Script = Join-Path $repositoryRoot 'tools\m00-sysvad\qualify.ps1' },
    @{ Name = 'M01 CLI'; Script = Join-Path $acceptanceRoot 'm01-cli.ps1' },
    @{ Name = 'M04 DSP and recording'; Script = Join-Path $acceptanceRoot 'm04-dsp-recording.ps1' },
    @{ Name = 'M05 UI'; Script = Join-Path $acceptanceRoot 'm05-ui.ps1' },
    @{ Name = 'M06 SDK installer'; Script = Join-Path $acceptanceRoot 'm06-sdk-installer.ps1' },
    @{ Name = 'M06 VST3 SDK'; Script = Join-Path $acceptanceRoot 'm06-vst3-sdk.ps1' },
    @{ Name = 'M06 native VST3 worker'; Script = Join-Path $acceptanceRoot 'm06-vst3-worker.ps1' },
    @{ Name = 'M06 VST2 fixture'; Script = Join-Path $acceptanceRoot 'm06-vst2-state-fixture.ps1' },
    @{ Name = 'M07 headless'; Script = Join-Path $acceptanceRoot 'm07-headless.ps1' },
    @{ Name = 'M08 release'; Script = Join-Path $acceptanceRoot 'm08-release.ps1' },
    @{ Name = 'M08 traceability'; Script = Join-Path $acceptanceRoot 'm08-traceability.ps1' },
    @{ Name = 'Documentation'; Script = Join-Path $acceptanceRoot 'docs.ps1' }
)

foreach ($step in $steps) {
    Write-Output "--- $($step.Name) ---"
    # Invoke checked-in scripts in this runner so their cleanup/final status
    # remains attached to the acceptance process. Nested PowerShell runners
    # can outlive the parent and hide a failed or incomplete terminal result.
    # Do not inspect LASTEXITCODE here: acceptance steps may intentionally run
    # negative child-process cases and leave that sentinel nonzero after
    # successfully validating the expected rejection. Each step owns its
    # native-command checks and throws on an actual failure.
    & $step.Script
}

Write-Output 'Safe acceptance chain passed.'
Write-Output 'Scope: compile/portable/SDK/reference-driver qualification only; live audio, driver installation, signing-mode changes, plugin registration, startup registration, and machine audio configuration are excluded.'
