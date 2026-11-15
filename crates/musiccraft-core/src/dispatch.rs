use crate::audio::{
    export_djcraft_handoff, render_project_mix, render_vocal_mono, split_stems, write_wav_16bit,
};
use crate::error::{validate_non_empty_bounded, MusicCraftError, Result};
use crate::project::{
    build_pitch_curve, parse_phoneme_line, MidiNote, Region, SongProject, TrackKind, VocalLine,
    VocalVoice, DEFAULT_SAMPLE_RATE,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

/// Machine-readable command metadata for `commands --json`, `describe`, MCP `tools/list`, and UI Command Palette.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub id: &'static str,
    pub mcp_name: &'static str,
    pub label: &'static str,
    pub menu: &'static str,
    pub shortcut: Option<&'static str>,
    pub description: &'static str,
    pub params_schema: Value,
}

/// Universal ArtCraft Command Bus for MusicCraft.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "params", rename_all = "snake_case")]
pub enum Command {
    SystemStatus,
    ProjectNew {
        name: String,
        bpm: f64,
        key: String,
        out_path: Option<String>,
    },
    ProjectOpen {
        path: String,
    },
    ProjectSave {
        path: String,
    },
    ProjectInfo {
        path: Option<String>,
    },
    TrackAdd {
        project_path: Option<String>,
        name: String,
        kind: String,
    },
    TrackList {
        project_path: Option<String>,
    },
    TrackSetMix {
        project_path: Option<String>,
        track_id: String,
        volume_db: Option<f32>,
        pan: Option<f32>,
        muted: Option<bool>,
        solo: Option<bool>,
    },
    RegionAdd {
        project_path: Option<String>,
        track_id: String,
        name: String,
        start_beat: f64,
        length_beats: f64,
    },
    RegionSplit {
        project_path: Option<String>,
        region_id: String,
        split_beat: f64,
    },
    ModifyQuantize {
        project_path: Option<String>,
        track_id: Option<String>,
        grid_beats: f64,
    },
    ModifyTranspose {
        project_path: Option<String>,
        semitones: i32,
        include_vocal: bool,
    },
    VocalSynth {
        project_path: Option<String>,
        line: String,
        voice: String,
        bpm: Option<f64>,
        base_midi: Option<u8>,
        out_wav: Option<String>,
    },
    StemSplit {
        project_path: Option<String>,
        in_wav: Option<String>,
        out_dir: String,
    },
    MixRender {
        project_path: Option<String>,
        out_wav: String,
    },
    ExportStems {
        project_path: Option<String>,
        out_dir: String,
    },
    ExportSong {
        project_path: Option<String>,
        out_dir: String,
    },
}

