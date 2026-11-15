# 0.1.0 (2026-11-15)

Pure-Rust ArtCraft 3-tier DAW Studio release:
- `musiccraft-core` (L1): `.mcraft` SongProject AST, 16-bit PCM WAV reader/writer, waveform peak extractor, formant virtual vocal synthesizer, 4-band stem splitter, mix renderer with per-track DSP FX (`Eq3Band`, `Compressor`, `Delay`, `Reverb`) & metronome click, LUFS telemetry, DJCraft handoff packager, and 25-verb `Command::dispatch` bus.
- `musiccraft-mcp` (L2): Unified `clap` CLI and MCP `2024-11-05` stdio JSON-RPC server (25 tools).
- `musiccraft-app` (L3): Pure-Rust `eframe` / `egui 0.37` Pattern 2 DAW workstation with real-time `cpal` transport (`Play`/`Pause`/`Stop`/`Rewind`/`Loop`/`Click`), interactive timeline (drag-move/resize, snap-to-grid, markers, WAV waveforms), interactive MIDI Piano Roll, Track FX Rack, `.wav` import, 8-menu Craft Menu Bar (`switch_on_hover`), 5-item Help suite, `harsh_visuals()`, Tofu Guard, and `--smoke`.
