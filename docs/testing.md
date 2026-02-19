# Testing

## Python entrypoint

Use `tools/py/run.py` for app run and golden tests:

```bash
python tools/py/run.py run [--release] [--pattern X]
python tools/py/run.py test [--release] [--update]
```

## Golden strategy

`python tools/py/run.py test` runs each case in `tests/golden/cases/*.json` by executing the Rust app in headless mode:

- `--headless-output <file.rgba>` writes raw RGBA8 bytes.
- Sidecar metadata is written next to it as JSON.
- The test harness computes SHA-256 and compares to each case's stored `sha256`.

On failure, the harness writes:

- `tests/golden/diffs/<case>.actual.rgba`
- `tests/golden/diffs/<case>.diff.json`

Use `--update` to refresh `sha256` values after intentional rendering changes.
