param(
    # Everything this run builds or writes goes here (deleted and recreated
    # only when -Clean is given).
    [string]$OutputDirectory = '',
    # The pinned VST3 SDK checkout; tools/m06-vst3-sdk/install.ps1 fetches it
    # here when it is missing.
    [string]$SdkDirectory = '',
    # CMake used for the SDK sample plugins; the default is the one on PATH.
    [string]$Cmake = 'cmake',
    # Skip the pinned open-source DPF download (needs the network).
    [switch]$SkipThirdParty,
    # Reuse what an earlier run built in the same output folder.
    [switch]$SkipBuild,
    # Only run tests whose name matches this regular expression.
    [string]$Only = '',
    [switch]$Clean
)

# Reproducible real-worker VST qualification.
#
# Builds the four plugin workers (VST2/VST3, x64/x86), the repository's own
# test plugins, the pinned Steinberg SDK sample plugins and (optionally) a
# pinned, hash-checked DPF release, then runs every opt-in real-worker test
# that needs nothing installed on the machine. Each test runs by its exact
# name and must report one passed test: a renamed or filtered-out test is a
# failure, never a silent pass. Results go to <output>\report.json.
#
# It opens no audio device and no plugin window. Editors, live endpoints,
# licensed plugins, hardware MIDI and the owner's own plugin collection are
# out of scope (see docs/operations/vst-qualification.md).
#
# Local:   powershell -NoProfile -ExecutionPolicy Bypass -File tools/vst-qualification/run.ps1
# Nightly: .github/workflows/vst-nightly.yml

$ErrorActionPreference = 'Stop'
# Windows PowerShell's progress bar makes downloads and Expand-Archive many
# times slower.
$ProgressPreference = 'SilentlyContinue'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repositoryRoot 'target\vst-nightly' }
if (-not $SdkDirectory) { $SdkDirectory = Join-Path $repositoryRoot 'third_party\vst3sdk' }
$out = [System.IO.Path]::GetFullPath($OutputDirectory)
$sdk = [System.IO.Path]::GetFullPath($SdkDirectory)
$sdkRevision = '3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96'
$dpfRelease = 'https://github.com/DISTRHO/DPF-Plugins/releases/download/v1.7'
$dpfArchives = @(
    @{ Name = 'DPF-Plugins-v1.7-win64.zip'; Sha256 = 'fdc609e0936e9c999395b9ddd98a9a72739cb672d8838ab71849469aee50ad98'; Folder = 'dpf64' },
    @{ Name = 'DPF-Plugins-v1.7-win32.zip'; Sha256 = '17989efe16b4530affa720605cf8b40acf5f0490e05ae68de4c1d6c1f7b4dfb3'; Folder = 'dpf32' }
)

if ($Clean -and (Test-Path -LiteralPath $out)) { Remove-Item -LiteralPath $out -Recurse -Force }
New-Item -ItemType Directory -Force -Path $out | Out-Null
$logs = Join-Path $out 'logs'
$workers = Join-Path $out 'workers'
$fixtures = Join-Path $out 'fixtures'
$thirdParty = Join-Path $out 'third-party'
foreach ($folder in @($logs, $workers, $fixtures, $thirdParty)) { New-Item -ItemType Directory -Force -Path $folder | Out-Null }

function Invoke-Logged([string]$Log, [string]$File, [string[]]$Arguments) {
    # Native tools write progress to stderr; under "Stop" Windows PowerShell
    # would treat that as fatal, so decide on the exit code alone.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $File @Arguments *>&1 | ForEach-Object { "$_" } | Out-File -FilePath $Log -Append -Encoding utf8
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
    return $code
}

function Invoke-Step([string]$Name, [string]$File, [string[]]$Arguments) {
    $log = Join-Path $logs "$Name.log"
    Write-Output "== $Name"
    $code = Invoke-Logged $log $File $Arguments
    if ($code -ne 0) {
        Get-Content -LiteralPath $log -Tail 40 | Write-Output
        throw "$Name failed with exit code $code (log: $log)"
    }
}

function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

function Find-Module([string]$Bundle) {
    # The binary inside a .vst3 bundle, for hashing.
    if (Test-Path -LiteralPath $Bundle -PathType Leaf) { return $Bundle }
    $module = Get-ChildItem -LiteralPath (Join-Path $Bundle 'Contents') -Recurse -File -Filter '*.vst3' | Select-Object -First 1
    if (-not $module) { throw "no module inside $Bundle" }
    return $module.FullName
}