/// Canonical command catalog shared across CLI, MCP, and GUI Command Palette.
pub fn command_catalog() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "system.status",
            mcp_name: "musiccraft_status",
            label: "System Status",
            menu: "Help",
            shortcut: None,
            description: "Inspect MusicCraft engine version, sample rate, and capabilities.",
            params_schema: json!({ "type": "object", "properties": {} }),
        },
        CommandSpec {
            id: "project.new",
            mcp_name: "musiccraft_project_new",
            label: "New Song Project",
            menu: "File",
            shortcut: Some("Ctrl+N"),
            description: "Create a new .mcraft song project with name, BPM, and musical key.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Song title" },
                    "bpm": { "type": "number", "description": "Tempo in BPM (20..300)" },
                    "key": { "type": "string", "description": "Musical key (e.g. C major, A minor)" },
                    "out_path": { "type": "string", "description": "Optional .mcraft path to save" }
                },
                "required": ["name"]
            }),
        },
        CommandSpec {
            id: "project.open",
            mcp_name: "musiccraft_project_open",
            label: "Open Song Project",
            menu: "File",
            shortcut: Some("Ctrl+O"),
            description: "Load an existing .mcraft song project from disk.",
            params_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        },
        CommandSpec {
            id: "project.save",
            mcp_name: "musiccraft_project_save",
            label: "Save Song Project",
            menu: "File",
            shortcut: Some("Ctrl+S"),
            description: "Save the active song project to a .mcraft file.",
            params_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        },
        CommandSpec {
            id: "project.info",
            mcp_name: "musiccraft_project_info",
            label: "Project Info",
            menu: "File",
            shortcut: None,
            description: "Inspect song project metadata, track count, vocal line, and stems.",
            params_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } }
            }),
        },
        CommandSpec {
            id: "track.add",
            mcp_name: "musiccraft_track_add",
            label: "Add Track",
            menu: "Insert",
            shortcut: None,
            description: "Add an audio, midi, vocal, or stem track to the song arrangement.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "name": { "type": "string" },
                    "kind": { "type": "string", "enum": ["audio", "midi", "vocal", "stem"] }
                },
                "required": ["name", "kind"]
            }),
        },
        CommandSpec {
            id: "track.list",
            mcp_name: "musiccraft_track_list",
            label: "List Tracks",
            menu: "View",
            shortcut: None,
            description: "List all tracks and regions in the song project.",
            params_schema: json!({
                "type": "object",
                "properties": { "project_path": { "type": "string" } }
            }),
        },
        CommandSpec {
            id: "track.set_mix",
            mcp_name: "musiccraft_track_set_mix",
            label: "Set Track Mix",
            menu: "Modify",
            shortcut: None,
            description: "Update track volume (dB), pan (-1..1), mute, or solo state.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "track_id": { "type": "string" },
                    "volume_db": { "type": "number" },
                    "pan": { "type": "number" },
                    "muted": { "type": "boolean" },
                    "solo": { "type": "boolean" }
                },
                "required": ["track_id"]
            }),
        },
        CommandSpec {
            id: "region.add",
            mcp_name: "musiccraft_region_add",
            label: "Add Region",
            menu: "Insert",
            shortcut: None,
            description: "Insert a new MIDI or audio region onto a track.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "track_id": { "type": "string" },
                    "name": { "type": "string" },
                    "start_beat": { "type": "number" },
                    "length_beats": { "type": "number" }
                },
                "required": ["track_id", "name"]
            }),
        },
        CommandSpec {
            id: "region.split",
            mcp_name: "musiccraft_region_split",
            label: "Split Region",
            menu: "Modify",
            shortcut: None,
            description: "Split a timeline region into two regions at split_beat.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "region_id": { "type": "string" },
                    "split_beat": { "type": "number" }
                },
                "required": ["region_id", "split_beat"]
            }),
        },
        CommandSpec {
            id: "modify.quantize",
            mcp_name: "musiccraft_modify_quantize",
            label: "Quantize Notes",
            menu: "Modify",
            shortcut: None,
            description: "Snap MIDI note start beats to the nearest rhythmic grid.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "track_id": { "type": "string" },
                    "grid_beats": { "type": "number" }
                }
            }),
        },
        CommandSpec {
            id: "modify.transpose",
            mcp_name: "musiccraft_modify_transpose",
            label: "Transpose Pitch",
            menu: "Modify",
            shortcut: None,
            description: "Transpose MIDI notes and optional vocal line by semitones (-48..48).",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "semitones": { "type": "integer" },
                    "include_vocal": { "type": "boolean" }
                },
                "required": ["semitones"]
            }),
        },
        CommandSpec {
            id: "vocal.synth",
            mcp_name: "musiccraft_vocal_synth",
            label: "Synthesize Vocal Line",
            menu: "Commands",
            shortcut: None,
            description: "Synthesize a virtual singer vocal line from phonemes (OPERA, SOPRANO, TENOR, CHOIR, SYNTH).",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "line": { "type": "string", "description": "Space-separated phonemes, e.g. 'aa ee oo ah'" },
                    "voice": { "type": "string", "enum": ["OPERA", "SOPRANO", "TENOR", "CHOIR", "SYNTH"] },
                    "bpm": { "type": "number" },
                    "base_midi": { "type": "integer" },
                    "out_wav": { "type": "string" }
                },
                "required": ["line"]
            }),
        },
        CommandSpec {
            id: "stem.split",
            mcp_name: "musiccraft_stem_split",
            label: "Run Stem Separation",
            menu: "Commands",
            shortcut: None,
            description: "Separate a WAV mix (or project demo mix) into 4 stems: vocals, drums, bass, other.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "in_wav": { "type": "string" },
                    "out_dir": { "type": "string" }
                },
                "required": ["out_dir"]
            }),
        },
        CommandSpec {
            id: "mix.render",
            mcp_name: "musiccraft_mix_render",
            label: "Render Master Mix",
            menu: "Commands",
            shortcut: Some("Ctrl+E"),
            description: "Offline-render the song arrangement into a mastered 16-bit stereo WAV with LUFS telemetry.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "out_wav": { "type": "string" }
                },
                "required": ["out_wav"]
            }),
        },
        CommandSpec {
            id: "export.stems",
            mcp_name: "musiccraft_export_stems",
            label: "Export Handoff to DJCraft",
            menu: "Commands",
            shortcut: None,
            description: "Export master mix, 4 stems, and handoff.json manifest for djcraft.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "out_dir": { "type": "string" }
                },
                "required": ["out_dir"]
            }),
        },
        CommandSpec {
            id: "export.song",
            mcp_name: "musiccraft_export_song",
            label: "Export Song Bundle",
            menu: "File",
            shortcut: None,
            description: "Export .mcraft project, master mix, stems, and handoff manifest into out_dir.",
            params_schema: json!({
                "type": "object",
                "properties": {
                    "project_path": { "type": "string" },
                    "out_dir": { "type": "string" }
                },
                "required": ["out_dir"]
            }),
        },
    ]
}

