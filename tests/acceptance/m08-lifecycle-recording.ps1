$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-recording.ps1')
$output = Join-Path $env:TEMP ('ar-wav-check-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $output | Out-Null
function Fixture([string]$Name, [int]$Frequency = 440, [int]$Rate = 48000, [int]$Seconds = 2, [int]$Channels = 1) {
    $path = Join-Path $output $Name
    $writer = [IO.BinaryWriter]::new([IO.File]::Create($path))
    try {
        $ascii = [Text.Encoding]::ASCII
        $writer.Write($ascii.GetBytes('RIFF'))
        $frames = $Rate * $Seconds
        $writer.Write([int](36 + 2 * $Channels * $frames))
        $writer.Write($ascii.GetBytes('WAVEfmt '))
        $writer.Write([int]16)
        $writer.Write([int16]1)
        $writer.Write([int16]$Channels)
        $writer.Write([int]$Rate)
        $writer.Write([int](2 * $Channels * $Rate))
        $writer.Write([int16](2 * $Channels))
        $writer.Write([int16]16)
        $writer.Write($ascii.GetBytes('data'))
        $writer.Write([int](2 * $Channels * $frames))
        for ($frame = 0; $frame -lt $frames; $frame++) {
            $sample = 4096 * [Math]::Sin(2 * [Math]::PI * $Frequency * $frame / $Rate)
            for ($channel = 0; $channel -lt $Channels; $channel++) { $writer.Write([int16]$sample) }
        }
    } finally { $writer.Dispose() }
    return $path
}
function Expect-Rejected([string]$Path, [string]$Pattern) {
    try { $null = Measure-QualificationWav $Path } catch {
        if ($_.Exception.Message -like $Pattern) { return }
        throw
    }
    throw "Invalid recording accepted: $Path"
}
$tone = Fixture 'tone.wav'
$metrics = Measure-QualificationWav $tone
if ($metrics.frames -ne 96000 -or $metrics.sampleRate -ne 48000 -or
    $metrics.sha256 -ne (Get-FileHash -LiteralPath $tone).Hash) { throw 'Incorrect fixture metrics' }
$stereoTone = Fixture 'stereo-tone.wav' 440 48000 2 2
$stereo = Measure-QualificationWav $stereoTone
if ($stereo.channels -ne 2 -or $stereo.frames -ne 96000 -or [Math]::Abs($stereo.frequencyHz - 440) -gt 8) { throw 'Stereo Recorder output was not measured per frame' }
Expect-Rejected (Fixture 'silence.wav' 0) '*non-silent*'
Expect-Rejected (Fixture 'stereo-silence.wav' 0 48000 2 2) '*non-silent*'
Expect-Rejected (Fixture 'stereo-short.wav' 440 48000 1 2) '*duration*'
Expect-Rejected (Fixture 'wrong-tone.wav' 660) '*frequency*'
Expect-Rejected (Fixture 'wrong-rate.wav' 440 44100) '*48 kHz*'
Expect-Rejected (Fixture 'short-tone.wav' 440 48000 1) '*duration*'
$broken = Join-Path $output 'truncated.wav'
$bytes = [IO.File]::ReadAllBytes($tone)
[IO.File]::WriteAllBytes($broken, $bytes[0..($bytes.Length - 2)])
Expect-Rejected $broken '*Invalid RIFF*'
$recorderText = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../../tools/vm/installer-lifecycle-recording.ps1'))
# WebView2 writes its debugging port below the EBWebView folder of the user data
# directory. Looking beside it left the installed-shell run waiting until timeout.
if ($recorderText -notmatch [regex]::Escape("'EBWebView\DevToolsActivePort'")) { throw 'Recorder must read DevToolsActivePort from EBWebView' }
Write-Output "Lifecycle WAV verifier passed. Evidence: $output"
