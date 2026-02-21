# Tessera Monorepo (Rust + Zig + Python)

Foundational scaffolding for a desktop runtime that keeps strict boundaries:

- **Rust** owns runtime/orchestration and software rendering.
- **Zig** owns platform integration only (window, events, present).
- **Python** is tooling-only.
- The Rust↔Zig seam is a **stable C ABI** in `include/platform.h`.

## Repository layout

```text
.
├── crates/
│   ├── app/             # Rust binary: tessera
│   ├── engine/          # Tiny HTML tokenizer/DOM/layout/display-list pipeline
│   ├── engine_loop/     # Scheduler and frame timing
│   ├── ipc/             # IPC schema, codec, in-process transport
│   ├── platform_abi/    # Rust ABI mirror types (FFI-safe)
│   ├── renderer/        # Rust software renderer (patterns + display lists)
│   └── script_host/     # JS host trait + stub implementation
├── include/
│   └── platform.h       # ABI contract (source of truth)
├── tests/
│   ├── fixtures/        # Local HTML fixtures used by headless/golden checks
│   └── golden/          # Golden frame hashes
├── tools/py/
│   ├── run.py           # Build/run/test helper (run, test, golden)
│   └── ipc_codegen.py   # IPC schema doc generator
├── zig/platform/        # Zig-built platform library
├── Cargo.toml           # Cargo workspace root
└── justfile             # One-command workflows
```

## Prerequisites

- Rust stable (`rustup toolchain install stable`)
- Zig (0.12+ recommended)
- Python 3.11+
- `just` command runner (`cargo install just`)

## Build and run

### macOS

```bash
just build
just run
```

`tools/py/run.py` auto-selects:
- `aarch64-apple-darwin` on Apple Silicon
- `x86_64-apple-darwin` on Intel Macs

### Windows (MSVC)

Use the x64 Native Tools prompt (or equivalent environment with MSVC linker available):

```powershell
just build
just run
```

The runner targets `x86_64-pc-windows-msvc`.

## Development commands

```bash
just build   # cargo build --workspace
just run     # python3 tools/py/run.py run
just test    # python3 tools/py/run.py test
just golden -- --update  # refresh golden hashes
just fmt     # cargo fmt --all
```

## CLI commands

```bash
# Windowed runtime
cargo run -p tessera -- run --pattern gradient

# Headless RGBA export
cargo run -p tessera -- headless --input tests/fixtures/basic.html --out /tmp/frame.rgba

# Golden checks
cargo run -p tessera -- golden
cargo run -p tessera -- golden --update
```

## ABI design notes

- `include/platform.h` is the canonical contract.
- Keep structs plain-old-data, fixed-width, and append-only.
- Breaking changes require version bump (`PLATFORM_ABI_VERSION`).
- No bindgen in runtime path; Rust mirrors ABI manually in `platform_abi`.

## Troubleshooting

- **`zig: command not found`**: windowed/native builds on macOS/Windows use a stub fallback. Install Zig for native window integration.
- **Windows link errors (`link.exe` not found)**: open a developer shell with MSVC tools configured.
- **`platform_init_window returned false` on non-macOS/Windows hosts**: expected; Linux path is a stub for now.
- **No window appears on macOS**: ensure app is allowed to create windows (System Settings security prompts).
