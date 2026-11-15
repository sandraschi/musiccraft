# MusicCraft

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

**Pure-Rust ArtCraft DAW arrangement, virtual vocal synthesis, 4-band stem separation, and offline mix rendering studio.**

## Install

Tell Cursor, Claude, Antigravity, or OpenCode: **Install this.**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

## Architecture (3-Tier ArtCraft Containment Harness)

- **`crates/musiccraft-core` (L1):** SongProject AST (`.mcraft`), 16-bit PCM WAV reader/writer (hostile-input fenced), waveform peak extractor, formant-based virtual vocal synthesizer (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`), 4-band stem separator (`vocals`, `drums`, `bass`, `other`), offline & live mix renderer with per-track DSP FX (`Eq3Band`, `Compressor`, `Delay`, `Reverb`) and RMS/Peak/LUFS telemetry, DJCraft handoff packager (`handoff.json`), and universal 25-verb `Command::dispatch` bus. `#![forbid(unsafe_code)]` + zero-panic clippy lints.
- **`crates/musiccraft-mcp` (L2):** Unified `clap` CLI (`--json`, `commands`, `describe`, `run`, `project`, `track`, `stem`, `vocal`, `mix`, `export`, `--smoke`) and hand-rolled MCP `2024-11-05` stdio JSON-RPC server (`mcp`) with 25-tool `mcpb/manifest.json` parity.
- **`crates/musiccraft-app` (L3):** Pure-Rust `eframe` / `egui 0.37` Pattern 2 DAW workstation calling `musiccraft_core::dispatch_on_project` directly in-process. Features real-time `cpal` audio transport (`Play`, `Pause`, `Stop`, `Rewind`, `Loop`, `Click` metronome, Beat Ruler scrubbing), interactive timeline editing (drag-move/resize, snap-to-grid, markers, WAV waveforms), interactive MIDI Piano Roll, per-track DSP FX Rack, `.wav` audio import, the 8-menu Craft Menu Bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`, the 5-item Help suite, `harsh_visuals()`, Tofu Guard (`glyph_allowlist`), and headless `--smoke` parity.

## Quick Start

```powershell
# Run all merge gates (tests, clippy -D warnings, fmt --check)
just gates

# Run headless smoke verification on both CLI/MCP and desktop app
just smoke

# Launch the desktop workstation
just app

# Run the MCP stdio server
just mcp

# CLI examples
cargo run -p musiccraft-mcp -- commands --json
cargo run -p musiccraft-mcp -- vocal synth --line "aa ee oo ah" --voice OPERA --out vocal.wav --json
cargo run -p musiccraft-mcp -- mix render --out mix.wav --json
cargo run -p musiccraft-mcp -- export stems --out-dir ./handoff --json
```

## Studio-to-Stage Boundary (Normative)

`djcraft` is the stage; `musiccraft` is the studio. MusicCraft arranges, synthesizes vocals, splits stems, renders mixes, and exports a `handoff.json` package for `djcraft`. Live 4-deck mixing and Camelot performance stay in `djcraft`.
