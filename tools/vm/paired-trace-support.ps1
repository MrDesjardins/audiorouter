# Definitions only. No trace, process, device or shared-folder operation on import.
function Write-PairedTraceJson {
    param([string] $Path, $Value)
    # Publish a complete file: the peer must never see partially written JSON.
    # Each protocol message has a unique path; never overwrite another run.
    $temporary = $Path + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
    try {
        $Value | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $temporary -Encoding UTF8
        [IO.File]::Move($temporary, $Path)
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary }
    }
}

function Read-PairedTraceJson {
    param([string] $Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    if ((Get-Item -LiteralPath $Path).Length -gt 16KB) { throw 'Protocol message exceeds 16 KB.' }
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Wait-PairedTraceMessage {
    param([string] $Path, [string] $Run, [int] $TimeoutSeconds = 30, [string] $AbortPath,
        [ValidateRange(1,100)][int] $PollMilliseconds = 10, [scriptblock] $Guard)
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $lastGuard = -1.0
    do {
        if ($Guard -and $watch.Elapsed.TotalSeconds - $lastGuard -ge 1) { & $Guard; $lastGuard = $watch.Elapsed.TotalSeconds }
        if ($AbortPath) {
            $abort = Read-PairedTraceJson $AbortPath
            if ($abort -and $abort.Run -ceq $Run) { throw "Peer failed: $($abort.Error)" }
        }
        $value = Read-PairedTraceJson $Path
        if ($value) {
            if ($value.Run -cne $Run) { throw 'Protocol run identity mismatch.' }
            return $value
        }
        Start-Sleep -Milliseconds $PollMilliseconds
    } while ($watch.Elapsed.TotalSeconds -lt $TimeoutSeconds)
    throw "Timed out waiting for $(Split-Path -Leaf $Path). Preserve both evidence folders."
}

function Assert-PairedTraceBundle {
    param([string] $Bundle)
    $entries = @(Get-Content -LiteralPath (Join-Path $Bundle 'MANIFEST.txt') | Where-Object { $_ -match '^[a-fA-F0-9]{64}  ' })
    if ($entries.Count -eq 0) { throw 'Bundle manifest has no hashes.' }
    foreach ($entry in $entries) {
        $path = [IO.Path]::GetFullPath((Join-Path $Bundle $entry.Substring(66)))
        if (-not $path.StartsWith($Bundle + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escapes bundle.' }
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $entry.Substring(0,64)) {
            throw "Bundle hash mismatch: $($entry.Substring(66))"
        }
    }
}

function Assert-PairedTraceAdministrator {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Open PowerShell as Administrator on this machine.' }
}

function Save-PairedTraceClockSamples {
    param([string] $Directory, [string] $Run, [string] $Stage, [string] $Evidence, [scriptblock] $Guard)
    $samples = @()
    for ($index = 0; $index -lt 8; $index++) {
        $stem = "$Stage-$index"
        $startTicks = [DateTime]::UtcNow.Ticks
        $startQpc = [Diagnostics.Stopwatch]::GetTimestamp()
        Write-PairedTraceJson (Join-Path $Directory "$stem-request.json") @{ Run = $Run }
        $reply = Wait-PairedTraceMessage (Join-Path $Directory "$stem-reply.json") $Run 10 (Join-Path $Directory 'guest-failed.json') 1 $Guard
        $endQpc = [Diagnostics.Stopwatch]::GetTimestamp()
        $endTicks = [DateTime]::UtcNow.Ticks
        if ($endTicks -lt $startTicks -or [Math]::Abs(($endTicks - $startTicks) / 10000.0 - ($endQpc - $startQpc) * 1000.0 / [Diagnostics.Stopwatch]::Frequency) -gt 5) {
            throw 'Host wall clock changed during a clock bracket.'
        }
        # Guest reply timestamp lies somewhere inside this host round trip.
        # Keep the whole interval; never assume half the latency was one-way.
        $samples += [pscustomobject]@{
            Stage = $Stage; Index = $index; HostStartUtcTicks = $startTicks; HostEndUtcTicks = $endTicks
            HostStartQpc = $startQpc; HostEndQpc = $endQpc; HostQpcFrequency = [Diagnostics.Stopwatch]::Frequency
            GuestUtcTicks = $reply.UtcTicks; GuestUtcAfterQpcTicks = $reply.UtcAfterQpcTicks
            GuestQpc = $reply.Qpc; GuestQpcFrequency = $reply.QpcFrequency
            GuestMinusHostLowerMs = ([long]$reply.UtcTicks - $endTicks) / 10000.0
            GuestMinusHostUpperMs = ([long]$reply.UtcTicks - $startTicks) / 10000.0
            RoundTripMs = ($endTicks - $startTicks) / 10000.0
        }
    }
    $samples | Export-Csv -LiteralPath (Join-Path $Evidence "$Stage-clock.csv") -NoTypeInformation -Encoding UTF8
}

function Reply-PairedTraceClockSamples {
    param([string] $Directory, [string] $Run, [string] $Stage)
    for ($index = 0; $index -lt 8; $index++) {
        $stem = "$Stage-$index"
        $null = Wait-PairedTraceMessage (Join-Path $Directory "$stem-request.json") $Run 20 (Join-Path $Directory 'host-failed.json') 1
        $utcTicks = [DateTime]::UtcNow.Ticks
        $qpc = [Diagnostics.Stopwatch]::GetTimestamp()
        $afterTicks = [DateTime]::UtcNow.Ticks
        Write-PairedTraceJson (Join-Path $Directory "$stem-reply.json") @{
            Run = $Run; UtcTicks = $utcTicks; UtcAfterQpcTicks = $afterTicks
            Qpc = $qpc; QpcFrequency = [Diagnostics.Stopwatch]::Frequency
        }
    }
}

function Assert-PairedTraceBudget {
    param([string] $Evidence)
    $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($Evidence))
    $bytes = (Get-ChildItem -LiteralPath $Evidence -Recurse -File | Measure-Object -Property Length -Sum).Sum
    # A sampled stop threshold, not an exact file-size cap. Save can add bytes.
    if ($drive.AvailableFreeSpace -lt 2GB -or $bytes -gt 1GB) { throw 'Trace storage threshold reached; stopping the owned recorder.' }
}
