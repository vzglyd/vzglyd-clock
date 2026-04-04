# VRX-64-clock

A clock slide for the [vzglyd](https://github.com/vzglyd/vzglyd) display engine. Displays local time alongside a 3D rotating globe rendered via a custom WGSL shader.

## Usage

Build the slide:

```bash
./build.sh
```

This produces `clock.vzglyd` — a packaged slide ready to be placed in your vzglyd slides directory.

## Tracing

This slide now emits top-level guest spans for `vzglyd_configure`, `vzglyd_init`, and
`vzglyd_update`. Use the normal host workflows to capture them:

```bash
cargo run --manifest-path /home/rodgerbenham/.openclaw/workspace/VRX-64-native/Cargo.toml -- \
  --trace \
  --scene /home/rodgerbenham/.openclaw/workspace/lume-clock/clock.vzglyd
```

For web profiling, serve the bundle through the canonical `VRX-64-web/web-preview/view.html`
player and use `Start Trace` / `Stop & Download`.

## Requirements

- Rust stable with `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) or the build script dependencies listed in `build.sh`

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
