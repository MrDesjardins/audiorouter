$ErrorActionPreference = "Stop"
$workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
foreach ($profile in @("tauri.release.conf.json", "tauri.app-only.release.conf.json")) {
    $profileConfig = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/$profile") -Raw | ConvertFrom-Json
    $libraryResource = $profileConfig.bundle.resources.PSObject.Properties["../resources/sound-library"]
    if ($null -eq $libraryResource -or $libraryResource.Value -ne "sound-library") {
        throw "$profile must bundle the offline sound-library directory at sound-library."
    }
}
$config = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/tauri.release.conf.json") -Raw | ConvertFrom-Json
$nsis = $config.bundle.windows.nsis
if ($nsis.installMode -ne "currentUser") { throw "Cable installer must remain per-user." }
if ($nsis.installerHooks -ne "installer-hooks.nsh") { throw "NSIS uninstall hook is not configured." }
$resources = $config.bundle.resources
$resourceValues = @($resources.PSObject.Properties | ForEach-Object { $_.Value })
foreach ($name in @(
    "audiorouter-driver-helper.exe",
    "driver/audioroutervirtual.inf",
    "driver/audioroutervirtual.sys",
    "driver/audioroutervirtual.cat",
    "driver/LICENSE-MS-PL.txt",
    "driver/package.json"
)) {
    if (-not ($resourceValues -contains $name)) { throw "NSIS resources omit $name." }
}
$hook = Get-Content -LiteralPath (Join-Path $workspace "src-tauri/installer-hooks.nsh") -Raw
foreach ($required in @(
    "NSIS_HOOK_PREUNINSTALL",
    "MB_YESNO|MB_DEFBUTTON2",
    "Also remove AudioRouter virtual cables?",
    '$INSTDIR\resources\audiorouter-driver-helper.exe',
    'remove'' $ar_remove_exit',
    "The app will still be uninstalled"
)) {
    if (-not $hook.Contains($required)) { throw "NSIS hook is missing required behavior: $required" }
}
Write-Output "WP-11 installer resource and uninstall-hook checks passed"