# ---------------------------------------------------------------- inputs ---

if (-not (Test-Path -LiteralPath (Join-Path $sdk 'CMakeLists.txt'))) {
    Invoke-Step 'sdk-install' 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        (Join-Path $repositoryRoot 'tools\m06-vst3-sdk\install.ps1'), '-Destination', $sdk)
}
$foundRevision = (& git -C $sdk rev-parse HEAD).Trim()
if ($foundRevision -ne $sdkRevision) { throw "VST3 SDK revision mismatch: expected $sdkRevision, found $foundRevision" }

$sdk64 = Join-Path $out 'build\sdk-x64'
$sdk32 = Join-Path $out 'build\sdk-x86'
$vst2Fixtures = Join-Path $fixtures 'vst2'
$instrumentRoot = Join-Path $out 'build\vst2-test-instrument'
$worker = @{
    Vst2X64 = Join-Path $workers 'audiorouter-plugin-worker.exe'
    Vst2X86 = Join-Path $workers 'audiorouter-plugin-worker-x86.exe'
    Vst3X64 = Join-Path $workers 'audiorouter-vst3-worker.exe'
    Vst3X86 = Join-Path $workers 'audiorouter-vst3-worker-x86.exe'
}

if (-not $SkipBuild) {
    # SDK sample plugins. SMTG_CREATE_PLUGIN_LINK=0 keeps the build from
    # linking into the system VST3 folder (that needs administrator rights).
    Invoke-Step 'sdk-configure-x64' $Cmake @('-S', $sdk, '-B', $sdk64, '-A', 'x64', '-DSMTG_CREATE_PLUGIN_LINK=0')
    Invoke-Step 'sdk-build-x64' $Cmake @('--build', $sdk64, '--config', 'Release', '--parallel', '4',
        '--target', 'again', 'mda-vst3', 'utf16-name', 'validator', '--', '/nr:false')
    Invoke-Step 'sdk-configure-x86' $Cmake @('-S', $sdk, '-B', $sdk32, '-A', 'Win32', '-DSMTG_CREATE_PLUGIN_LINK=0',
        '-DSMTG_ENABLE_VSTGUI_SUPPORT=OFF')
    Invoke-Step 'sdk-build-x86' $Cmake @('--build', $sdk32, '--config', 'Release', '--parallel', '4',
        '--target', 'mda-vst3', '--', '/nr:false')

    # The four workers, built the way tools/build-desktop.ps1 builds them.
    Invoke-Step 'worker-vst2-x64' 'cargo' @('build', '--locked', '--release', '-p', 'audiorouter-plugin-host', '--bin', 'audiorouter-plugin-worker')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'target\release\audiorouter-plugin-worker.exe') -Destination $worker.Vst2X64 -Force
    Invoke-Step 'worker-vst2-x86' 'cargo' @('build', '--locked', '--release', '-p', 'audiorouter-plugin-host', '--bin', 'audiorouter-plugin-worker',
        '--target', 'i686-pc-windows-msvc')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'target\i686-pc-windows-msvc\release\audiorouter-plugin-worker.exe') -Destination $worker.Vst2X86 -Force
    $nativeBuild = Join-Path $repositoryRoot 'tools\m06-vst3-worker\build.ps1'
    Invoke-Step 'worker-vst3-x64' 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $nativeBuild,
        '-SdkInclude', $sdk, '-Release', '-Arch', 'x64', '-Output', $worker.Vst3X64)
    Invoke-Step 'worker-vst3-x86' 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $nativeBuild,
        '-SdkInclude', $sdk, '-Release', '-Arch', 'x86', '-Output', $worker.Vst3X86)

    # Repository-owned test plugins.
    Invoke-Step 'fixtures-vst2-c' 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        (Join-Path $repositoryRoot 'tests\acceptance\m06-vst2-state-fixture.ps1'), '-OutputDirectory', $vst2Fixtures, '-BuildOnly')
    $instrumentManifest = Join-Path $repositoryRoot 'tools\vst2-test-instrument\Cargo.toml'
    foreach ($variant in @(
            @{ Name = 'instrument'; Features = @() },
            @{ Name = 'bus'; Features = @('--features', 'bus-fixture') },
            @{ Name = 'shell'; Features = @('--features', 'shell-fixture') })) {
        $target = Join-Path $instrumentRoot $variant.Name
        Invoke-Step "fixtures-vst2-$($variant.Name)" 'cargo' (@('build', '--locked', '--release', '--manifest-path', $instrumentManifest,
                '--target-dir', $target) + $variant.Features)
        $folder = Join-Path $fixtures "vst2-$($variant.Name)"
        New-Item -ItemType Directory -Force -Path $folder | Out-Null
        Copy-Item -LiteralPath (Join-Path $target 'release\ar_test_instrument.dll') -Destination (Join-Path $folder 'ar_test_instrument.dll') -Force
    }
    $fixtureBuild = Join-Path $repositoryRoot 'tools\m06-vst3-worker\fixtures\build-fixture.ps1'
    foreach ($name in @('Sample64Only', 'TwoClasses')) {
        Invoke-Step "fixtures-vst3-$name" 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $fixtureBuild,
            '-SdkInclude', $sdk, '-Name', $name, '-OutputDirectory', (Join-Path $fixtures 'vst3'))
    }

    # Pinned open-source plugins (DPF, ISC licence), checked by SHA-256.
    if (-not $SkipThirdParty) {
        foreach ($archive in $dpfArchives) {
            $zip = Join-Path $thirdParty $archive.Name
            if (-not (Test-Path -LiteralPath $zip) -or (Hash $zip) -ne $archive.Sha256) {
                Write-Output "== download $($archive.Name)"
                Invoke-WebRequest -UseBasicParsing -Uri "$dpfRelease/$($archive.Name)" -OutFile $zip
            }
            $actual = Hash $zip
            if ($actual -ne $archive.Sha256) { throw "$($archive.Name) hash mismatch: $actual" }
            $folder = Join-Path $thirdParty $archive.Folder
            if (Test-Path -LiteralPath $folder) { Remove-Item -LiteralPath $folder -Recurse -Force }
            Expand-Archive -LiteralPath $zip -DestinationPath $folder
        }
    }
}

