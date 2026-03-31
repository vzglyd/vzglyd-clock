# Contributing to vzglyd-clock

Thanks for your interest.

`vzglyd-clock` is a reference slide for the vzglyd display engine. It is primarily useful as an example of how to author a vzglyd slide.

## Development

```bash
cargo build --target wasm32-wasip1 --release
cargo clippy --target wasm32-wasip1 -- -D warnings
cargo fmt
```

## Pull requests

- Keep changes focused on the clock slide itself
- The slide must continue to compile against the published `vzglyd-slide` ABI

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
