param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'release',
    [switch]$Build
)

$ErrorActionPreference = 'Stop'
$mcpWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$mcpSource = Join-Path $mcpWorkspace "target\$Profile\audiorouter-cli.exe"
$mcpDestination = Join-Path $mcpWorkspace "src-tauri\target\$Profile\audiorouter-cli.exe"

if ($Build) {
    $mcpBuildArgs = @('build', '--locked', '--manifest-path', (Join-Path $mcpWorkspace 'Cargo.toml'), '-p', 'audiorouter-cli')
    if ($Profile -eq 'release') { $mcpBuildArgs += '--release' }
    & cargo.exe @mcpBuildArgs
    if ($LASTEXITCODE -ne 0) { throw 'MCP CLI build failed.' }
}
if (-not (Test-Path -LiteralPath $mcpSource -PathType Leaf)) {
    throw "Build the MCP CLI first, or use -Build: $mcpSource"
}
$mcpExpectedHash = (Get-FileHash -LiteralPath $mcpSource -Algorithm SHA256).Hash
if (Test-Path -LiteralPath $mcpDestination -PathType Leaf) {
    if ((Get-FileHash -LiteralPath $mcpDestination -Algorithm SHA256).Hash -eq $mcpExpectedHash) {
        Write-Output "MCP CLI already current: $mcpDestination ($mcpExpectedHash)"
        exit 0
    }
    $mcpBackup = "$mcpDestination.before-$(Get-Date -Format 'yyyyMMddHHmmss')"
    Copy-Item -LiteralPath $mcpDestination -Destination $mcpBackup
}
New-Item -ItemType Directory -Path (Split-Path -Parent $mcpDestination) -Force | Out-Null
try {
    Copy-Item -LiteralPath $mcpSource -Destination $mcpDestination -Force
} catch {
    throw 'Cannot stage the MCP CLI. Disconnect local MCP clients and the ChatGPT tunnel, then run this script again.'
}
if ((Get-FileHash -LiteralPath $mcpDestination -Algorithm SHA256).Hash -ne $mcpExpectedHash) {
    throw 'The staged MCP CLI does not match the built executable.'
}
Write-Output "MCP CLI staged: $mcpDestination ($mcpExpectedHash)"
