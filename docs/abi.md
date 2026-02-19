# Platform ABI rules

This document defines the C ABI contract between Rust and Zig (`include/platform.h`).

## Versioning

- `PLATFORM_ABI_VERSION` is a monotonically increasing `uint32_t`.
- `platform_get_abi_version()` returns the runtime ABI version exported by the platform library.
- Callers must compare their compile-time ABI version against runtime before calling other APIs.

## Type and layout rules

- Use only fixed-width integer types (`uint8_t`, `uint32_t`, ...).
- Do **not** expose C `bool` at the boundary; use `uint8_t` (`PLATFORM_TRUE` / `PLATFORM_FALSE`).
- Structs use native C layout and alignment (`repr(C)` on Rust mirrors).
- Do not use `#pragma pack` or `__attribute__((packed))` for ABI structs.

## ABI bump rules

### Breaking changes (must bump `PLATFORM_ABI_VERSION`)

- Removing or renaming an exported symbol.
- Changing function signatures or return types.
- Reordering, removing, or changing type/meaning of existing struct fields.
- Changing enum discriminant values used across the ABI.
- Changing struct packing/alignment requirements.

### Non-breaking changes (ABI version can stay the same)

- Adding new exported symbols.
- Adding new enum values that older consumers can ignore.
- Appending optional trailing struct fields **when** guarded by `struct_size` checks.

## Safe struct extension pattern

Every cross-ABI struct includes `struct_size` as the first field:

1. Caller sets `struct_size = sizeof(struct_type)`.
2. Callee validates the minimum size required for fields it reads.
3. Newly-added fields are appended at the tail and are read only when `struct_size` is large enough.

This keeps older and newer components interoperable across minor evolution.

## Migration plan for future input features

To add mouse input, scroll, text input, and IME without breaking ABI:

1. Keep `platform_event` stable and append fields only at the end.
2. Add new event kinds:
   - mouse move/button
   - scroll
   - text input (UTF-8 chunk)
   - IME composition lifecycle
3. For payload-heavy events, add a new `platform_event_v2` struct and a new function
   (for example `platform_poll_event_v2`) while preserving old symbols.
4. Use capability probing via new non-breaking symbols (for example
   `platform_get_capabilities`) before enabling optional features.
5. Deprecate but do not remove old APIs until a planned major ABI bump.
