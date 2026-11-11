# MusicCraft PRD (draft 0 — pre-implementation)

**Status:** draft, 2026-11-05. Freezes before any code lands.

## 1. Model

A song project: `{ tracks[], vocalLine?, arrangement, stems, mix }`.
Tracks hold regions (audio/MIDI); the vocal line holds phonemes + pitch curve;
stems are the exportable separations; the mix is the render recipe. The project
file is the document (same pattern as `databasecraft`'s `.dbcraft` file).

## 2. Verbs (planned, not promised)

- `project new|open|info` — song project lifecycle
- `track add|list` — arrangement building
- `stem split --in mix.wav` — stems out (via `stems-mcp` knowledge first)
- `vocal synth --line phonemes.txt --voice OPERA` — the virtual singer
- `mix render --out mix.wav` — offline render + artifact
- `export stems|song` — handoff package for `djcraft`
- `mcp` — stdio server over the same dispatch bus

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
- **1:** scope freeze — answer §5, lock the verb list.
- **2:** pilot verbs (`project`, `track`, `export`) + `just ci` + fixtures.
- **3:** vocal synth path + first democraft recipe (`music/stems-sim`).

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
