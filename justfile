set shell := ["bash", "-cu"]

default: build

build:
    cargo build --workspace

run:
    python3 tools/py/run.py run

test:
    python3 tools/py/run.py test

fmt:
    cargo fmt --all
