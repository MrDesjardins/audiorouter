param(
    [ValidateSet('build', 'dev')]
    [string]$Task = 'build'
)

# Tauri runs hooks through cmd.exe. A file avoids nested -Command quoting
# turning the PowerShell program into a string that only prints its contents.
$ErrorActionPreference = 'Stop'
$uiWorkspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& npm.cmd --prefix (Join-Path $uiWorkspace 'ui') run $Task
exit $LASTEXITCODE
