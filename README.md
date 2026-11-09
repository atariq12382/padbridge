# padbridge

Padbridge is a Rust workspace for bridging gamepad input to configurable controls.

## Workspace crates

- `padbridge-core`: device types and input mapping configuration.
- `padbridge-daemon`: background service entry point.
- `padbridge-cli`: command-line controls for the daemon.

## Development

Run the workspace checks with:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```