# 0.1.0 (2026-11-15)

Initial Pure-Rust ArtCraft 3-tier release:
- `musiccraft-core` (L1): `.mcraft` SongProject AST, 16-bit PCM WAV reader/writer, formant virtual vocal synthesizer, 4-band stem splitter, offline mix renderer with LUFS telemetry, DJCraft handoff packager, and universal `Command::dispatch` bus.
- `musiccraft-mcp` (L2): Unified `clap` CLI and MCP `2024-11-05` stdio JSON-RPC server.
- `musiccraft-app` (L3): Pure-Rust `eframe` / `egui 0.37` Pattern 2 DAW workstation with 8-menu Craft Menu Bar (`switch_on_hover`), 5-item Help suite, `harsh_visuals()`, Tofu Guard, and `--smoke`.
