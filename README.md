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
│   ├── platform_abi/    # Rust ABI mirror types (FFI-safe)
│   └── renderer/        # Rust software test-pattern renderer
├── include/
│   └── platform.h       # ABI contract (source of truth)
├── tools/py/
│   └── run.py           # Build+run helper
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

Windows platform integration details:
- Zig builds `zig/platform/src/platform_windows.c` into a thin static `platform` library.
- The implementation exposes only the ABI entry points from `include/platform.h` and keeps Win32 internals private.
- `platform_present_frame` presents Rust-provided `RGBA8` pixels through `StretchDIBits` for correctness-first output.
- Event mapping currently includes:
  - `WM_CLOSE` / `WM_DESTROY` / `WM_QUIT` -> `PLATFORM_EVENT_QUIT`
  - `WM_KEYDOWN` / `WM_KEYUP` (`Esc`) -> `PLATFORM_EVENT_KEY_DOWN` / `PLATFORM_EVENT_KEY_UP`
  - `WM_SIZE` -> `PLATFORM_EVENT_RESIZE`

Linking notes for Cargo + MSVC:
- `crates/app/build.rs` drives `zig build` with `x86_64-windows-msvc`.
- The build script links `platform` and Win32 system libraries (`user32`, `gdi32`) into the Rust binary.

Minimal CI hints (optional, not required yet):
- Validate Windows on `windows-latest` with:
  - `cargo build --workspace --target x86_64-pc-windows-msvc`
  - `cargo test --workspace --target x86_64-pc-windows-msvc`
- Keep Zig available in CI `PATH` before Cargo runs so `crates/app/build.rs` can invoke it.

## Development commands

```bash
just build   # cargo build --workspace
just run     # python3 tools/py/run.py
just test    # cargo test --workspace
just fmt     # cargo fmt --all
```

## ABI design notes

- `include/platform.h` is the canonical contract.
- Keep structs plain-old-data, fixed-width, and append-only.
- Breaking changes require version bump (`PLATFORM_ABI_VERSION`).
- No bindgen in runtime path; Rust mirrors ABI manually in `platform_abi`.

## Troubleshooting

- **`zig: command not found`**: Install Zig and ensure it is on `PATH`.
- **Windows link errors (`link.exe` not found)**: open a developer shell with MSVC tools configured.
- **`platform_init_window returned false` on non-macOS/Windows hosts**: expected; Linux path is a stub for now.
- **No window appears on macOS**: ensure app is allowed to create windows (System Settings security prompts).
