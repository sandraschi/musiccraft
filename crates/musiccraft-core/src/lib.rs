//! MusicCraft L1 Core Engine.
//!
//! Pure-Rust song project AST, WAV PCM I/O, formant vocal synthesizer,
//! 4-band stem splitter, offline mix renderer, DJCraft handoff packager,
//! and universal `Command::dispatch` bus.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod audio;
pub mod dispatch;
pub mod error;
pub mod project;

pub const APP_NAME: &str = "MusicCraft";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use audio::{
    export_djcraft_handoff, key_to_camelot, measure_loudness, read_wav_16bit, render_project_mix,
    render_project_stereo_samples, render_vocal_mono, split_stems, write_wav_16bit, DjCraftHandoff,
    MixRenderReport, WavBuffer,
};
pub use dispatch::{
    command_catalog, dispatch, dispatch_on_project, dispatch_sync, resolve_project, Command,
    CommandSpec,
};
pub use error::{MusicCraftError, Result};
pub use project::{
    build_pitch_curve, midi_to_hz, parse_phoneme_line, MidiNote, MixRecipe, PhonemeEvent,
    PitchPoint, Region, SongProject, StemAsset, StemKind, Track, TrackKind, VocalLine, VocalVoice,
    DEFAULT_SAMPLE_RATE,
};
