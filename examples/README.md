# External integration examples

Examples run independently of AudioRouter and use its existing local API.
Start the API in AudioRouter and supply its current bearer token privately.

- [Stats.cc / Siege](integrations/stats-cc-siege/README.md): a separate Node
  service that follows game phases and changes two Mixer input percentages.

Put future application integrations under `integrations/<application>/`, with
their own README, configuration, dependencies and tests. API recipes and device
utilities can live in separate folders when examples for them are added.
