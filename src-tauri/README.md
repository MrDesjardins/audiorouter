# AudioRouter native shell

This is the Tauri 2 desktop shell for the existing AudioRouter UI. It is a
standalone Cargo workspace so portable workspace builds do not acquire desktop
runtime dependencies.

The `rpc_request` command uses the existing authenticated Windows named-pipe
transport. Set `AUDIOROUTER_CONTROL_PIPE` only when connecting to a deliberately
started AudioRouter control service; the default is
`\\\\.\\pipe\\audiorouter-control`. The shell does not install a driver,
register plugins, change Windows audio endpoints, or start an unconfigured
service.

Compile without launching or packaging:

```text
cargo check --manifest-path src-tauri/Cargo.toml
```

