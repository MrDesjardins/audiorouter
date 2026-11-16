$ErrorActionPreference = 'Stop'
$desktopFixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ('audiorouter-desktop-build-' + [guid]::NewGuid().ToString('N'))
$desktopFixtureTools = Join-Path $desktopFixtureRoot 'tools'
$null = New-Item -ItemType Directory -Path $desktopFixtureTools
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'build-desktop.ps1') -Destination $desktopFixtureTools
$global:desktopFixtureOmitWorker = $false
$global:desktopFixtureFailBuild = $false
$global:desktopFixtureCommands = @()

function Write-DesktopFixture([string]$Name) {
    $directory = Join-Path $desktopFixtureRoot 'src-tauri/target/release'
    $null = New-Item -ItemType Directory -Path $directory -Force
    [IO.File]::WriteAllText((Join-Path $directory $Name), 'fixture only')
}
function global:cargo.exe {
    $global:desktopFixtureCommands += "cargo $args"
    if ($global:desktopFixtureFailBuild) { $global:LASTEXITCODE = 9; return }
    if ($args -contains 'audiorouter-plugin-worker') {
        if ($args -contains 'i686-pc-windows-msvc') { Write-DesktopFixture 'audiorouter-plugin-worker-x86.exe' }
        else { Write-DesktopFixture 'audiorouter-plugin-worker.exe' }
    }
    else { Write-DesktopFixture 'audiorouter-shell.exe' }
    $global:LASTEXITCODE = 0
}
function global:npm.cmd { $global:desktopFixtureCommands += "npm $args"; $global:LASTEXITCODE = 0 }
function global:powershell.exe {
    $global:desktopFixtureCommands += "powershell $args"
    if ($args -contains 'tools/stage-mcp-cli.ps1') { Write-DesktopFixture 'audiorouter-cli.exe' }
    elseif (-not $global:desktopFixtureOmitWorker) {
        if ($args -contains 'x86') { Write-DesktopFixture 'audiorouter-vst3-worker-x86.exe' }
        else { Write-DesktopFixture 'audiorouter-vst3-worker.exe' }
    }
    $global:LASTEXITCODE = 0
}

$fixtureBuild = Join-Path $desktopFixtureTools 'build-desktop.ps1'
$global:desktopFixtureOmitWorker = $true
try {
    & $fixtureBuild
    throw 'Accepted an incomplete desktop build'
} catch {
    if ($_.Exception.Message -notlike '*Incomplete desktop build:*audiorouter-vst3-worker.exe*') { throw }
}
$global:desktopFixtureOmitWorker = $false
$result = @(& $fixtureBuild)
if (@($result | Where-Object { $_ -like '*SHA256*' }).Count -ne 6) { throw 'Not all six binaries were checked' }
if (-not ($global:desktopFixtureCommands -match 'npm --prefix ui run build')) { throw 'UI was not rebuilt' }
$global:desktopFixtureFailBuild = $true
try {
    & $fixtureBuild
    throw 'Accepted a failed worker build'
} catch {
    if ($_.Exception.Message -notlike '*cargo.exe failed with exit code 9*') { throw }
}
Write-Output 'PASS: missing VST3 rejected, complete package checked, failed build rejected.'
Write-Output "Disposable fixture (no executable code): $desktopFixtureRoot"