$again = Join-Path $sdk64 'VST3\Release\again.vst3'
$mda64 = Join-Path $sdk64 'VST3\Release\mda-vst3.vst3'
$mda32 = Join-Path $sdk32 'VST3\Release\mda-vst3.vst3'
$utf16 = Join-Path $sdk64 'VST3\Release\utf16-name.vst3'
$validator = Join-Path $sdk64 'bin\Release\validator.exe'
$sample64 = Join-Path $fixtures 'vst3\Sample64Only.vst3'
$twoClasses = Join-Path $fixtures 'vst3\TwoClasses.vst3'
$vst2State = Join-Path $vst2Fixtures 'audiorouter-vst2-state-fixture.dll'
$vst2Bus = Join-Path $fixtures 'vst2-bus\ar_test_instrument.dll'
$vst2Shell = Join-Path $fixtures 'vst2-shell\ar_test_instrument.dll'
$vst2Instrument = Join-Path $fixtures 'vst2-instrument\ar_test_instrument.dll'
$dpf64 = Join-Path $thirdParty 'dpf64\DPF-Plugins-v1.7'
$dpf32 = Join-Path $thirdParty 'dpf32\DPF-Plugins-v1.7'
$jx10Class = '5653544D44416A6D6461206A78313000'

foreach ($path in @($worker.Vst2X64, $worker.Vst2X86, $worker.Vst3X64, $worker.Vst3X86, $again, $mda64, $mda32, $utf16,
        $validator, $sample64, $twoClasses, $vst2State, $vst2Bus, $vst2Shell, $vst2Instrument)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "missing build output: $path" }
}
$haveDpf = (-not $SkipThirdParty) -and (Test-Path -LiteralPath $dpf64) -and (Test-Path -LiteralPath $dpf32)

# Supplemental evidence only: the validator passing says nothing about
# AudioRouter's own worker, which the tests below exercise.
Invoke-Step 'host-self-test-x64' $worker.Vst3X64 @('--self-test-host')
Invoke-Step 'host-self-test-x86' $worker.Vst3X86 @('--self-test-host')
Invoke-Step 'validator-again' $validator @($again)
Invoke-Step 'validator-mda' $validator @($mda64)

