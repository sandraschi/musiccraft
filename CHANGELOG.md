# Changelog

All notable changes to MusicCraft will be documented in this file.

## [0.1.0] - 2026-11-15
### Added
- **L1 (`musiccraft-core`)**: Pure-Rust `#![forbid(unsafe_code)]` song project AST (`.mcraft`), 16-bit PCM WAV reader/writer, waveform peak extractor (`compute_waveform_peaks`), formant-based virtual vocal synthesizer (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`), 4-band stem splitter (`vocals`, `drums`, `bass`, `other`), offline & live mix renderer with per-track DSP FX (`Eq3Band`, `Compressor`, `Delay`, `Reverb`), optional metronome click (`render_project_with_metronome`), RMS/Peak/LUFS telemetry, DJCraft handoff packager, and universal 25-verb `Command::dispatch` bus (including `region.move`, `region.resize`, `region.duplicate`, `region.delete`, `note.add`, `note.remove`, `marker.add`, `track.add_fx`).
- **L2 (`musiccraft-mcp`)**: Unified `clap` CLI (`--json`, `commands`, `describe`, `run`, `project`, `track`, `stem`, `vocal`, `mix`, `export`, `--smoke`) and hand-rolled MCP `2024-11-05` stdio JSON-RPC server (`mcp`) with 25-tool `mcpb/manifest.json` parity.
- **L3 (`musiccraft-app`)**: Pure-Rust `eframe` / `egui 0.37` Pattern 2 DAW workstation with real-time `cpal` stereo transport (`Play`, `Pause`, `Stop`, `Rewind`, `Loop`, `Click` metronome, beat-ruler scrubbing, live playhead), interactive timeline (click-select, drag-move, right-edge drag-resize, snap-to-grid, `Ctrl+D`/`S`/`Del` shortcuts, markers `M`, loop bar, real WAV waveforms), interactive MIDI Piano Roll (`C3..C5`), per-track DSP FX Rack inspector, `.wav` Audio Track import (`rfd`), universal 8-menu bar (`switch_on_hover`), 5-item Help suite (`HELP.md`), `harsh_visuals()`, Tofu Guard (`glyph_allowlist`), and headless `--smoke` parity.
