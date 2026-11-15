# MusicCraft Command & MCP Tool Reference

All 25 commands execute through the single `musiccraft_core::Command::dispatch` bus. CLI, MCP (`musiccraft-mcp mcp`), and the desktop workstation (`musiccraft`) share the exact same implementation.

| Command ID | MCP Tool Name | Menu | Shortcut | Description |
|---|---|---|---|---|
| `system.status` | `musiccraft_status` | Help | — | Inspect MusicCraft engine version, sample rate, and capabilities |
| `project.new` | `musiccraft_project_new` | File | `Ctrl+N` | Create a new `.mcraft` song project with name, BPM, and musical key |
| `project.open` | `musiccraft_project_open` | File | `Ctrl+O` | Load an existing `.mcraft` song project from disk |
| `project.save` | `musiccraft_project_save` | File | `Ctrl+S` | Save the active song project to a `.mcraft` file |
| `project.info` | `musiccraft_project_info` | File | — | Inspect song project metadata, track count, vocal line, and stems |
| `track.add` | `musiccraft_track_add` | Insert | — | Add an audio, midi, vocal, or stem track to the song arrangement |
| `track.list` | `musiccraft_track_list` | View | — | List all tracks and regions in the song project |
| `track.set_mix` | `musiccraft_track_set_mix` | Modify | — | Update track volume (dB), pan (-1..1), mute, or solo state |
| `region.add` | `musiccraft_region_add` | Insert | — | Insert a new MIDI or audio region onto a track |
| `region.split` | `musiccraft_region_split` | Modify | `S` | Split a timeline region into two regions at `split_beat` |
| `region.move` | `musiccraft_region_move` | Modify | — | Move a timeline region to a new start beat |
| `region.resize` | `musiccraft_region_resize` | Modify | — | Resize a timeline region length in beats |
| `region.duplicate` | `musiccraft_region_duplicate` | Modify | `Ctrl+D` | Duplicate a timeline region immediately after its end beat |
| `region.delete` | `musiccraft_region_delete` | Modify | `Del` | Delete a region from the arrangement |
| `note.add` | `musiccraft_note_add` | Insert | — | Add a MIDI note to a region in the piano roll |
| `note.remove` | `musiccraft_note_remove` | Modify | — | Remove a MIDI note from a region by pitch and start beat |
| `marker.add` | `musiccraft_marker_add` | Insert | `M` | Add a named timeline marker at a beat position |
| `track.add_fx` | `musiccraft_track_add_fx` | Insert | — | Append a DSP effect (`eq`, `compressor`, `delay`, `reverb`) to a track FX rack |
| `modify.quantize` | `musiccraft_modify_quantize` | Modify | — | Snap MIDI note start beats to the nearest rhythmic grid |
| `modify.transpose` | `musiccraft_modify_transpose` | Modify | — | Transpose MIDI notes and optional vocal line by semitones (-48..48) |
| `vocal.synth` | `musiccraft_vocal_synth` | Commands | — | Synthesize a virtual singer vocal line (`OPERA`, `SOPRANO`, `TENOR`, `CHOIR`, `SYNTH`) |
| `stem.split` | `musiccraft_stem_split` | Commands | — | Separate a WAV mix into 4 stems (`vocals`, `drums`, `bass`, `other`) |
| `mix.render` | `musiccraft_mix_render` | Commands | `Ctrl+E` | Offline-render the song arrangement into a mastered 16-bit stereo WAV |
| `export.stems` | `musiccraft_export_stems` | Commands | — | Export master mix, 4 stems, and `handoff.json` manifest for `djcraft` |
| `export.song` | `musiccraft_export_song` | File | — | Export `.mcraft` project, master mix, stems, and handoff manifest |
