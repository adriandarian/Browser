# Testing and Goldens

The golden test flow is deterministic and windowless. It renders fixtures through
the Rust binary in headless mode and compares outputs against committed goldens.

## Commands

- `python3 tools/py/run.py run [--release] [--pattern X]`
- `python3 tools/py/run.py test [--release] [--update]`

`test` runs `cargo test --workspace` first, then golden checks.

## Golden Folder Structure

`tests/golden/` contains:

- `expected/`: committed baselines (`*.rgba`, `*.json`, `*.sha256`)
- `actual/`: latest rendered outputs from test runs (not committed)
- `diff/`: failure reports (`*.diff.txt`, not committed)

## Update Workflow

1. Edit fixtures in `tests/fixtures/*.html`.
2. Refresh goldens intentionally:

```bash
python3 tools/py/run.py test --update
```

3. Commit files from `tests/golden/expected/`.

## Rust Headless Export Flags

The renderer exports raw RGBA plus metadata:

```bash
cargo run -p browser -- headless \
  --input tests/fixtures/basic.html \
  --width 960 --height 540 --frame 0 \
  --out-rgba /tmp/basic.rgba \
  --out-meta /tmp/basic.json
```

- `--out-rgba` (or `--out`) writes raw RGBA8 (`width * height * 4` bytes).
- `--out-meta` writes JSON metadata (`format`, `width`, `height`, `stride_bytes`, `frame`).

## Report Output

Golden test output is minimal and machine-friendly:

- `PASS <fixture>`
- `FAIL <fixture> diff=<path>`
- `UPDATED <fixture> (<hash-prefix>)`
- `golden summary: pass=N fail=M`
