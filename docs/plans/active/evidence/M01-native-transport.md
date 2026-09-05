# M01 native transport evidence

Date: 2026-09-05  
Host: `PATRICK5080`, Windows 11 x64 build `26200`  
Scope: native Rust named-pipe transport only

## Result

`crates/transport` provides a Windows-only local named-pipe boundary for the
existing 4-byte little-endian protocol framing. It validates the `\\.\pipe\`
namespace, bounds frames at the protocol maximum, handles partial reads and
writes, rejects remote clients at pipe creation, and flushes the response
before disconnecting.

The native test completed a request/response round trip through an actual
Windows named pipe. The test used only a temporary pipe name containing the
current process ID and closed/disconnected it at completion.

## Validation

- `cargo test -p audiorouter-transport`: 4 passed, including native round trip, control-plane dispatch, and peer process identity.
- `cargo test --workspace`: all workspace tests passed (37 unit tests; doc tests passed).
- `cargo fmt --all` and `git diff --check`: passed.

## Security and audio boundary

This is transport evidence, not authentication evidence. The prototype uses
the default pipe security descriptor and explicitly documents that production
must add a same-user ACL/authentication layer. No audio endpoint was opened,
no stream was started or read, no driver was installed, and no Windows audio
configuration was changed.

## Follow-up

The pipe integration now dispatches a framed `system.describe` request through
the real `ControlPlane` authority. The server also exposes the connected client
process ID and the native test verified it against the current process. This is
an identity primitive only; it is not yet same-user authentication. The next
transport task is to add an explicit authenticated/same-user security descriptor.
The
native C++/WDK audio and process-loopback gates remain blocked by the missing
Visual Studio/WDK toolchain.
