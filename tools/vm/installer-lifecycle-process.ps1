function Invoke-LifecycleProcess {
    param(
        [string]$Executable,
        [string]$Name,
        [string[]]$Arguments,
        [string]$Evidence,
        [int]$TimeoutSeconds,
        [switch]$AllowNonzero,
        [switch]$SuppressOutput,
        [switch]$KillTree,
        [ref]$ExitStatus,
        [string]$OutputFilter
    )
    $prefix = "$Name-$([guid]::NewGuid().ToString('N'))"
    $stdout = Join-Path $Evidence "$prefix.stdout.txt"
    $stderr = Join-Path $Evidence "$prefix.stderr.txt"
    $record = [ordered]@{
        name = $Name
        attempt = $prefix
        startedUtc = [DateTime]::UtcNow.ToString('o')
        timeoutSeconds = $TimeoutSeconds
        processId = $null
        processStartedUtc = $null
        exitCode = $null
        cleanupExitCode = $null
        rootExited = $null
        status = 'start-failed'
    }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $process = $null
    try {
        # Windows argv: escape quotes and double backslashes before a closing quote.
        $quoted = ($Arguments | ForEach-Object {
            '"' + ($_ -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
        }) -join ' '
        $process = Start-Process -FilePath $Executable -ArgumentList $quoted -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr -ErrorAction Stop
        $null = $process.Handle
        $record.processId = $process.Id
        try { $record.processStartedUtc = $process.StartTime.ToUniversalTime().ToString('o') } catch {}
        $record.status = 'wait-failed'
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $record.status = 'timeout'
            try {
                if (-not $process.HasExited) {
                    if ($KillTree) {
                        $killer = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\taskkill.exe') -ArgumentList @('/PID', $process.Id, '/T', '/F') -WindowStyle Hidden -PassThru
                        try {
                            if ($killer.WaitForExit(5000)) { $record.cleanupExitCode = $killer.ExitCode }
                            else { $killer.Kill() }
                        } finally { $killer.Dispose() }
                    } else {
                        # A VBoxManage client can launch VM infrastructure. Leave the guest intact.
                        $process.Kill()
                    }
                }
                $record.rootExited = $process.WaitForExit(5000)
            } catch {
                $record.cleanupError = $_.Exception.GetType().Name
            }
            throw "$Name timed out after $TimeoutSeconds seconds. See $prefix in steps.jsonl for cleanup status."
        }
        $record.exitCode = $process.ExitCode
        if ($ExitStatus) { $ExitStatus.Value = $record.exitCode }
        $record.status = if ($record.exitCode -eq 0) { 'passed' } else { 'nonzero' }
        if ($record.exitCode -ne 0 -and -not $AllowNonzero) { throw "$Name failed with exit code $($record.exitCode)." }
        if (($record.exitCode -eq 0 -or $ExitStatus) -and -not $SuppressOutput) {
            $lines = @(Get-Content -LiteralPath $stdout)
            if ($OutputFilter) { $lines = @($lines -match $OutputFilter) }
            $lines
        }
    } catch {
        if ($record.status -eq 'start-failed') {
            $record.failureType = $_.Exception.GetType().Name
            throw "$Name could not start. See $prefix in steps.jsonl."
        }
        throw
    } finally {
        if ($process) { $process.Dispose() }
        $record.endedUtc = [DateTime]::UtcNow.ToString('o')
        $record.durationMs = $clock.ElapsedMilliseconds
        [IO.File]::AppendAllText((Join-Path $Evidence 'steps.jsonl'), (($record | ConvertTo-Json -Compress) + [Environment]::NewLine))
        if ($SuppressOutput -or $OutputFilter) {
            foreach ($file in @($stdout, $stderr)) {
                if (Test-Path -LiteralPath $file) { Remove-Item -LiteralPath $file }
            }
        }
    }
}
