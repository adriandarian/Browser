#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "tools" / "py" / "ipc_schema.json"
OUT_PATH = ROOT / "docs" / "ipc-schema.md"


def render_section(title: str, rows: list[dict]) -> str:
    lines = [f"## {title}", "", "| Message | Fields |", "| --- | --- |"]
    for row in rows:
        fields = ", ".join(f"`{name}: {ty}`" for name, ty in row.get("fields", [])) or "(none)"
        lines.append(f"| `{row['name']}` | {fields} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    schema = json.loads(SCHEMA_PATH.read_text())

    content = [
        "# IPC Schema",
        "",
        f"Schema version: `{schema['version']}`",
        "",
        render_section("Browser -> Content", schema["browser_to_content"]),
        render_section("Content -> Browser", schema["content_to_browser"]),
    ]

    OUT_PATH.write_text("\n".join(content).rstrip() + "\n")
    print(f"[ipc_codegen] wrote {OUT_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