impl Command {
    /// Parse a command by canonical ID (e.g. `project.new`) or MCP name (`musiccraft_project_new`).
    pub fn from_name_and_params(name: &str, params: &Value) -> Result<Self> {
        let str_opt = |k: &str| {
            params
                .get(k)
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
        };
        let str_req = |k: &str| {
            params
                .get(k)
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .ok_or_else(|| MusicCraftError::InvalidInput(format!("missing string param '{k}'")))
        };

        match name.trim() {
            "system.status" | "musiccraft_status" | "system_status" => Ok(Self::SystemStatus),
            "project.new" | "musiccraft_project_new" | "project_new" => Ok(Self::ProjectNew {
                name: str_opt("name").unwrap_or_else(|| "Untitled Song".to_string()),
                bpm: params.get("bpm").and_then(|v| v.as_f64()).unwrap_or(120.0),
                key: str_opt("key").unwrap_or_else(|| "C major".to_string()),
                out_path: str_opt("out_path"),
            }),
            "project.open" | "musiccraft_project_open" | "project_open" => Ok(Self::ProjectOpen {
                path: str_req("path")?,
            }),
            "project.save" | "musiccraft_project_save" | "project_save" => Ok(Self::ProjectSave {
                path: str_req("path")?,
            }),
            "project.info" | "musiccraft_project_info" | "project_info" => Ok(Self::ProjectInfo {
                path: str_opt("path"),
            }),
            "track.add" | "musiccraft_track_add" | "track_add" => Ok(Self::TrackAdd {
                project_path: str_opt("project_path"),
                name: str_req("name")?,
                kind: str_opt("kind").unwrap_or_else(|| "midi".to_string()),
            }),
            "track.list" | "musiccraft_track_list" | "track_list" => Ok(Self::TrackList {
                project_path: str_opt("project_path"),
            }),
            "track.set_mix" | "musiccraft_track_set_mix" | "track_set_mix" => {
                Ok(Self::TrackSetMix {
                    project_path: str_opt("project_path"),
                    track_id: str_req("track_id")?,
                    volume_db: params
                        .get("volume_db")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32),
                    pan: params.get("pan").and_then(|v| v.as_f64()).map(|v| v as f32),
                    muted: params.get("muted").and_then(|v| v.as_bool()),
                    solo: params.get("solo").and_then(|v| v.as_bool()),
                })
            }
            "region.add" | "musiccraft_region_add" | "region_add" => Ok(Self::RegionAdd {
                project_path: str_opt("project_path"),
                track_id: str_req("track_id")?,
                name: str_req("name")?,
                start_beat: params
                    .get("start_beat")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                length_beats: params
                    .get("length_beats")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(4.0),
            }),
            "region.split" | "musiccraft_region_split" | "region_split" => Ok(Self::RegionSplit {
                project_path: str_opt("project_path"),
                region_id: str_req("region_id")?,
                split_beat: params
                    .get("split_beat")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        MusicCraftError::InvalidInput("missing number param 'split_beat'".into())
                    })?,
            }),
            "modify.quantize" | "musiccraft_modify_quantize" | "modify_quantize" => {
                Ok(Self::ModifyQuantize {
                    project_path: str_opt("project_path"),
                    track_id: str_opt("track_id"),
                    grid_beats: params
                        .get("grid_beats")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5),
                })
            }
            "modify.transpose" | "musiccraft_modify_transpose" | "modify_transpose" => {
                Ok(Self::ModifyTranspose {
                    project_path: str_opt("project_path"),
                    semitones: params
                        .get("semitones")
                        .and_then(|v| v.as_i64())
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or_else(|| {
                            MusicCraftError::InvalidInput(
                                "missing integer param 'semitones'".into(),
                            )
                        })?,
                    include_vocal: params
                        .get("include_vocal")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                })
            }
            "vocal.synth" | "musiccraft_vocal_synth" | "vocal_synth" => Ok(Self::VocalSynth {
                project_path: str_opt("project_path"),
                line: str_req("line")?,
                voice: str_opt("voice").unwrap_or_else(|| "OPERA".to_string()),
                bpm: params.get("bpm").and_then(|v| v.as_f64()),
                base_midi: params
                    .get("base_midi")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u8::try_from(v).ok()),
                out_wav: str_opt("out_wav"),
            }),
            "stem.split" | "musiccraft_stem_split" | "stem_split" => Ok(Self::StemSplit {
                project_path: str_opt("project_path"),
                in_wav: str_opt("in_wav"),
                out_dir: str_req("out_dir")?,
            }),
            "mix.render" | "musiccraft_mix_render" | "mix_render" => Ok(Self::MixRender {
                project_path: str_opt("project_path"),
                out_wav: str_req("out_wav")?,
            }),
            "export.stems" | "musiccraft_export_stems" | "export_stems" => Ok(Self::ExportStems {
                project_path: str_opt("project_path"),
                out_dir: str_req("out_dir")?,
            }),
            "export.song" | "musiccraft_export_song" | "export_song" => Ok(Self::ExportSong {
                project_path: str_opt("project_path"),
                out_dir: str_req("out_dir")?,
            }),
            other => Err(MusicCraftError::UnknownCommand(other.to_string())),
        }
    }
}

