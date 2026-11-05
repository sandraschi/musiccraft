# MusicCraft (reserved — initial plan)

**Status:** `[ALPHA]` name reservation with an initial plan. No implementation
yet. This repo exists so the fleet's music-studio umbrella has a home and the
name isn't taken by someone else first. What is written below is intent, not
documentation of running code. Implementation follows once the scope in
`docs/PRD.md` is frozen.

## Why this repo

Music is a wide field, and the fleet already touches it from five directions
without a roof over them: DAW arrangement (`reaper-mcp`), stem work
(`stems-mcp`), AI song generation (`songgeneration-mcp`, lessons from
`suno-mcp`), vocal synthesis (`kyutai-mcp` vocal work), and live performance
(`mixx-dj-mcp`, `virtualdj-mcp`, `djcraft`). MusicCraft is that roof: the
studio where songs are arranged, sung, stemmed and mixed.

## Scope (initial plan)

- **DAW arrangement** — tracks, regions, stems in/out. Sources: `reaper-mcp`
  (arrangement knowledge), `stems-mcp` (separation).
- **Vocal synthesis / virtual opera singer** — a singable vocal line rendered
  locally when the GPU allows, via user-token cloud when it doesn't. Sources:
  the fleet's vocal-synth work (`songgeneration-mcp`, `kyutai-mcp`).
- **Studio-to-stage link** — MusicCraft renders the stems and songs;
  `djcraft` performs them. The boundary is deliberate and permanent (below).

## Non-goals (explicit)

- **Not a second djcraft.** Live 4-deck mixing, the Camelot wheel, stem
  performance and the performance swarm stay in `djcraft`, which already owns
  them. MusicCraft never grows decks.
- **No cloud lock-in.** Local-first: runs fully local with no cloud tokens when
  the GPU is up to it; falls back to cloud models (a few minutes on your own
  Claude, Google, or OpenAI pro account) when it isn't.
- **No notation software in v1.** Score engraving is deferred, not denied.

## Harness (when built)

Implements the Storytold craft-app harness like the rest of the fleet: one
binary that is both the human CLI and the agent MCP server (stdio),
headless-first, `run.json` + report artifacts. Tech triage pending (likely a
Rust ArtCraft 3-tier workspace in the shape of `djcraft`/`speechcraft`; a
Python FastMCP pilot is acceptable the way the six workstation crafts did it).

## Lineage — the Storytold craft-app pattern

Storytold ArtCraft ([storytold/artcraft](https://github.com/storytold/artcraft) — "the IDE for artists") was the starting point. The 16 craft apps in the Storytold stable each ship one binary that is both the human CLI and the agent MCP server (stdio), headless-first, with `run.json` + report artifacts. That modus operandi and harness is what this repo will implement; the code here is independent, the pattern is shared.