# ----------------------------------------------------------------- tests ---

$hostTests = @{ Package = 'audiorouter-plugin-host' }
$common = @{
    AUDIOROUTER_PLUGIN_WORKER_PATH = $worker.Vst2X64
    AUDIOROUTER_PLUGIN_WORKER_X86_PATH = $worker.Vst2X86
    AUDIOROUTER_VST3_WORKER_PATH = $worker.Vst3X64
    AUDIOROUTER_VST3_WORKER_X86_PATH = $worker.Vst3X86
    AUDIOROUTER_TEST_VST2_WORKER = $worker.Vst2X64
    AUDIOROUTER_TEST_VST3_WORKER = $worker.Vst3X64
    AUDIOROUTER_VST3_NATIVE_WORKER = $worker.Vst3X64
    AUDIOROUTER_TEST_VST3_NATIVE_WORKER = $worker.Vst3X64
    AUDIOROUTER_SIGNAL_ARTIFACTS = (Join-Path $out 'signals')
}
function T([string]$Package, [string]$Target, [string]$Name, [hashtable]$Environment, [string]$Label = '', [string[]]$Features = @()) {
    if (-not $Label) { $Label = $Name }
    return @{ Package = $Package; Target = $Target; Name = $Name; Environment = $Environment; Label = $Label; Features = $Features }
}
$ph = 'audiorouter-plugin-host'
$ctl = 'audiorouter-control'
$fx = @('test-fixtures')
$tests = @(
    # VST2 worker with the repository fixtures.
    (T $ph 'worker_process' 'verified_worker_loads_and_processes_an_opt_in_vst2_fixture' @{ AUDIOROUTER_VST2_FIXTURE = $vst2State } -Features $fx),
    (T $ph 'worker_process' 'verified_worker_applies_restored_vst2_chunk_state' @{ AUDIOROUTER_VST2_FIXTURE = $vst2State } -Features $fx),
    (T $ph 'worker_process' 'verified_worker_imports_vst2_program_chunk_with_program_scope_index' @{ AUDIOROUTER_VST2_FIXTURE = $vst2State } -Features $fx),
    (T $ph 'worker_process' 'verified_worker_applies_vst2_bank_chunk_with_bank_scope_index' @{ AUDIOROUTER_VST2_FIXTURE = (Join-Path $vst2Fixtures 'audiorouter-vst2-bank-fixture.dll') } -Features $fx),
    (T $ph 'worker_process' 'verified_worker_rejects_nonfinite_vst2_output' @{ AUDIOROUTER_VST2_FIXTURE = (Join-Path $vst2Fixtures 'audiorouter-vst2-nonfinite-fixture.dll') } -Features $fx),
    (T $ph 'worker_process' 'verified_worker_contains_native_vst2_fault' @{ AUDIOROUTER_VST2_FIXTURE = (Join-Path $vst2Fixtures 'audiorouter-vst2-crash-fixture.dll') } 'vst2 fault: crash' -Features $fx),
    (T $ph 'worker_process' 'verified_worker_contains_native_vst2_fault' @{ AUDIOROUTER_VST2_FIXTURE = (Join-Path $vst2Fixtures 'audiorouter-vst2-hang-fixture.dll') } 'vst2 fault: hang' -Features $fx),
    (T $ph 'worker_process' 'shared_vst2_chain_routes_parameters_and_individual_state' @{ AUDIOROUTER_VST2_CHAIN_FIXTURES = $vst2Fixtures } -Features $fx),
    (T $ph 'worker_process' 'shared_vst2_chain_fault_silences_all_members_and_preserves_other_workers' @{ AUDIOROUTER_VST2_CHAIN_FIXTURES = $vst2Fixtures } -Features $fx),
    (T $ph 'worker_process' 'shared_vst2_chain_is_silenced_when_it_passes_protected_audio_through' @{ AUDIOROUTER_VST2_CHAIN_FIXTURES = $vst2Fixtures } -Features $fx),
    (T $ph 'deterministic_audio' 'native_vst2_fixture_matches_exact_gain_and_sample_offset_automation' @{ AUDIOROUTER_VST2_FIXTURE = $vst2State } -Features $fx),
    (T $ph 'vst2_bus_discovery' 'deterministic_bus_fixture_attenuates_main_without_mixing_key' @{ AUDIOROUTER_TEST_VST2_BUS_FIXTURE = $vst2Bus }),
    (T $ph 'vst2_bus_discovery' 'labeled_pins_are_discovered_but_unlabeled_surround_is_refused' @{ AUDIOROUTER_TEST_VST2_BUS_FIXTURE = $vst2Bus }),
    (T $ph 'vst2_bus_discovery' 'real_vst2_worker_describes_buses_and_keeps_processing' @{ AUDIOROUTER_TEST_VST2_EFFECT = $vst2State }),
    (T $ph 'vst2_shell_discovery' 'a_shell_fixture_lists_three_named_sub_plugins' @{ AUDIOROUTER_TEST_VST2_SHELL_FIXTURE = $vst2Shell }),
    (T $ph 'vst2_shell_discovery' 'real_worker_keeps_exact_shell_sub_id_and_refuses_unknown_selection' @{ AUDIOROUTER_TEST_VST2_SHELL_FIXTURE = $vst2Shell }),
    (T $ph 'vst2_shell_discovery' 'the_worker_protocol_lists_and_selects_real_programs' @{ AUDIOROUTER_TEST_VST2_EFFECT = $vst2State }),
    (T $ph 'vst2_shell_discovery' 'a_real_plugin_lists_and_selects_bounded_programs' @{ AUDIOROUTER_TEST_VST2_PLUGIN = $vst2State }),
    # VST3 worker with the SDK samples and the repository fixtures.
    (T $ph 'worker_process' 'verified_native_vst3_worker_processes_an_opt_in_fixture' @{ AUDIOROUTER_VST3_FIXTURE = $again } -Features $fx),
    (T $ph 'worker_process' 'verified_native_vst3_worker_processes_an_opt_in_multi_bus_fixture' @{ AUDIOROUTER_VST3_FIXTURE = $again } -Features $fx),
    (T $ph 'worker_process' 'verified_native_vst3_async_bus_worker_bridges_the_graph_scheduler' @{ AUDIOROUTER_VST3_FIXTURE = $again } -Features $fx),
    (T $ph 'worker_process' 'native_vst3_fixture_signal_report' @{ AUDIOROUTER_VST3_FIXTURE = $again } -Features $fx),
    # Hidden parent window: open, resize and close three times while audio flows.
    (T $ph 'worker_process' 'native_vst3_editor_survives_repeated_open_resize_and_close' @{ AUDIOROUTER_VST3_FIXTURE = $again } 'vst3 editor lifecycle (AGain, fixed size)' -Features $fx),
    (T $ph 'deterministic_audio' 'native_vst3_again_matches_exact_gain_and_bypass_transfer' @{ AUDIOROUTER_VST3_FIXTURE = $again } -Features $fx),
    (T $ph 'vst3_bus_discovery' 'real_worker_describes_buses_without_interrupting_processing' @{ AUDIOROUTER_TEST_VST3_EFFECT = $again }),
    (T $ph 'vst3_bus_discovery' 'a_missing_requested_class_never_substitutes_another_effect' @{ AUDIOROUTER_TEST_VST3_EFFECT = $again }),
    (T $ph 'vst3_bus_discovery' 'each_again_class_keeps_its_own_bus_identity' @{ AUDIOROUTER_VST3_FIXTURE = $again }),
    (T $ph 'vst3_instrument' 'a_vst3_effect_is_refused_as_an_instrument_with_its_own_reason' @{ AUDIOROUTER_TEST_VST3_EFFECT = $again }),
    (T $ph 'vst3_instrument' 'a_vst3_instrument_sounds_when_it_gets_a_note_and_not_before' @{ AUDIOROUTER_TEST_VST3_INSTRUMENT = $mda64; AUDIOROUTER_TEST_VST3_INSTRUMENT_CLASS = $jx10Class } 'vst3 instrument x64 (mda JX10)'),
    (T $ph 'vst3_instrument' 'a_vst3_instrument_sounds_when_it_gets_a_note_and_not_before' @{ AUDIOROUTER_TEST_VST3_INSTRUMENT = $mda32; AUDIOROUTER_TEST_VST3_INSTRUMENT_CLASS = $jx10Class; AUDIOROUTER_TEST_VST3_WORKER = $worker.Vst3X86 } 'vst3 instrument x86 (mda JX10)'),
    (T $ph 'vst3_multi_class' 'each_class_of_a_module_loads_alone_and_keeps_its_own_state' @{ AUDIOROUTER_TEST_VST3_TWOCLASSES_FIXTURE = $twoClasses }),
    (T $ph 'vst3_programs' 'a_vst3_lists_bounded_program_names_and_refuses_selection' @{ AUDIOROUTER_TEST_VST3_EFFECT = $utf16; AUDIOROUTER_TEST_VST3_UNITS_MIN = '5' } 'vst3 units (UTF16Name)'),
    (T $ph 'vst3_programs' 'a_vst3_lists_bounded_program_names_and_refuses_selection' @{ AUDIOROUTER_TEST_VST3_EFFECT = $mda64 } 'vst3 programs (mda)'),
    (T $ph 'vst3_sample_format' 'a_plugin_that_only_takes_64_bit_samples_is_refused_with_a_plain_reason' @{ AUDIOROUTER_TEST_VST3_SAMPLE64_FIXTURE = $sample64 }),
    (T $ph 'vst3_state_fallback' 'a_refused_state_falls_back_to_the_saved_parameter_values' @{ AUDIOROUTER_TEST_VST3_EFFECT = $again }),
    # The control plane: dispatcher, saved routes, graph transactions.
    (T $ctl '--lib' 'plugins_buses_reports_the_real_again_auxiliary_input' @{ AUDIOROUTER_VST3_FIXTURE = $again }),
    (T $ctl '--lib' 'add_sidechain_port_adds_the_real_again_auxiliary_input_once_and_undoes' @{ AUDIOROUTER_VST3_FIXTURE = $again }),
    (T $ctl '--lib' 'backend_plugin_stage_path_prepares_and_compiles_a_verified_sidechain_route' @{ AUDIOROUTER_VST3_FIXTURE = $again }),
    (T $ctl '--lib' 'backend_compiles_the_verified_vst2_labelled_key_route' @{ AUDIOROUTER_TEST_VST2_BUS_FIXTURE = $vst2Bus }),
    (T $ctl '--lib' 'discovered_sidechain_port_plans_commit_replay_and_undo_atomically' @{ AUDIOROUTER_TEST_VST2_BUS_FIXTURE = $vst2Bus }),
    (T $ctl '--lib' 'plugins_programs_dispatches_real_vst3_unit_hierarchy' @{ AUDIOROUTER_TEST_VST3_EFFECT = $utf16 }),
    (T $ctl '--lib' 'shell_scan_and_saved_route_bind_the_exact_sub_id' @{ AUDIOROUTER_TEST_VST2_SHELL_FIXTURE = $vst2Shell }),
    (T $ctl '--lib' 'an_instrument_node_starts_a_worker_and_plays_midi' @{ AUDIOROUTER_TEST_VST2_INSTRUMENT = $vst2Instrument; AUDIOROUTER_TEST_INSTRUMENT = $vst2Instrument })
)
if ($haveDpf) {
    $tests += @(
        (T $ph 'vst3_stateless_state' 'a_stateless_dpf_plugin_restores_its_own_saved_state' @{ AUDIOROUTER_TEST_VST3_STATELESS = (Join-Path $dpf64 'ProM.vst3') } 'vst3 stateless restore x64 (DPF ProM)'),
        (T $ph 'vst3_stateless_state' 'a_stateless_dpf_plugin_restores_its_own_saved_state' @{ AUDIOROUTER_TEST_VST3_STATELESS = (Join-Path $dpf32 'ProM.vst3'); AUDIOROUTER_TEST_VST3_NATIVE_WORKER = $worker.Vst3X86 } 'vst3 stateless restore x86 (DPF ProM)'),
        (T $ph 'vst3_instrument' 'a_vst3_instrument_sounds_when_it_gets_a_note_and_not_before' @{ AUDIOROUTER_TEST_VST3_INSTRUMENT = (Join-Path $dpf64 'Kars.vst3') } 'vst3 instrument x64 (DPF Kars)'),
        (T $ph 'worker_process' 'native_vst3_editor_survives_repeated_open_resize_and_close' @{ AUDIOROUTER_VST3_FIXTURE = (Join-Path $dpf64 'ProM.vst3') } 'vst3 editor lifecycle (DPF ProM, resizable)' -Features $fx)
    )
}
if ($Only) { $tests = @($tests | Where-Object { $_.Label -match $Only -or $_.Name -match $Only }) }