fn load_or_demo(project_path: Option<&str>) -> Result<SongProject> {
    match project_path {
        Some(p) => {
            validate_non_empty_bounded("project_path", p, 512)?;
            SongProject::load_from_file(&PathBuf::from(p))
        }
        None => Ok(SongProject::new_demo()),
    }
}

fn maybe_save(project: &SongProject, project_path: Option<&str>) -> Result<()> {
    if let Some(p) = project_path {
        project.save_to_file(&PathBuf::from(p))?;
    }
    Ok(())
}

/// Execute a `Command` in-process against an active `SongProject` (used by L3 GUI and batch `run`).
pub fn dispatch_on_project(project: &mut SongProject, cmd: Command) -> Value {
    match dispatch_on_project_inner(project, cmd) {
        Ok(v) => v,
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

fn dispatch_on_project_inner(project: &mut SongProject, cmd: Command) -> Result<Value> {
    match cmd {
        Command::SystemStatus => Ok(json!({
            "ok": true,
            "app": "musiccraft",
            "version": env!("CARGO_PKG_VERSION"),
            "sample_rate": DEFAULT_SAMPLE_RATE,
            "unsafe_code": "forbidden",
            "command_count": command_catalog().len(),
            "normative_boundary": "studio_only_djcraft_performs"
        })),
        Command::ProjectNew {
            name,
            bpm,
            key,
            out_path,
        } => {
            let new_proj = SongProject::new(&name, bpm, &key)?;
            if let Some(p) = &out_path {
                validate_non_empty_bounded("out_path", p, 512)?;
                new_proj.save_to_file(&PathBuf::from(p))?;
            }
            *project = new_proj;
            Ok(json!({
                "ok": true,
                "project": project,
                "saved_to": out_path
            }))
        }
        Command::ProjectOpen { path } => {
            validate_non_empty_bounded("path", &path, 512)?;
            let loaded = SongProject::load_from_file(&PathBuf::from(&path))?;
            *project = loaded;
            Ok(json!({
                "ok": true,
                "name": project.name,
                "bpm": project.bpm,
                "key": project.key,
                "tracks": project.tracks.len(),
                "path": path
            }))
        }
        Command::ProjectSave { path } => {
            validate_non_empty_bounded("path", &path, 512)?;
            project.save_to_file(&PathBuf::from(&path))?;
            Ok(json!({ "ok": true, "saved_to": path }))
        }
        Command::ProjectInfo { path } => {
            if let Some(p) = path {
                validate_non_empty_bounded("path", &p, 512)?;
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            Ok(json!({
                "ok": true,
                "name": project.name,
                "version": project.version,
                "bpm": project.bpm,
                "key": project.key,
                "sample_rate": project.sample_rate,
                "total_beats": project.total_beats(),
                "total_seconds": project.total_seconds(),
                "track_count": project.tracks.len(),
                "has_vocal_line": project.vocal_line.is_some(),
                "stem_count": project.stems.len(),
                "mix": project.mix
            }))
        }
        Command::TrackAdd {
            project_path,
            name,
            kind,
        } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let tk = TrackKind::from_str_loose(&kind)?;
            let trk = project.add_track(&name, tk)?.clone();
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "track": trk }))
        }
        Command::TrackList { project_path } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            Ok(json!({ "ok": true, "tracks": project.tracks }))
        }
        Command::TrackSetMix {
            project_path,
            track_id,
            volume_db,
            pan,
            muted,
            solo,
        } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let trk = project
                .set_track_mix(&track_id, volume_db, pan, muted, solo)?
                .clone();
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "track": trk }))
        }
        Command::RegionAdd {
            project_path,
            track_id,
            name,
            start_beat,
            length_beats,
        } => {
            validate_non_empty_bounded("region name", &name, 128)?;
            if !start_beat.is_finite()
                || start_beat < 0.0
                || !length_beats.is_finite()
                || length_beats <= 0.0
            {
                return Err(MusicCraftError::InvalidInput(
                    "start_beat >= 0 and length_beats > 0 required".into(),
                ));
            }
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let reg_id = format!(
                "reg-{}",
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("0000")
            );
            let region = Region {
                id: reg_id,
                name,
                start_beat,
                length_beats,
                notes: vec![MidiNote {
                    pitch: 60,
                    velocity: 96,
                    start_beat: 0.0,
                    duration_beats: (length_beats * 0.5).max(0.5),
                }],
                audio_ref: None,
                gain_db: 0.0,
            };
            project.add_region(&track_id, region.clone())?;
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "region": region }))
        }
        Command::RegionSplit {
            project_path,
            region_id,
            split_beat,
        } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            project.split_region(&region_id, split_beat)?;
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "split_region_id": region_id, "at_beat": split_beat }))
        }
        Command::ModifyQuantize {
            project_path,
            track_id,
            grid_beats,
        } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let quantized = project.quantize_notes(track_id.as_deref(), grid_beats)?;
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "notes_quantized": quantized, "grid_beats": grid_beats }))
        }
        Command::ModifyTranspose {
            project_path,
            semitones,
            include_vocal,
        } => {
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let count = project.transpose_pitch(semitones, include_vocal)?;
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "events_transposed": count, "semitones": semitones }))
        }
        Command::VocalSynth {
            project_path,
            line,
            voice,
            bpm,
            base_midi,
            out_wav,
        } => {
            validate_non_empty_bounded("phoneme line", &line, 4096)?;
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let v = VocalVoice::from_str_loose(&voice)?;
            let use_bpm = bpm.unwrap_or(project.bpm).clamp(20.0, 300.0);
            let phonemes = parse_phoneme_line(&line, base_midi.unwrap_or(69), 1.0);
            let pitch_curve = build_pitch_curve(&phonemes);
            let vocal_line = VocalLine {
                voice: v,
                phonemes: phonemes.clone(),
                pitch_curve,
            };
            project.vocal_line = Some(vocal_line.clone());

            let rendered_path = if let Some(wav_path) = out_wav {
                validate_non_empty_bounded("out_wav", &wav_path, 512)?;
                let total_beats = phonemes
                    .iter()
                    .map(|p| p.start_beat + p.duration_beats)
                    .fold(1.0_f64, f64::max);
                let total_secs = (total_beats * 60.0) / use_bpm;
                let total_frames =
                    ((total_secs * f64::from(project.sample_rate)).ceil() as usize).max(1);
                let mono =
                    render_vocal_mono(&vocal_line, use_bpm, project.sample_rate, total_frames);
                write_wav_16bit(&PathBuf::from(&wav_path), project.sample_rate, 1, &mono)?;
                Some(wav_path)
            } else {
                None
            };

            maybe_save(project, project_path.as_deref())?;
            Ok(json!({
                "ok": true,
                "voice": v.as_str(),
                "phoneme_count": phonemes.len(),
                "bpm": use_bpm,
                "out_wav": rendered_path
            }))
        }
        Command::StemSplit {
            project_path,
            in_wav,
            out_dir,
        } => {
            validate_non_empty_bounded("out_dir", &out_dir, 512)?;
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let in_path = in_wav.as_ref().map(PathBuf::from);
            let stems = split_stems(in_path.as_deref(), &PathBuf::from(&out_dir), project)?;
            project.stems = stems.clone();
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "stems": stems }))
        }
        Command::MixRender {
            project_path,
            out_wav,
        } => {
            validate_non_empty_bounded("out_wav", &out_wav, 512)?;
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let report = render_project_mix(project, &PathBuf::from(&out_wav))?;
            project.mix.last_render_path = Some(report.out_path.clone());
            project.mix.last_lufs = Some(report.integrated_lufs);
            maybe_save(project, project_path.as_deref())?;
            Ok(json!({ "ok": true, "render": report }))
        }
        Command::ExportStems {
            project_path,
            out_dir,
        } => {
            validate_non_empty_bounded("out_dir", &out_dir, 512)?;
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let handoff = export_djcraft_handoff(project, &PathBuf::from(&out_dir))?;
            project.stems = handoff.stems.clone();
            Ok(json!({ "ok": true, "handoff": handoff }))
        }
        Command::ExportSong {
            project_path,
            out_dir,
        } => {
            validate_non_empty_bounded("out_dir", &out_dir, 512)?;
            if let Some(p) = &project_path {
                *project = SongProject::load_from_file(&PathBuf::from(p))?;
            }
            let dir = PathBuf::from(&out_dir);
            let handoff = export_djcraft_handoff(project, &dir)?;
            let proj_out = dir.join("project.mcraft");
            project.save_to_file(&proj_out)?;
            Ok(json!({
                "ok": true,
                "project_file": proj_out.to_string_lossy(),
                "handoff": handoff
            }))
        }
    }
}

/// Stateless async ArtCraft Command Bus entrypoint shared by L2 (`musiccraft-mcp`) and L3 (`musiccraft-app`).
pub async fn dispatch(cmd: Command) -> Value {
    let mut project = match &cmd {
        Command::ProjectOpen { path } => match SongProject::load_from_file(&PathBuf::from(path)) {
            Ok(p) => p,
            Err(e) => return json!({ "ok": false, "error": e.to_string() }),
        },
        _ => SongProject::new_demo(),
    };
    dispatch_on_project(&mut project, cmd)
}

/// Synchronous wrapper for non-async callers.
pub fn dispatch_sync(cmd: Command) -> Value {
    let mut project = SongProject::new_demo();
    dispatch_on_project(&mut project, cmd)
}

/// Load a project from `path` if provided, otherwise return the deterministic demo project.
pub fn resolve_project(path: Option<&str>) -> Result<SongProject> {
    load_or_demo(path)
}
