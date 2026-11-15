# MusicCraft PRD (v0.1.0 — Implemented)

**Status:** implemented (`v0.1.0`), 2026-11-15.

## 1. Model

A song project: `{ tracks[], markers[], vocalLine?, arrangement, stems, mix, loopRange, metronome }`.
Tracks hold beat-indexed regions (audio with waveform peaks / MIDI notes) and a per-track DSP FX rack (`Eq3Band`, `Compressor`, `Delay`, `Reverb`); the vocal line holds phonemes + pitch curve;
stems are the exportable separations; the mix is the render recipe. The `.mcraft` project
file is the document.

## 2. Verbs (25-tool Unified Dispatch Bus)

- `project.new|open|save|info` — song project lifecycle
- `track.add|list|set_mix|add_fx` — arrangement & DSP FX rack building
- `region.add|split|move|resize|duplicate|delete` — REAPER-style timeline clip editing
- `note.add|remove` — interactive MIDI Piano Roll editing
- `marker.add` — named timeline section markers
- `modify.quantize|transpose` — rhythmic quantization & pitch transposition
- `stem.split` — 4-band stem separation (`vocals`, `drums`, `bass`, `other`)
- `vocal.synth` — formant virtual singer (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`)
- `mix.render` — offline mastered 16-bit stereo WAV render + LUFS telemetry
- `export.stems|song` — handoff package (`handoff.json`) for `djcraft`
- `system.status` & `mcp` — diagnostics and stdio JSON-RPC `2024-11-05` server over the same dispatch bus

## 3. Consolidation map

| Source | What MusicCraft takes | What stays put |
|---|---|---|
| `reaper-mcp` | arrangement concepts, edit verbs | the Reaper bridge itself |
| `stems-mcp` | separation verbs | standalone stem tooling |
| `songgeneration-mcp` | generation workflow knowledge | provider integrations |
| `kyutai-mcp` | vocal-synth pathfinding | speech/STT work |
| `djcraft` | **nothing** — sibling, not source | live decks, Camelot, performance |

## 4. Relation to djcraft (normative)

`djcraft` is the stage, MusicCraft is the studio. MusicCraft renders;
`djcraft` performs. Neither grows into the other. If a verb fits a live set,
it belongs in `djcraft`; if it fits a render queue, it belongs here.

## 5. Tech triage (open)

Bucket A/B per `RUST_ARTCRAFT_STANDARD.md` → Rust 3-tier likely; Python
FastMCP pilot acceptable. Audio latency (WASAPI), model licensing (vocal
models), and the local-vs-cloud vocal render split are the three decisions
that gate v1. No code until they are written down.

## 6. Phases

- **0 (done 2026-11-05):** name reserved, plan published, lineage declared.
- **1 (done 2026-11-15):** scope freeze & L1/L2/L3 Pure-Rust ArtCraft 3-tier containment workspace.
- **2 (done 2026-11-15):** 25-verb `Command::dispatch` bus, `musiccraft-mcp` CLI/MCP stdio server, and `mcpb/manifest.json` parity.
- **3 (done 2026-11-15):** real-time `cpal` stereo transport, interactive timeline editing (drag-move/resize, snap, markers, WAV waveforms), MIDI Piano Roll, per-track DSP FX rack, WAV import, and Craft Menu Bar / 5-item Help suite.

## 7. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Studio DAW Workstation):** Persistent multi-track timeline arrangement stage with mixer strips, piano roll, and vocal phoneme curve inspector. Top quick button row acts as accelerators into the `View` menu. To add a new synth module or audio effect:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Track / Synth toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New Project (`Ctrl+N`), Open Project (`Ctrl+O`), Open Recent ▸, Save (`Ctrl+S`), Save As... (`Ctrl+Shift+S`), Export Mix / Stems (`Ctrl+E`), Close (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy, Paste, Select All, Preferences (`Ctrl+,`).
  - `View`: Multi-Track Timeline, Mixer Console, Piano Roll / Vocal Curve Editor, Stem Separator View, Master Loudness HUD, Theme (`Dark`/`Light`).
  - `Insert`: Add Audio Track, Add MIDI Track, Insert Virtual Vocal Line, Add Effect Plugin.
  - `Modify`: Quantize Notes, Transpose Pitch, Split Audio Region, Normalize Clip.
  - `Commands`: Render Master Mix, Run Stem Separation, Synthesize Vocal Line, Export Handoff Package to DJCraft.
  - `Window`: Reset Layout, Arrangement Viewport, Mixer Drawer, Vocal Inspector.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About MusicCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
