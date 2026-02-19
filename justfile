set shell := ["bash", "-cu"]

default: build

build:
    cargo build --workspace

run:
    python3 tools/py/run.py

test:
    cargo test --workspace

fmt:
    cargo fmt --all
