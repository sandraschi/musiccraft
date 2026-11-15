# MusicCraft tasks (pwsh-friendly: `just gates`)
set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

default:
    @just --list

check:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo test --workspace -- --test-threads=1

fmt-check:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo fmt --all --check

gates: test check fmt-check

build:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo build --release --workspace

app:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo run --release -p musiccraft-app

smoke:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo run -p musiccraft-mcp -- --smoke
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo run -p musiccraft-app -- --smoke

mcp:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo run -p musiccraft-mcp -- mcp

cli *ARGS:
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo run -p musiccraft-mcp -- {{ARGS}}

# Build and stage dist/ (plain exes, installer, launcher zip, SHA256SUMS). Nothing is published.
release-dry:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1

# Same, then tag, push the tag and upload to GitHub (clean, pushed trunk required).
release:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1 -Publish
