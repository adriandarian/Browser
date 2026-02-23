set shell := ["bash", "-cu"]

default: build

build:
    cargo build --workspace

run:
    python3 tools/py/run.py run

test:
    python3 tools/py/run.py test

golden *ARGS:
    python3 tools/py/run.py test --update {{ARGS}}

fmt:
    cargo fmt --all
