# MusicCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(musiccraft-core)      (musiccraft-mcp)      (musiccraft-app)      (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Audio Arrangement & Vocal Synth Engine (`musiccraft-core`)
- [x] L1 `Command::dispatch` universal music production execution bus.
- [x] Song project schema (tracks, regions, arrangement, vocal lines, stems, mix).
- [x] Multi-track audio timeline and phoneme pitch curve alignment.
- [x] Stem separation driver and handoff packaging to DJCraft.
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`musiccraft-mcp`)
- [x] Unified CLI `musiccraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] Offline audio rendering pipeline and stem export.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`musiccraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation.
- [x] Multi-track timeline arrangement stage with mixer faders.
- [x] Piano roll and vocal synthesizer curve editor.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About MusicCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
