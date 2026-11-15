use crate::error::{validate_non_empty_bounded, MusicCraftError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

pub const PROJECT_SCHEMA_VERSION: &str = "1.0.0";
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;
pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Audio,
    Midi,
    Vocal,
    Stem,
}

impl TrackKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Midi => "midi",
            Self::Vocal => "vocal",
            Self::Stem => "stem",
        }
    }

    pub fn from_str_loose(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "audio" => Ok(Self::Audio),
            "midi" => Ok(Self::Midi),
            "vocal" => Ok(Self::Vocal),
            "stem" => Ok(Self::Stem),
            other => Err(MusicCraftError::InvalidInput(format!(
                "unknown track kind '{other}'; expected audio, midi, vocal, or stem"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiNote {
    pub pitch: u8,
    pub velocity: u8,
    pub start_beat: f64,
    pub duration_beats: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub start_beat: f64,
    pub length_beats: f64,
    pub notes: Vec<MidiNote>,
    pub audio_ref: Option<String>,
    pub gain_db: f32,
    #[serde(default)]
    pub waveform_peaks: Vec<f32>,
}

/// Per-track DSP effect plugin (REAPER-style channel FX rack).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrackFx {
    Eq3Band {
        enabled: bool,
        low_db: f32,
        mid_db: f32,
        high_db: f32,
    },
    Compressor {
        enabled: bool,
        threshold_db: f32,
        ratio: f32,
    },
    Delay {
        enabled: bool,
        time_beats: f32,
        feedback: f32,
        mix: f32,
    },
    Reverb {
        enabled: bool,
        room_size: f32,
        mix: f32,
    },
}

impl TrackFx {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Eq3Band { .. } => "3-Band EQ",
            Self::Compressor { .. } => "Compressor",
            Self::Delay { .. } => "Stereo Delay",
            Self::Reverb { .. } => "Hall Reverb",
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Eq3Band { enabled, .. }
            | Self::Compressor { enabled, .. }
            | Self::Delay { enabled, .. }
            | Self::Reverb { enabled, .. } => *enabled,
        }
    }
}

/// Named timeline marker (Intro, Verse, Chorus, Drop, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub id: String,
    pub name: String,
    pub beat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: TrackKind,
    pub volume_db: f32,
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    pub regions: Vec<Region>,
    #[serde(default)]
    pub fx: Vec<TrackFx>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VocalVoice {
    Opera,
    Soprano,
    Tenor,
    Choir,
    Synth,
}

impl VocalVoice {
    pub fn from_str_loose(s: &str) -> Result<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "OPERA" => Ok(Self::Opera),
            "SOPRANO" => Ok(Self::Soprano),
            "TENOR" => Ok(Self::Tenor),
            "CHOIR" => Ok(Self::Choir),
            "SYNTH" => Ok(Self::Synth),
            other => Err(MusicCraftError::InvalidInput(format!(
                "unknown vocal voice '{other}'; expected OPERA, SOPRANO, TENOR, CHOIR, or SYNTH"
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Opera => "OPERA",
            Self::Soprano => "SOPRANO",
            Self::Tenor => "TENOR",
            Self::Choir => "CHOIR",
            Self::Synth => "SYNTH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhonemeEvent {
    pub phoneme: String,
    pub midi_note: u8,
    pub start_beat: f64,
    pub duration_beats: f64,
    pub vibrato_depth: f32,
    pub vibrato_rate_hz: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchPoint {
    pub beat: f64,
    pub hz: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VocalLine {
    pub voice: VocalVoice,
    pub phonemes: Vec<PhonemeEvent>,
    pub pitch_curve: Vec<PitchPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StemKind {
    Vocals,
    Drums,
    Bass,
    Other,
}

impl StemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vocals => "vocals",
            Self::Drums => "drums",
            Self::Bass => "bass",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemAsset {
    pub kind: StemKind,
    pub path: String,
    pub rms_db: f32,
    pub peak_db: f32,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixRecipe {
    pub master_gain_db: f32,
    pub target_lufs: f32,
    pub normalize: bool,
    pub last_render_path: Option<String>,
    pub last_lufs: Option<f32>,
}

impl Default for MixRecipe {
    fn default() -> Self {
        Self {
            master_gain_db: 0.0,
            target_lufs: -14.0,
            normalize: true,
            last_render_path: None,
            last_lufs: None,
        }
    }
}

/// Canonical `.mcraft` song project document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SongProject {
    pub version: String,
    pub name: String,
    pub bpm: f64,
    pub key: String,
    pub time_signature: (u8, u8),
    pub sample_rate: u32,
    pub created_at: DateTime<Utc>,
    pub tracks: Vec<Track>,
    pub vocal_line: Option<VocalLine>,
    pub stems: Vec<StemAsset>,
    pub mix: MixRecipe,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub loop_start_beat: f64,
    #[serde(default = "default_loop_end")]
    pub loop_end_beat: f64,
    #[serde(default)]
    pub metronome_enabled: bool,
}

fn default_loop_end() -> f64 {
    16.0
}

impl SongProject {
    /// Create a new empty song project with validated parameters.
    pub fn new(name: &str, bpm: f64, key: &str) -> Result<Self> {
        validate_non_empty_bounded("project name", name, 128)?;
        validate_non_empty_bounded("musical key", key, 32)?;
        if !bpm.is_finite() || !(MIN_BPM..=MAX_BPM).contains(&bpm) {
            return Err(MusicCraftError::InvalidInput(format!(
                "bpm must be between {MIN_BPM} and {MAX_BPM}, got {bpm}"
            )));
        }
        Ok(Self {
            version: PROJECT_SCHEMA_VERSION.to_string(),
            name: name.trim().to_string(),
            bpm,
            key: key.trim().to_string(),
            time_signature: (4, 4),
            sample_rate: DEFAULT_SAMPLE_RATE,
            created_at: Utc::now(),
            tracks: Vec::new(),
            vocal_line: None,
            stems: Vec::new(),
            mix: MixRecipe::default(),
            markers: Vec::new(),
            loop_start_beat: 0.0,
            loop_end_beat: 16.0,
            metronome_enabled: false,
        })
    }

    /// Deterministic demo song project for first-paint studio preview and `--smoke`.
    pub fn new_demo() -> Self {
        let mut proj = Self {
            version: PROJECT_SCHEMA_VERSION.to_string(),
            name: "Vienna Aria (Demo)".to_string(),
            bpm: 120.0,
            key: "C major".to_string(),
            time_signature: (4, 4),
            sample_rate: DEFAULT_SAMPLE_RATE,
            created_at: Utc::now(),
            tracks: Vec::new(),
            vocal_line: None,
            stems: Vec::new(),
            mix: MixRecipe::default(),
            markers: vec![
                Marker {
                    id: "mkr-intro".to_string(),
                    name: "Intro".to_string(),
                    beat: 0.0,
                },
                Marker {
                    id: "mkr-aria".to_string(),
                    name: "Aria Hook".to_string(),
                    beat: 8.0,
                },
            ],
            loop_start_beat: 0.0,
            loop_end_beat: 16.0,
            metronome_enabled: false,
        };

        proj.tracks.push(Track {
            id: "trk-piano".to_string(),
            name: "Grand Piano".to_string(),
            kind: TrackKind::Midi,
            volume_db: -3.0,
            pan: -0.15,
            muted: false,
            solo: false,
            regions: vec![Region {
                id: "reg-chords".to_string(),
                name: "C-G-Am-F Progression".to_string(),
                start_beat: 0.0,
                length_beats: 16.0,
                notes: vec![
                    MidiNote {
                        pitch: 60,
                        velocity: 92,
                        start_beat: 0.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 64,
                        velocity: 88,
                        start_beat: 0.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 67,
                        velocity: 86,
                        start_beat: 0.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 55,
                        velocity: 90,
                        start_beat: 4.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 59,
                        velocity: 84,
                        start_beat: 4.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 62,
                        velocity: 84,
                        start_beat: 4.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 57,
                        velocity: 90,
                        start_beat: 8.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 60,
                        velocity: 85,
                        start_beat: 8.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 64,
                        velocity: 85,
                        start_beat: 8.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 53,
                        velocity: 92,
                        start_beat: 12.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 57,
                        velocity: 86,
                        start_beat: 12.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 60,
                        velocity: 86,
                        start_beat: 12.0,
                        duration_beats: 3.5,
                    },
                ],
                audio_ref: None,
                gain_db: 0.0,
                waveform_peaks: Vec::new(),
            }],
            fx: vec![TrackFx::Eq3Band {
                enabled: true,
                low_db: 0.0,
                mid_db: 1.0,
                high_db: 1.5,
            }],
        });

        proj.tracks.push(Track {
            id: "trk-bass".to_string(),
            name: "Sub Bass".to_string(),
            kind: TrackKind::Midi,
            volume_db: -4.5,
            pan: 0.0,
            muted: false,
            solo: false,
            regions: vec![Region {
                id: "reg-bass".to_string(),
                name: "Root Pulse".to_string(),
                start_beat: 0.0,
                length_beats: 16.0,
                notes: vec![
                    MidiNote {
                        pitch: 36,
                        velocity: 105,
                        start_beat: 0.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 43,
                        velocity: 100,
                        start_beat: 4.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 45,
                        velocity: 100,
                        start_beat: 8.0,
                        duration_beats: 3.5,
                    },
                    MidiNote {
                        pitch: 41,
                        velocity: 102,
                        start_beat: 12.0,
                        duration_beats: 3.5,
                    },
                ],
                audio_ref: None,
                gain_db: 0.0,
                waveform_peaks: Vec::new(),
            }],
            fx: vec![TrackFx::Eq3Band {
                enabled: true,
                low_db: 0.0,
                mid_db: 1.0,
                high_db: 1.5,
            }],
        });

        proj.tracks.push(Track {
            id: "trk-vocal".to_string(),
            name: "Lead Aria".to_string(),
            kind: TrackKind::Vocal,
            volume_db: -1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            regions: vec![Region {
                id: "reg-aria".to_string(),
                name: "Aria Phrase 1".to_string(),
                start_beat: 0.0,
                length_beats: 16.0,
                notes: Vec::new(),
                audio_ref: None,
                gain_db: 0.0,
                waveform_peaks: Vec::new(),
            }],
            fx: vec![TrackFx::Eq3Band {
                enabled: true,
                low_db: 0.0,
                mid_db: 1.0,
                high_db: 1.5,
            }],
        });

        let phonemes = parse_phoneme_line("aa ee oo ah", 69, 4.0);
        let pitch_curve = build_pitch_curve(&phonemes);
        proj.vocal_line = Some(VocalLine {
            voice: VocalVoice::Opera,
            phonemes,
            pitch_curve,
        });

        proj
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let project: Self = serde_json::from_str(&content)?;
        Ok(project)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn add_track(&mut self, name: &str, kind: TrackKind) -> Result<&Track> {
        validate_non_empty_bounded("track name", name, 128)?;
        let id = format!(
            "trk-{}",
            Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("0000")
        );
        self.tracks.push(Track {
            id,
            name: name.trim().to_string(),
            kind,
            volume_db: 0.0,
            pan: 0.0,
            muted: false,
            solo: false,
            regions: Vec::new(),
            fx: Vec::new(),
        });
        self.tracks
            .last()
            .ok_or_else(|| MusicCraftError::TrackNotFound("newly added track missing".to_string()))
    }

    pub fn set_track_mix(
        &mut self,
        track_id: &str,
        volume_db: Option<f32>,
        pan: Option<f32>,
        muted: Option<bool>,
        solo: Option<bool>,
    ) -> Result<&Track> {
        let track = self
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id || t.name.eq_ignore_ascii_case(track_id))
            .ok_or_else(|| MusicCraftError::TrackNotFound(track_id.to_string()))?;

        if let Some(v) = volume_db {
            if v.is_finite() {
                track.volume_db = v.clamp(-60.0, 12.0);
            }
        }
        if let Some(p) = pan {
            if p.is_finite() {
                track.pan = p.clamp(-1.0, 1.0);
            }
        }
        if let Some(m) = muted {
            track.muted = m;
        }
        if let Some(s) = solo {
            track.solo = s;
        }
        Ok(track)
    }

    pub fn add_region(&mut self, track_id: &str, region: Region) -> Result<()> {
        let track = self
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id || t.name.eq_ignore_ascii_case(track_id))
            .ok_or_else(|| MusicCraftError::TrackNotFound(track_id.to_string()))?;
        track.regions.push(region);
        Ok(())
    }

    /// Quantize all MIDI notes in the project (or a specific track) to `grid_beats`.
    pub fn quantize_notes(&mut self, track_id: Option<&str>, grid_beats: f64) -> Result<usize> {
        if !grid_beats.is_finite() || grid_beats <= 0.0 || grid_beats > 16.0 {
            return Err(MusicCraftError::InvalidInput(
                "grid_beats must be in (0.0, 16.0]".to_string(),
            ));
        }
        let mut count = 0usize;
        for track in &mut self.tracks {
            if let Some(target) = track_id {
                if track.id != target && !track.name.eq_ignore_ascii_case(target) {
                    continue;
                }
            }
            for region in &mut track.regions {
                for note in &mut region.notes {
                    note.start_beat = (note.start_beat / grid_beats).round() * grid_beats;
                    count = count.saturating_add(1);
                }
            }
        }
        Ok(count)
    }

    /// Transpose all MIDI notes (and vocal phonemes if requested) by `semitones`.
    pub fn transpose_pitch(&mut self, semitones: i32, include_vocal: bool) -> Result<usize> {
        if !(-48..=48).contains(&semitones) {
            return Err(MusicCraftError::InvalidInput(
                "semitones must be between -48 and 48".to_string(),
            ));
        }
        let mut count = 0usize;
        for track in &mut self.tracks {
            for region in &mut track.regions {
                for note in &mut region.notes {
                    let shifted = (i32::from(note.pitch) + semitones).clamp(0, 127) as u8;
                    note.pitch = shifted;
                    count = count.saturating_add(1);
                }
            }
        }
        if include_vocal {
            if let Some(vocal) = &mut self.vocal_line {
                for ph in &mut vocal.phonemes {
                    ph.midi_note = (i32::from(ph.midi_note) + semitones).clamp(24, 108) as u8;
                    count = count.saturating_add(1);
                }
                vocal.pitch_curve = build_pitch_curve(&vocal.phonemes);
            }
        }
        Ok(count)
    }

    /// Split a region at `split_beat` (relative to project start) into two regions.
    pub fn split_region(&mut self, region_id: &str, split_beat: f64) -> Result<()> {
        if !split_beat.is_finite() {
            return Err(MusicCraftError::InvalidInput(
                "split_beat must be finite".to_string(),
            ));
        }
        for track in &mut self.tracks {
            if let Some(pos) = track.regions.iter().position(|r| r.id == region_id) {
                let region = &track.regions[pos];
                let end = region.start_beat + region.length_beats;
                if split_beat <= region.start_beat || split_beat >= end {
                    return Err(MusicCraftError::InvalidInput(format!(
                        "split_beat ({split_beat}) must lie strictly inside [{}, {end})",
                        region.start_beat
                    )));
                }
                let left_len = split_beat - region.start_beat;
                let right_len = end - split_beat;
                let offset_local = left_len;

                let mut left_notes = Vec::new();
                let mut right_notes = Vec::new();
                for n in &region.notes {
                    if n.start_beat < offset_local {
                        let mut ln = n.clone();
                        ln.duration_beats = ln.duration_beats.min(offset_local - ln.start_beat);
                        left_notes.push(ln);
                    } else {
                        let mut rn = n.clone();
                        rn.start_beat -= offset_local;
                        right_notes.push(rn);
                    }
                }

                let right_region = Region {
                    id: format!("{}-b", region.id),
                    name: format!("{} (Part 2)", region.name),
                    start_beat: split_beat,
                    length_beats: right_len,
                    notes: right_notes,
                    audio_ref: region.audio_ref.clone(),
                    gain_db: region.gain_db,
                    waveform_peaks: Vec::new(),
                };

                let left_region = &mut track.regions[pos];
                left_region.length_beats = left_len;
                left_region.notes = left_notes;
                track.regions.insert(pos + 1, right_region);
                return Ok(());
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    /// Total duration of the arrangement in beats.
    pub fn total_beats(&self) -> f64 {
        let track_max = self
            .tracks
            .iter()
            .flat_map(|t| t.regions.iter())
            .map(|r| r.start_beat + r.length_beats)
            .fold(0.0_f64, f64::max);
        let vocal_max = self
            .vocal_line
            .as_ref()
            .and_then(|v| {
                v.phonemes
                    .iter()
                    .map(|p| p.start_beat + p.duration_beats)
                    .reduce(f64::max)
            })
            .unwrap_or(0.0);
        track_max.max(vocal_max).max(4.0)
    }

    pub fn total_seconds(&self) -> f64 {
        (self.total_beats() * 60.0) / self.bpm
    }

    pub fn move_region(&mut self, region_id: &str, new_start_beat: f64) -> Result<()> {
        if !new_start_beat.is_finite() || new_start_beat < 0.0 {
            return Err(MusicCraftError::InvalidInput(
                "new_start_beat must be finite and >= 0.0".into(),
            ));
        }
        for track in &mut self.tracks {
            if let Some(reg) = track.regions.iter_mut().find(|r| r.id == region_id) {
                reg.start_beat = new_start_beat;
                return Ok(());
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn resize_region(&mut self, region_id: &str, new_length_beats: f64) -> Result<()> {
        if !new_length_beats.is_finite() || new_length_beats < 0.25 {
            return Err(MusicCraftError::InvalidInput(
                "new_length_beats must be >= 0.25".into(),
            ));
        }
        for track in &mut self.tracks {
            if let Some(reg) = track.regions.iter_mut().find(|r| r.id == region_id) {
                reg.length_beats = new_length_beats;
                return Ok(());
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn duplicate_region(&mut self, region_id: &str) -> Result<Region> {
        for track in &mut self.tracks {
            if let Some(reg) = track.regions.iter().find(|r| r.id == region_id).cloned() {
                let mut copy = reg;
                copy.id = format!(
                    "reg-{}",
                    Uuid::new_v4()
                        .to_string()
                        .split('-')
                        .next()
                        .unwrap_or("dup")
                );
                copy.name = format!("{} (Copy)", copy.name);
                copy.start_beat += copy.length_beats;
                track.regions.push(copy.clone());
                return Ok(copy);
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn delete_region(&mut self, region_id: &str) -> Result<()> {
        for track in &mut self.tracks {
            if let Some(pos) = track.regions.iter().position(|r| r.id == region_id) {
                track.regions.remove(pos);
                return Ok(());
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn add_midi_note(&mut self, region_id: &str, note: MidiNote) -> Result<()> {
        for track in &mut self.tracks {
            if let Some(reg) = track.regions.iter_mut().find(|r| r.id == region_id) {
                reg.notes.push(note);
                return Ok(());
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn remove_midi_note(
        &mut self,
        region_id: &str,
        pitch: u8,
        start_beat: f64,
    ) -> Result<bool> {
        for track in &mut self.tracks {
            if let Some(reg) = track.regions.iter_mut().find(|r| r.id == region_id) {
                let before = reg.notes.len();
                reg.notes
                    .retain(|n| !(n.pitch == pitch && (n.start_beat - start_beat).abs() < 0.25));
                return Ok(reg.notes.len() < before);
            }
        }
        Err(MusicCraftError::RegionNotFound(region_id.to_string()))
    }

    pub fn add_marker(&mut self, name: &str, beat: f64) -> Result<Marker> {
        validate_non_empty_bounded("marker name", name, 64)?;
        if !beat.is_finite() || beat < 0.0 {
            return Err(MusicCraftError::InvalidInput("beat must be >= 0".into()));
        }
        let m = Marker {
            id: format!(
                "mkr-{}",
                Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("000")
            ),
            name: name.trim().to_string(),
            beat,
        };
        self.markers.push(m.clone());
        Ok(m)
    }

    pub fn add_track_fx(&mut self, track_id: &str, fx: TrackFx) -> Result<()> {
        let track = self
            .tracks
            .iter_mut()
            .find(|t| t.id == track_id || t.name.eq_ignore_ascii_case(track_id))
            .ok_or_else(|| MusicCraftError::TrackNotFound(track_id.to_string()))?;
        track.fx.push(fx);
        Ok(())
    }
}

/// Parse a space- or comma-separated phoneme string into timed phoneme events.
pub fn parse_phoneme_line(line: &str, base_midi: u8, beats_per_phoneme: f64) -> Vec<PhonemeEvent> {
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ',' || c == '|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let step = if beats_per_phoneme.is_finite() && beats_per_phoneme > 0.0 {
        beats_per_phoneme
    } else {
        1.0
    };

    // Gentle melodic contour across the phrase so the virtual singer sounds musical.
    let contour: [i16; 8] = [0, 2, 4, 5, 4, 7, 5, 0];
    tokens
        .into_iter()
        .enumerate()
        .map(|(i, tok)| {
            let offset = contour.get(i % contour.len()).copied().unwrap_or(0);
            let midi = (i16::from(base_midi) + offset).clamp(36, 96) as u8;
            PhonemeEvent {
                phoneme: tok.to_ascii_lowercase(),
                midi_note: midi,
                start_beat: (i as f64) * step,
                duration_beats: step * 0.92,
                vibrato_depth: 0.25,
                vibrato_rate_hz: 5.5,
            }
        })
        .collect()
}

/// Convert MIDI note to frequency in Hz.
pub fn midi_to_hz(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(midi) - 69.0) / 12.0)
}

/// Build a continuous pitch curve from phoneme events.
pub fn build_pitch_curve(phonemes: &[PhonemeEvent]) -> Vec<PitchPoint> {
    let mut points = Vec::with_capacity(phonemes.len().saturating_mul(2));
    for ph in phonemes {
        let hz = midi_to_hz(ph.midi_note);
        points.push(PitchPoint {
            beat: ph.start_beat,
            hz,
        });
        points.push(PitchPoint {
            beat: ph.start_beat + ph.duration_beats,
            hz,
        });
    }
    points
}