$testLists = @{}
function Resolve-TestPath($Test) {
    # The full path of the test, from cargo's own list, so --exact can run it
    # and a renamed test fails here instead of passing with zero tests.
    $key = "$($Test.Package)|$($Test.Target)|$($Test.Features -join ',')"
    if (-not $testLists.ContainsKey($key)) {
        $arguments = @('test', '--locked', '-p', $Test.Package)
        if ($Test.Target -eq '--lib') { $arguments += '--lib' } else { $arguments += @('--test', $Test.Target) }
        if ($Test.Features.Count -gt 0) { $arguments += @('--features', ($Test.Features -join ',')) }
        $arguments += @('--', '--list', '--ignored')
        $log = Join-Path $logs ("list-" + ($key -replace '[^A-Za-z0-9]+', '-') + '.log')
        if (Test-Path -LiteralPath $log) { Remove-Item -LiteralPath $log }
        $code = Invoke-Logged $log 'cargo' $arguments
        if ($code -ne 0) { Get-Content -LiteralPath $log -Tail 40 | Write-Output; throw "listing $key failed (log: $log)" }
        $testLists[$key] = @(Get-Content -LiteralPath $log | Where-Object { $_ -match ': test$' } | ForEach-Object { $_ -replace ': test$', '' })
    }
    $found = @($testLists[$key] | Where-Object { $_ -eq $Test.Name -or $_.EndsWith("::$($Test.Name)") })
    if ($found.Count -ne 1) { return $null }
    return $found[0]
}

