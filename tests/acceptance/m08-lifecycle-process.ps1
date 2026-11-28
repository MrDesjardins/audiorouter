$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-process.ps1')
$evidence = Join-Path $env:TEMP ("ar-process-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $evidence | Out-Null
$exe = Join-Path $PSHOME 'powershell.exe'
$fixture = Join-Path $evidence 'argument fixture.ps1'
Set-Content -LiteralPath $fixture -Value 'param([string]$Value) [Console]::WriteLine($Value)'
$expected = 'C:\path with spaces\'
$actual = Invoke-LifecycleProcess $exe 'repeat' @('-NoProfile','-File',$fixture,$expected) $evidence 10
if ($actual -cne $expected) { throw "Argument round trip failed: $actual" }
$null = Invoke-LifecycleProcess $exe 'repeat' @('-NoProfile','-Command','exit 23') $evidence 10 -AllowNonzero
function Expect-Failure([scriptblock]$Action, [string]$Pattern) {
    try { & $Action } catch {
        if ($_.Exception.Message -like $Pattern) { return }
        throw
    }
    throw "Expected failure matching $Pattern"
}
Expect-Failure { Invoke-LifecycleProcess $exe 'nonzero' @('-NoProfile','-Command','exit 23') $evidence 10 } '*exit code 23*'
Expect-Failure { Invoke-LifecycleProcess (Join-Path $evidence 'missing.exe') 'missing' @('arg') $evidence 10 } '*could not start*'
$unrelated = Start-Process $exe -ArgumentList '-NoProfile -Command Start-Sleep -Seconds 60' -WindowStyle Hidden -PassThru
try {
    $marker = Join-Path $evidence 'child.txt'
    $hang = Join-Path $evidence 'hang.ps1'
    Set-Content -LiteralPath $hang -Value @'
param([string]$Marker)
$child = Start-Process (Join-Path $PSHOME 'powershell.exe') -ArgumentList '-NoProfile -Command Start-Sleep -Seconds 60' -WindowStyle Hidden -PassThru
[IO.File]::WriteAllText($Marker, "$($child.Id) $($child.StartTime.ToUniversalTime().Ticks)")
Start-Sleep -Seconds 60
'@
    Expect-Failure { Invoke-LifecycleProcess $exe 'hang' @('-NoProfile','-File',$hang,$marker) $evidence 4 -KillTree } '*timed out*'
    $childId, $ticks = (Get-Content -LiteralPath $marker) -split ' '
    $child = Get-Process -Id $childId -ErrorAction SilentlyContinue
    # taskkill /T /F returns once termination is requested; the descendant can
    # still be listed for a moment. It must be gone within 5 s, not instantly.
    if ($child -and $child.StartTime.ToUniversalTime().Ticks -eq [long]$ticks -and
        -not $child.WaitForExit(5000)) {
        try { $child.Kill() } catch [InvalidOperationException] { }
        throw 'Owned descendant survived timeout'
    }
    if ($unrelated.HasExited) { throw 'Unrelated process was terminated' }
    $null = Invoke-LifecycleProcess $exe 'recovery' @('-NoProfile','-Command','exit 0') $evidence 10
    $rows = @(Get-Content (Join-Path $evidence 'steps.jsonl') | ForEach-Object { $_ | ConvertFrom-Json })
    if (($rows.status -join ',') -ne 'passed,nonzero,nonzero,start-failed,timeout,passed' -or
        $rows[4].cleanupExitCode -ne 0 -or -not $rows[4].rootExited -or
        $rows[0].attempt -eq $rows[1].attempt) { throw 'Unexpected process evidence' }
    $filtered = @(Invoke-LifecycleProcess $exe 'filtered' @('-NoProfile','-Command',"'safe'; 'private fixture'") $evidence 10 -OutputFilter '^safe$')
    if (($filtered -join ',') -cne 'safe' -or @(Get-ChildItem $evidence -Filter 'filtered-*.txt').Count -ne 0) {
        throw 'Filtered output retained data outside the allowlist'
    }
    Expect-Failure { Invoke-LifecycleProcess $exe 'single-timeout' @('-NoProfile','-Command','Start-Sleep -Seconds 60') $evidence 2 } '*timed out*'
    $last = Get-Content (Join-Path $evidence 'steps.jsonl') -Tail 1 | ConvertFrom-Json
    if (-not $last.rootExited -or $unrelated.HasExited) { throw 'Single-process cleanup failed' }
} finally {
    if (-not $unrelated.HasExited) { $unrelated.Kill() }
    $unrelated.Dispose()
}
Write-Output "Lifecycle process regressions passed. Evidence: $evidence"
