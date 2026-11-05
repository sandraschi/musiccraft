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
