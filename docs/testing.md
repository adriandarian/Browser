# Testing and Goldens

This repository uses deterministic headless rendering for baseline checks.

## Commands

- `python3 tools/py/run.py run` runs the windowed app.
- `python3 tools/py/run.py test` runs `cargo test --workspace` and golden checks.
- `python3 tools/py/run.py golden` validates hashes in `tests/golden/`.
- `python3 tools/py/run.py golden --update` refreshes expected hashes.

## Golden workflow

1. Add or update HTML fixtures in `tests/fixtures/`.
2. Run `python3 tools/py/run.py golden --update` when changes are intentional.
3. Commit the updated `tests/golden/*.hash` files.

## Headless frame export

The Rust binary supports explicit frame export:

```bash
cargo run -p tessera -- headless \
  --input tests/fixtures/basic.html \
  --width 960 --height 540 --frame 0 \
  --out /tmp/basic.rgba
```

The output is raw RGBA8 pixel data (`width * height * 4` bytes).
