# MusicCraft Help

Pure-Rust ArtCraft studio workstation for DAW arrangement, virtual vocal synthesis, 4-band stem separation, offline mix rendering, and `djcraft` handoff packaging.

---

## 1. Get Started

1. **Launch the Studio:** Run `musiccraft.exe` (or `just app`). It opens with the deterministic *Vienna Aria (Demo)* project (`120.0 BPM`, `C major`, Camelot `8B`) pre-loaded so you can explore immediately.
2. **Create Your Own Song:** Click `File > New...` (`Ctrl+N`) or run:
   ```powershell
   musiccraft-mcp project new --name "Midnight Aria" --bpm 118 --key "A minor" --out song.mcraft --json
   ```
3. **Add Tracks & Regions:** Use `Insert > Add MIDI Track` or `Add Audio Track` (or the left **Studio Toolbox**), then `Insert > Add Region to Selected Track`.
4. **Synthesize a Vocal Line:** Open the **Vocal & Mix Inspector** on the right, enter vowel phonemes (`aa ee oo ah`), pick a voice (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`), and click **Synthesize Vocal**.
5. **Render & Export:** Click **Render Mix** in the top toolbar (or `Commands > Render Master Mix`) to write a mastered 16-bit stereo WAV with LUFS telemetry, or **Export to DJCraft** to generate `master_mix.wav`, 4 stems, and `handoff.json`.

---

## 2. Functions

- **Multi-Track Arrangement (`song.mcraft`):** JSON-backed song project holding `Audio`, `MIDI`, `Vocal`, and `Stem` tracks, beat-indexed regions, MIDI notes, volume/pan/mute/solo mixer strips, and undo/redo history.
- **Virtual Vocal Synthesizer:** Formant-filtered additive glottal synthesizer with vibrato LFO and pitch curve interpolation across five voice archetypes (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`).
- **4-Band Stem Separator:** Splits any 16-bit PCM WAV (or the current project mix) into `vocals.wav`, `drums.wav`, `bass.wav`, and `other.wav` with per-stem RMS and peak dB metrics.
- **Offline Mix Renderer:** Sums unmuted/soloed tracks with equal-power panning, soft-clip tanh limiting, and optional LUFS normalization (default `-14.0 LUFS`).
- **DJCraft Handoff Packager:** Exports `master_mix.wav`, all 4 stems, and `handoff.json` (BPM, musical key, Camelot code, duration) ready for live deck performance.
- **Unified Command Bus:** All 17 studio verbs execute through `musiccraft_core::Command::dispatch`, shared identically by the desktop UI, `musiccraft-mcp` CLI (`--json`), and MCP `2024-11-05` stdio server.

---

## 3. Usage (Window, CLI & MCP)

### Window Controls
- **Top Menu Bar:** `File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help` (with `switch_on_hover`).
- **Command Palette (`Ctrl+Shift+P` or `Ctrl+K`):** Fuzzy-search and run any of the 17 studio commands.
- **Center Tabs:** Switch between **Timeline**, **Piano Roll & Vocal**, and **Stems** from the top toolbar or `View` menu.
- **Mixer Console (Bottom):** Adjust track volume (`-48..+6 dB`), pan (`-1..+1`), Mute (`M`), and Solo (`S`).

### CLI & Batch Runner
```powershell
# List all 17 commands and inspect a schema
musiccraft-mcp commands --json
musiccraft-mcp describe vocal.synth --json

# Chain multiple edits in one headless batch
musiccraft-mcp run --cmd 'track.add={"name":"Strings","kind":"midi"}' --cmd 'modify.transpose={"semitones":2,"include_vocal":true}' --save updated.mcraft --json

# Render mix and split stems
musiccraft-mcp mix render --project updated.mcraft --out mix.wav --json
musiccraft-mcp stem split --in mix.wav --out-dir ./stems --json
```

### MCP Stdio Server
Start `musiccraft-mcp mcp` (or `musiccraft-mcp` with no arguments). `stdout` is 100% pure line-delimited JSON-RPC (`2024-11-05`); diagnostics go to `stderr`.

---

## 4. Tips & Tricks

- **Vowel Formants for Singing:** Separate vowel tokens with spaces or `|` (e.g. `aa ee oo ah oh`). `ee` boosts brightness around 2.25 kHz; `oo` and `oh` give warm rounded operatic phrasing; `aa` opens the chest resonance.
- **Camelot Key Continuity:** The top toolbar displays the Camelot wheel code next to your song key (e.g. `C major (Camelot 8B)`). Match adjacent Camelot numbers (`7B`, `8B`, `9B`, or relative minor `8A`) before exporting to `djcraft` for seamless harmonic transitions.
- **Gain Staging:** Keep individual track faders around `-6 dB` to `-1 dB` and let the master normalizer target `-14.0 LUFS` (streaming standard) so the soft-clip limiter stays transparent.
- **Non-Destructive Experiments:** Every action in the window pushes to a 32-step undo stack (`Ctrl+Z` / `Ctrl+Y`). In the CLI, pass `--project` without `--save` or omit `--project` to test against the built-in demo project safely.
- **Quick Keyboard Flow:** `Ctrl+K` runs any command, `F1` opens shortcuts, `Ctrl+=` / `Ctrl+-` zooms the timeline beat grid, and `Window > Reset Panel Layout` restores all docks.

---

## 5. Other Crafts (Ecosystem)

MusicCraft is part of the **Storytold Craft** pure-Rust workstation suite. Each app owns one domain and hands off clean artifacts rather than duplicating another app's UI:

| Craft App | Role | How It Connects to MusicCraft |
|---|---|---|
| **`djcraft`** | Live 4-deck DJ performance & Camelot mixing | **Stage partner.** MusicCraft renders `master_mix.wav`, 4 stems, and `handoff.json`; `djcraft` loads and performs them live. |
| **`speechcraft`** | Acoustic wake-word & neural voice command bus | Complementary voice studio: `speechcraft` handles spoken commands and voiceprints; MusicCraft handles pitched singing synthesis. |
| **`celcraft`** | 2D cel animation & lip-sync exposure sheets | Export rendered vocal or mix WAVs from MusicCraft to score and lip-sync animated scenes in `celcraft`. |
| **`smallvideocraft`** | Short-form video editing & audio-video assembly | Use MusicCraft's mastered `-14 LUFS` WAVs and stems as music beds and vocal tracks for video cuts. |
| **`gamecraft`** | 3D scene & game asset workstation | Generate looping stems and musical cues in MusicCraft for interactive game audio. |
| **`craft-deck`** | Unified launcher for all Craft apps | Installs and launches `musiccraft.exe` alongside the rest of your installed Craft suite. |
