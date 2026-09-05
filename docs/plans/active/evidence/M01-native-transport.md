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

- `cargo test -p audiorouter-transport`: 8 passed, including preservation of all batch responses.
- `cargo test --workspace`: all workspace tests passed (43 unit tests; doc tests passed).
- `cargo fmt --all` and `git diff --check`: passed.

## Security and audio boundary

The transport creates the pipe with an owner-only SDDL security descriptor,
rejects a client whose process-token user SID differs from the server, and
performs these checks before reading the request. This is a baseline local
authentication boundary, not a replacement for deployment-account review and
method-level authorization. No audio endpoint was opened,
no stream was started or read, no driver was installed, and no Windows audio
configuration was changed.

## Follow-up

The pipe integration now dispatches a framed `system.describe` request through
the real `ControlPlane` authority. The server also exposes the connected client
process ID and the native test verified it against the current process. The
bounded `serve_connections` loop, transient startup/rotation retries,
per-request `ClientGrant` dispatch, and `serve_control_connections` entry point
now cover the basic connection lifecycle and authorization path. Optional
response handling and `send_oneway` allow JSON-RPC notifications to complete
without a response or client block. Batch response frames are combined in order
and can be read individually by the multi-frame client helper.
Mixed batches preserve successful reads and authorization denials in request
order; native coverage verifies the second response remains `-32001`.
The
native C++/WDK audio and process-loopback gates remain blocked by the missing
Visual Studio/WDK toolchain.
