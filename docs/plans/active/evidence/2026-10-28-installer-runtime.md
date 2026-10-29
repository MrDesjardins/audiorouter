# Installer runtime qualification — 2026-10-28

On Windows 11, a linked clone of AR-DriverTest snapshot 02-test-signing-ready
was used. It is a test-configured VM, not clean-machine qualification. Under a
medium-integrity token with administrator membership disabled, published
0.0.23 installed and 0.0.24 upgraded with exit code 0. Both published CLIs
exited 0xC0000135 before opening a database. The guest lacked VCRUNTIME140.dll
and VCRUNTIME140_1.dll; PE import inspection found VCRUNTIME140.dll in the
published CLI and both DLLs in the shell. A running process alone is not shell
readiness evidence. On downgrade to 0.0.23 and silent uninstall, registration
was removed and a test marker survived. This does not yet prove real database
or recording retention or compatibility.

A separate `RUSTFLAGS=-C target-feature=+crt-static` release build of CLI,
plugin worker and shell imports no VCRUNTIME or MSVCP DLL. In the same VM,
the CLI ran `--json session list --database <isolated path>` with exit 0 and
created a real empty database. The shell started with an isolated database and
its one-shot native bridge probe fired. This is binary-level evidence, not a
packaged installer result or normal multi-request shell qualification.

The release preparation script now sets static CRT for its Rust builds and
rejects actual packaged shell/CLI/VST2/VST3-worker imports of the VC runtime.
The PE guard's bounded synthetic tests accept kernel32 and reject VCRUNTIME140;
manual qualification also rejected the published CLI and accepted the static
CLI and worker. Pending: build and execute the exact new installer, exercise
real database and recording retention across install/upgrade/uninstall, check
full native bridge and actual Apps-tab polling. Published 0.0.24 remains
unchanged. No physical audio, device defaults, signing or user state were changed.