$results = @()
$failed = 0
$index = 0
foreach ($test in $tests) {
    $index++
    $environment = $common.Clone()
    foreach ($entry in $test.Environment.GetEnumerator()) { $environment[$entry.Key] = $entry.Value }
    $path = Resolve-TestPath $test
    $logName = ('{0:D2}-{1}' -f $index, ($test.Label -replace '[^A-Za-z0-9]+', '-'))
    $log = Join-Path $logs "$logName.log"
    if (Test-Path -LiteralPath $log) { Remove-Item -LiteralPath $log }
    $status = 'failed'
    $passed = 0
    $seconds = 0
    if (-not $path) {
        "test '$($test.Name)' was not found as exactly one ignored test in $($test.Package) $($test.Target)" | Out-File -FilePath $log -Encoding utf8
    } else {
        $saved = @{}
        foreach ($entry in $environment.GetEnumerator()) {
            $saved[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key, 'Process')
            [Environment]::SetEnvironmentVariable($entry.Key, [string]$entry.Value, 'Process')
        }
        $arguments = @('test', '--locked', '-p', $test.Package)
        if ($test.Target -eq '--lib') { $arguments += '--lib' } else { $arguments += @('--test', $test.Target) }
        if ($test.Features.Count -gt 0) { $arguments += @('--features', ($test.Features -join ',')) }
        $arguments += @('--', '--ignored', '--exact', $path, '--nocapture', '--test-threads', '1')
        $watch = [Diagnostics.Stopwatch]::StartNew()
        try {
            $code = Invoke-Logged $log 'cargo' $arguments
        } finally {
            foreach ($entry in $saved.GetEnumerator()) { [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process') }
        }
        $seconds = [math]::Round($watch.Elapsed.TotalSeconds, 1)
        $summary = Get-Content -LiteralPath $log | Where-Object { $_ -match '^test result: ' } | Select-Object -Last 1
        if ($summary -match '(\d+) passed; (\d+) failed') {
            $passed = [int]$Matches[1]
            if ($code -eq 0 -and $passed -eq 1 -and [int]$Matches[2] -eq 0) { $status = 'passed' }
        }
    }
    if ($status -ne 'passed') { $failed++ }
    Write-Output ("{0,-7} {1,6}s  {2}" -f $status, $seconds, $test.Label)
    $results += [ordered]@{ label = $test.Label; test = $path; package = $test.Package; target = $test.Target; status = $status; passed = $passed; seconds = $seconds; log = "logs/$logName.log" }
}

# A bounded census over the same plugins: every format and bitness, effects
# and instruments, state save/restore in the same and a fresh worker.
$census = $null
if (-not $Only) {
    $censusRoot = Join-Path $out 'census-plugins'
    if (Test-Path -LiteralPath $censusRoot) { Remove-Item -LiteralPath $censusRoot -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $censusRoot | Out-Null
    foreach ($pair in @(@('sdk-x64', $again), @('sdk-x64', $mda64), @('sdk-x64', $utf16), @('sdk-x86', $mda32), @('vst2', $vst2State))) {
        $destination = Join-Path $censusRoot $pair[0]
        New-Item -ItemType Directory -Force -Path $destination | Out-Null
        Copy-Item -LiteralPath $pair[1] -Destination $destination -Recurse -Force
    }
    $directories = @($censusRoot)
    if ($haveDpf) { $directories += @($dpf64, $dpf32) }
    $censusReport = Join-Path $out 'census.json'
    $saved = @{}
    $censusEnvironment = $common.Clone()
    $censusEnvironment['AUDIOROUTER_CENSUS_DIRS'] = ($directories -join ';')
    $censusEnvironment['AUDIOROUTER_CENSUS_SKIP'] = 'BC ,Blue Cat,BlueCat'
    $censusEnvironment['AUDIOROUTER_CENSUS_REPORT'] = $censusReport
    foreach ($entry in $censusEnvironment.GetEnumerator()) {
        $saved[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key, 'Process')
        [Environment]::SetEnvironmentVariable($entry.Key, [string]$entry.Value, 'Process')
    }
    $log = Join-Path $logs 'census.log'
    if (Test-Path -LiteralPath $log) { Remove-Item -LiteralPath $log }
    try {
        $code = Invoke-Logged $log 'cargo' @('test', '--locked', '-p', $ctl, '--lib', '--', '--ignored', '--exact',
            'plugins::census_tests::plugin_census', '--nocapture')
    } finally {
        foreach ($entry in $saved.GetEnumerator()) { [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process') }
    }
    $censusStatus = if ($code -eq 0 -and (Test-Path -LiteralPath $censusReport)) { 'passed' } else { 'failed' }
    if ($censusStatus -ne 'passed') { $failed++ }
    $census = [ordered]@{ status = $censusStatus; report = 'census.json'; log = 'logs/census.log' }
    Write-Output ("{0,-7} census over the SDK samples, repository fixtures{1}" -f $censusStatus, $(if ($haveDpf) { ' and DPF v1.7' } else { '' }))
}

# ---------------------------------------------------------------- report ---

$inputs = [ordered]@{}
foreach ($entry in $worker.GetEnumerator()) { $inputs["worker:$($entry.Key)"] = Hash $entry.Value }
foreach ($pair in @(@('again-x64', $again), @('mda-x64', $mda64), @('mda-x86', $mda32), @('utf16name-x64', $utf16),
        @('sample64only', $sample64), @('twoclasses', $twoClasses))) { $inputs["vst3:$($pair[0])"] = Hash (Find-Module $pair[1]) }
foreach ($pair in @(@('state', $vst2State), @('bus', $vst2Bus), @('shell', $vst2Shell), @('instrument', $vst2Instrument))) { $inputs["vst2:$($pair[0])"] = Hash $pair[1] }
if ($haveDpf) { foreach ($archive in $dpfArchives) { $inputs["dpf:$($archive.Name)"] = $archive.Sha256 } }

$commit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
$dirty = [bool](& git -C $repositoryRoot status --porcelain --untracked-files=no)
$report = [ordered]@{
    schema = 1
    generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    commit = $commit
    dirty = $dirty
    sdkRevision = $sdkRevision
    thirdParty = $haveDpf
    inputs = $inputs
    tests = $results
    census = $census
    failed = $failed
}
$reportPath = Join-Path $out 'report.json'
$report | ConvertTo-Json -Depth 6 | Out-File -FilePath $reportPath -Encoding utf8
$total = $results.Count + $(if ($census) { 1 } else { 0 })
Write-Output "VST qualification: $($total - $failed) of $total passed. Report: $reportPath"
if ($failed -gt 0) { exit 1 }
