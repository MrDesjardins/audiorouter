# Optional Stream Deck extensions

The core Stream Deck plugin is implemented, packaged with AudioRouter and
confirmed working well by the owner. Its decisions, delivered controls and
implementation history are in the [archived core plan](../archived/2026-10-15-stream-deck-core.md).
Nothing below is required for the working integration or authorized merely by
appearing in this backlog.

| Extension | Current state | Reconsider when |
| --- | --- | --- |
| Stream Deck + dials and touch strips | Not implemented; core targets key actions | The owner requests dial support and a Stream Deck + is available for attended usability and load checks |
| Revocable, reduced-scope pairing grant | Core plugin uses the configured local HTTP API token | A concrete security scope and pairing flow is approved; coordinate with the hardware-control-panel plan |
| Faster change updates / lighter meter reads | Current plugin shares polling and uses existing API reads | Device measurements show polling latency, response size or CPU is a problem worth changing |
| Atomic scenes and multi-tool actions | Current calls change supported settings through existing validated APIs | A batch transaction and rollback contract is designed and approved; see [external app integration extensions](external-app-integrations.md) |
| Ready-made profiles and Elgato Marketplace listing | Release package is available; no Marketplace submission is part of the current delivery | The owner requests a profile or marketplace distribution and reviews the release/account requirements |

Device-specific latency, CPU and usability claims need measurements on the
target Stream Deck. Keep local API transport, privacy behavior, and the
working keys unchanged unless a selected extension requires an approved
contract change.
