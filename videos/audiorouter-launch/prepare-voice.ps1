$ErrorActionPreference = 'Stop'
$env:HYPERFRAMES_PYTHON = Join-Path $PSScriptRoot '.venv/Scripts/python.exe'
Set-Location $PSScriptRoot
$lines = @(
  'Your sound. Your setup. Your way.',
  'Move around your mix. See every stage. Make it yours.',
  'Shape your tone. Choose a starting point. Make it yours.',
  'Keep levels steady. Let your voice lead. Catch the peaks before they take over.',
  'Clean up noise. Watch the spectrum. Let your game step back when your voice comes in.',
  'Bring your favorite plugins. Switch sources. Record the moment.',
  'Gaming PC to streaming PC. Send your mix across your home network.',
  'Tune it live. See the timing of every tool, and your whole audio journey.',
  'Make it part of your workflow. Connect your controls through the API. Or let an AI assistant help configure your sound with MCP.',
  'See your sound. Shape it. Send it anywhere. AudioRouter.'
)
for ($i = 1; $i -lt $lines.Count; $i++) {
  $outputPath = 'assets/audio/voice-{0:D2}.wav' -f ($i + 1)
  & npx.cmd --yes hyperframes@0.8.106 tts $lines[$i] --voice af_heart --output $outputPath
  if ($LASTEXITCODE -ne 0) { throw "Voice line $($i + 1) failed" }
}
