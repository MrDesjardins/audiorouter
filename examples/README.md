# Examples

## Setups

Complete routing setups you can copy, with the Windows, application and
AudioRouter settings that go with them.

- [Gaming, Discord and clip recording](setups/gaming-discord-recording/README.md):
  three virtual cables, a processed mic, and a recorder (Outplayed) that
  captures your voice once.

## External integrations

Examples run independently of AudioRouter and use its existing local API.
Start the API in AudioRouter and supply its current bearer token privately.

- [Stats.cc / Siege](integrations/stats-cc-siege/README.md): a separate Node
  service that follows game phases and changes two Mixer input percentages.

Put future application integrations under `integrations/<application>/`, with
their own README, configuration, dependencies and tests. Put routing setups
under `setups/<name>/`. API recipes and device utilities can live in separate
folders when examples for them are added.
