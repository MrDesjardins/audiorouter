param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'release'
)

$ErrorActionPreference = 'Stop'
$desktopWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$desktopOutput = Join-Path $desktopWorkspace "src-tauri/target/$Profile"

function Invoke-DesktopBuild([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}

Push-Location $desktopWorkspace
try {
    # Both workers are required even when the current route has no plugins.
    # Cargo alone builds the shell, not the separate VST2/VST3 executables.
    $desktopCargoArguments = @('build', '--locked', '--manifest-path', 'Cargo.toml', '-p', 'audiorouter-plugin-host', '--bin', 'audiorouter-plugin-worker', '--target-dir', 'src-tauri/target')
    if ($Profile -eq 'release') { $desktopCargoArguments += '--release' }
    Invoke-DesktopBuild 'cargo.exe' $desktopCargoArguments
    $desktopNativeArguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'tools/m06-vst3-worker/build.ps1', '-Output', (Join-Path $desktopOutput 'audiorouter-vst3-worker.exe'))
    if ($Profile -eq 'release') { $desktopNativeArguments += '-Release' }
    Invoke-DesktopBuild 'powershell.exe' $desktopNativeArguments
    # 32-bit workers host 32-bit plugins (a 32-bit DLL cannot load into a
    # 64-bit process). Cargo's i686 target needs its own target directory.
    $desktopX86Arguments = @('build', '--locked', '--manifest-path', 'Cargo.toml', '-p', 'audiorouter-plugin-host', '--bin', 'audiorouter-plugin-worker', '--target', 'i686-pc-windows-msvc', '--target-dir', 'src-tauri/target-x86')
    if ($Profile -eq 'release') { $desktopX86Arguments += '--release' }
    Invoke-DesktopBuild 'cargo.exe' $desktopX86Arguments
    $desktopX86Built = Join-Path $desktopWorkspace "src-tauri/target-x86/i686-pc-windows-msvc/$Profile/audiorouter-plugin-worker.exe"
    if (Test-Path -LiteralPath $desktopX86Built -PathType Leaf) { Copy-Item -LiteralPath $desktopX86Built -Destination (Join-Path $desktopOutput 'audiorouter-plugin-worker-x86.exe') -Force }
    $desktopX86Native = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'tools/m06-vst3-worker/build.ps1', '-Arch', 'x86', '-Output', (Join-Path $desktopOutput 'audiorouter-vst3-worker-x86.exe'))
    if ($Profile -eq 'release') { $desktopX86Native += '-Release' }
    Invoke-DesktopBuild 'powershell.exe' $desktopX86Native
    Invoke-DesktopBuild 'powershell.exe' @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'tools/stage-mcp-cli.ps1', '-Profile', $Profile, '-Build')
    Invoke-DesktopBuild 'npm.cmd' @('--prefix', 'ui', 'run', 'build')
    $desktopShellArguments = @('build', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--target-dir', 'src-tauri/target', '--features', 'custom-protocol')
    if ($Profile -eq 'release') { $desktopShellArguments += '--release' }
    Invoke-DesktopBuild 'cargo.exe' $desktopShellArguments
    foreach ($desktopBinary in @('audiorouter-shell.exe', 'audiorouter-cli.exe', 'audiorouter-plugin-worker.exe', 'audiorouter-vst3-worker.exe', 'audiorouter-plugin-worker-x86.exe', 'audiorouter-vst3-worker-x86.exe')) {
        $desktopPath = Join-Path $desktopOutput $desktopBinary
        if (-not (Test-Path -LiteralPath $desktopPath -PathType Leaf)) { throw "Incomplete desktop build: $desktopPath" }
        Write-Output "$desktopBinary SHA256 $((Get-FileHash -LiteralPath $desktopPath -Algorithm SHA256).Hash)"
    }
    Write-Output "Complete desktop build: $desktopOutput"
} finally {
    Pop-Location
}
