# Changelog

All notable changes to MusicCraft will be documented in this file.

## [0.1.0] - 2026-11-15
### Added
- **L1 (`musiccraft-core`)**: Pure-Rust `#![forbid(unsafe_code)]` song project AST (`.mcraft`), 16-bit PCM WAV reader/writer, formant-based virtual vocal synthesizer (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`), 4-band stem splitter (`vocals`, `drums`, `bass`, `other`), offline mix renderer with RMS/Peak/LUFS telemetry, DJCraft handoff packager, and universal `Command::dispatch` bus.
- **L2 (`musiccraft-mcp`)**: Unified `clap` CLI (`--json`, `commands`, `describe`, `run`, `project`, `track`, `stem`, `vocal`, `mix`, `export`, `--smoke`) and hand-rolled MCP `2024-11-05` stdio JSON-RPC server (`mcp`).
- **L3 (`musiccraft-app`)**: Pure-Rust `eframe` / `egui 0.37` Pattern 2 DAW workstation with universal 8-menu bar (`switch_on_hover`), mandatory 5-item Help suite, `harsh_visuals()`, multi-track timeline, piano roll / vocal pitch curve editor, mixer console, stem separator, Tofu Guard (`glyph_allowlist`), and headless `--smoke` parity.
