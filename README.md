# screen-dump

`screen-dump` is a macOS command-line utility that reports the visible window
layout, window bounds, owning applications, display bounds, z-order, and
window-level Accessibility metadata.

It is intentionally a Cargo binary and currently targets the macOS toolchain
used to build it.

## Permissions

Accessibility permission is required for every metadata snapshot. Add the
`screen-dump` executable to:

```text
System Settings > Privacy & Security > Accessibility
```

The optional `--screenshot` mode also requires Screen Recording permission.
ScreenCaptureKit may identify the executable by its path, so grant permission
to the exact binary you invoke.

## Usage

```text
cargo run -- --help
cargo run --
cargo run -- --json | jq '.windows[] | {id: .window_id, app: .owner.name, bounds}'
cargo run -- --app Safari
cargo run -- --pid 1234
cargo run -- --window-id 456
cargo run -- -v
cargo run -- --screenshot /tmp/screen.png
```

The default report is ordered front-to-back. `*` marks the focused window and
`>` marks windows belonging to the frontmost application. `-v` adds practical
AX and Core Graphics diagnostics; `-vv` adds matching errors and raw details.

The JSON output is currently experimental. Window bounds use Quartz global
display coordinates. A single-display snapshot uses a
compact `display` field; multi-display snapshots use `displays` and include a
`display_id` on each window.

This project is licensed under the MIT License.

Default filtering keeps user-facing, on-screen, nonzero windows and hides
desktop shell/system utility windows. Use `--all`, `--include-hidden`, or
`--include-system` to broaden the result.

## Development

```text
cargo fmt
cargo test
cargo check
cargo run -- --json
```
