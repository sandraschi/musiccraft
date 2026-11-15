use crate::error::{MusicCraftError, Result};
use crate::project::{
    midi_to_hz, SongProject, StemAsset, StemKind, TrackFx, TrackKind, VocalLine, VocalVoice,
    DEFAULT_SAMPLE_RATE,
};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;
use std::path::Path;

/// Pure-Rust in-memory PCM audio buffer (interleaved f32 in [-1.0, 1.0]).
#[derive(Debug, Clone)]
pub struct WavBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixRenderReport {
    pub out_path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f64,
    pub total_frames: usize,
    pub peak_db: f32,
    pub rms_db: f32,
    pub integrated_lufs: f32,
    pub tracks_rendered: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjCraftHandoff {
    pub schema: String,
    pub source_app: String,
    pub project_name: String,
    pub bpm: f64,
    pub key: String,
    pub camelot: String,
    pub duration_secs: f64,
    pub master_mix_path: String,
    pub stems: Vec<StemAsset>,
}

fn db_to_linear(db: f32) -> f32 {
    if db <= -90.0 {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

fn linear_to_db(lin: f32) -> f32 {
    if lin <= 1e-6 {
        -120.0
    } else {
        20.0 * lin.log10()
    }
}

/// Compute peak dB, RMS dB, and approximate integrated LUFS for stereo interleaved samples.
pub fn measure_loudness(samples: &[f32]) -> (f32, f32, f32) {
    if samples.is_empty() {
        return (-120.0, -120.0, -70.0);
    }
    let mut peak = 0.0_f32;
    let mut sum_sq = 0.0_f64;
    for &s in samples {
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
        let d = f64::from(s);
        sum_sq += d * d;
    }
    let rms = (sum_sq / (samples.len() as f64)).sqrt() as f32;
    let peak_db = linear_to_db(peak);
    let rms_db = linear_to_db(rms);
    // Approximate K-weighted LUFS offset from RMS for broadband musical material.
    let lufs = (rms_db - 0.7).max(-70.0);
    (peak_db, rms_db, lufs)
}

/// Write a 16-bit PCM RIFF/WAVE file with bounds-checked header construction.
pub fn write_wav_16bit(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> Result<()> {
    if channels == 0 || channels > 2 {
        return Err(MusicCraftError::InvalidWav(format!(
            "unsupported channel count {channels}; expected 1 or 2"
        )));
    }
    if sample_rate == 0 {
        return Err(MusicCraftError::InvalidWav(
            "sample_rate must be > 0".to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let data_bytes_u32 = u32::try_from(samples.len().saturating_mul(2))
        .map_err(|_| MusicCraftError::InvalidWav("audio buffer exceeds 4 GB WAV limit".into()))?;
    let riff_size = 36_u32
        .checked_add(data_bytes_u32)
        .ok_or_else(|| MusicCraftError::InvalidWav("RIFF chunk size overflow".into()))?;
    let byte_rate = sample_rate
        .checked_mul(u32::from(channels))
        .and_then(|v| v.checked_mul(2))
        .ok_or_else(|| MusicCraftError::InvalidWav("byte_rate overflow".into()))?;
    let block_align = channels
        .checked_mul(2)
        .ok_or_else(|| MusicCraftError::InvalidWav("block_align overflow".into()))?;

    let mut bytes = Vec::with_capacity(44_usize.saturating_add(data_bytes_u32 as usize));
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes_u32.to_le_bytes());

    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let q = (clamped * 32767.0).round() as i16;
        bytes.extend_from_slice(&q.to_le_bytes());
    }

    std::fs::write(path, bytes)?;
    Ok(())
}

/// Read a 16-bit PCM RIFF/WAVE file using a hostile-input-fenced parser.
pub fn read_wav_16bit(path: &Path) -> Result<WavBuffer> {
    let raw = std::fs::read(path)?;
    if raw.len() < 44 {
        return Err(MusicCraftError::InvalidWav(
            "file smaller than 44-byte WAV header".to_string(),
        ));
    }
    if raw.get(0..4) != Some(b"RIFF") || raw.get(8..12) != Some(b"WAVE") {
        return Err(MusicCraftError::InvalidWav(
            "missing RIFF/WAVE signature".to_string(),
        ));
    }

    let mut pos = 12usize;
    let mut sample_rate = DEFAULT_SAMPLE_RATE;
    let mut channels = 2u16;
    let mut bits_per_sample = 16u16;
    let mut found_fmt = false;
    let mut pcm_samples: Option<Vec<f32>> = None;

    while pos.saturating_add(8) <= raw.len() {
        let chunk_id = raw
            .get(pos..pos + 4)
            .ok_or_else(|| MusicCraftError::InvalidWav("truncated chunk id".into()))?;
        let len_bytes: [u8; 4] = raw
            .get(pos + 4..pos + 8)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| MusicCraftError::InvalidWav("truncated chunk size".into()))?;
        let chunk_len = u32::from_le_bytes(len_bytes) as usize;
        let payload_start = pos
            .checked_add(8)
            .ok_or_else(|| MusicCraftError::InvalidWav("chunk offset overflow".into()))?;
        let payload_end = payload_start
            .checked_add(chunk_len)
            .ok_or_else(|| MusicCraftError::InvalidWav("chunk end overflow".into()))?;
        if payload_end > raw.len() {
            return Err(MusicCraftError::InvalidWav(
                "WAV chunk extends past EOF".to_string(),
            ));
        }

        if chunk_id == b"fmt " && chunk_len >= 16 {
            let fmt = raw
                .get(payload_start..payload_end)
                .ok_or_else(|| MusicCraftError::InvalidWav("truncated fmt chunk".into()))?;
            let audio_fmt =
                u16::from_le_bytes([*fmt.first().unwrap_or(&0), *fmt.get(1).unwrap_or(&0)]);
            if audio_fmt != 1 {
                return Err(MusicCraftError::InvalidWav(format!(
                    "only uncompressed PCM (format 1) is supported, got {audio_fmt}"
                )));
            }
            channels = u16::from_le_bytes([*fmt.get(2).unwrap_or(&1), *fmt.get(3).unwrap_or(&0)]);
            sample_rate = u32::from_le_bytes([
                *fmt.get(4).unwrap_or(&0),
                *fmt.get(5).unwrap_or(&0),
                *fmt.get(6).unwrap_or(&0),
                *fmt.get(7).unwrap_or(&0),
            ]);
            bits_per_sample =
                u16::from_le_bytes([*fmt.get(14).unwrap_or(&16), *fmt.get(15).unwrap_or(&0)]);
            found_fmt = true;
        } else if chunk_id == b"data" {
            if bits_per_sample != 16 {
                return Err(MusicCraftError::InvalidWav(format!(
                    "only 16-bit PCM is supported, got {bits_per_sample}-bit"
                )));
            }
            let data = raw
                .get(payload_start..payload_end)
                .ok_or_else(|| MusicCraftError::InvalidWav("truncated data chunk".into()))?;
            let mut out = Vec::with_capacity(data.len() / 2);
            for pair in data.as_chunks::<2>().0 {
                let sample = i16::from_le_bytes(*pair);
                out.push(f32::from(sample) / 32768.0);
            }
            pcm_samples = Some(out);
            break;
        }

        // RIFF chunks are padded to even byte boundaries.
        let padded_len = chunk_len.saturating_add(chunk_len % 2);
        pos = payload_start
            .checked_add(padded_len)
            .ok_or_else(|| MusicCraftError::InvalidWav("next chunk offset overflow".into()))?;
    }

    if !found_fmt {
        return Err(MusicCraftError::InvalidWav("missing fmt chunk".to_string()));
    }
    let samples =
        pcm_samples.ok_or_else(|| MusicCraftError::InvalidWav("missing data chunk".to_string()))?;
    Ok(WavBuffer {
        sample_rate: sample_rate.max(8_000),
        channels: channels.clamp(1, 2),
        samples,
    })
}

/// Formant center frequencies (F1, F2, F3 in Hz) for vowel phonemes.
fn vowel_formants(phoneme: &str) -> (f32, f32, f32) {
    let p = phoneme.trim().to_ascii_lowercase();
    if p.contains('i') || p.contains("ee") {
        (280.0, 2250.0, 2890.0)
    } else if p.contains('e') || p.contains("eh") {
        (530.0, 1840.0, 2480.0)
    } else if p.contains('u') || p.contains("oo") {
        (310.0, 870.0, 2240.0)
    } else if p.contains('o') || p.contains("oh") {
        (500.0, 920.0, 2350.0)
    } else {
        // Default open 'aa' / 'ah' aria vowel
        (730.0, 1090.0, 2440.0)
    }
}

/// Synthesize a mono vocal track from a `VocalLine` using harmonic glottal + formant resonance.
pub fn render_vocal_mono(
    vocal: &VocalLine,
    bpm: f64,
    sample_rate: u32,
    total_frames: usize,
) -> Vec<f32> {
    let mut buf = vec![0.0_f32; total_frames];
    let sr = sample_rate as f32;
    let beats_per_sec = bpm / 60.0;

    let (bright_scale, formant_shift) = match vocal.voice {
        VocalVoice::Opera => (1.0, 1.0),
        VocalVoice::Soprano => (1.15, 1.18),
        VocalVoice::Tenor => (0.95, 0.90),
        VocalVoice::Choir => (0.85, 0.96),
        VocalVoice::Synth => (1.25, 1.05),
    };

    for ph in &vocal.phonemes {
        let start_frame = ((ph.start_beat / beats_per_sec) * f64::from(sr)).round() as usize;
        let dur_frames = ((ph.duration_beats / beats_per_sec) * f64::from(sr)).round() as usize;
        if start_frame >= total_frames || dur_frames == 0 {
            continue;
        }
        let end_frame = start_frame.saturating_add(dur_frames).min(total_frames);
        let base_f0 = midi_to_hz(ph.midi_note);
        let (f1, f2, f3) = vowel_formants(&ph.phoneme);
        let f1 = f1 * formant_shift;
        let f2 = f2 * formant_shift;
        let f3 = f3 * formant_shift;

        let mut phase = 0.0_f32;
        let attack = (0.04 * sr).max(1.0);
        let release = (0.06 * sr).max(1.0);

        for (local_i, frame_idx) in (start_frame..end_frame).enumerate() {
            let t = (local_i as f32) / sr;
            let remain = (end_frame - start_frame - local_i) as f32;
            let env = ((local_i as f32) / attack).min(1.0) * (remain / release).min(1.0);

            // Vibrato LFO
            let vibrato = 1.0 + 0.012 * ph.vibrato_depth * (TAU * ph.vibrato_rate_hz * t).sin();
            let f0 = base_f0 * vibrato;
            phase = (phase + f0 / sr) % 1.0;

            // Additive harmonic stack shaped by 3 formant Gaussians
            let mut sample = 0.0_f32;
            for h in 1..=10 {
                let hf = f0 * (h as f32);
                if hf > sr * 0.45 {
                    break;
                }
                let g1 = (-((hf - f1) / 140.0).powi(2)).exp();
                let g2 = 0.65 * (-((hf - f2) / 220.0).powi(2)).exp();
                let g3 = 0.35 * (-((hf - f3) / 300.0).powi(2)).exp();
                let amp = (g1 + g2 + g3 + 0.15 / (h as f32)) * bright_scale;
                sample += amp * (TAU * phase * (h as f32)).sin();
            }

            if let Some(slot) = buf.get_mut(frame_idx) {
                *slot += sample * env * 0.18;
            }
        }
    }

    buf
}

/// Compute downsampled peak amplitudes in `[0.0, 1.0]` for timeline waveform display.
pub fn compute_waveform_peaks(samples: &[f32], channels: u16, buckets: usize) -> Vec<f32> {
    let ch = usize::from(channels.max(1));
    let frames = samples.len() / ch;
    if frames == 0 || buckets == 0 {
        return Vec::new();
    }
    let step = (frames / buckets).max(1);
    let mut peaks = Vec::with_capacity(buckets);
    for b in 0..buckets {
        let start = b.saturating_mul(step);
        if start >= frames {
            break;
        }
        let end = start.saturating_add(step).min(frames);
        let mut max_amp = 0.0_f32;
        for f in start..end {
            let s = samples.get(f * ch).copied().unwrap_or(0.0).abs();
            if s > max_amp {
                max_amp = s;
            }
        }
        peaks.push(max_amp.min(1.0));
    }
    peaks
}

/// Apply a REAPER-style per-track DSP FX chain in-place on interleaved stereo frames.
fn apply_track_fx_chain(stereo: &mut [f32], fx_chain: &[TrackFx], sr: u32, bpm: f64) {
    let frames = stereo.len() / 2;
    if frames == 0 {
        return;
    }
    for fx in fx_chain {
        if !fx.is_enabled() {
            continue;
        }
        match *fx {
            TrackFx::Eq3Band {
                low_db,
                mid_db,
                high_db,
                ..
            } => {
                let g_low = db_to_linear(low_db);
                let g_mid = db_to_linear(mid_db);
                let g_high = db_to_linear(high_db);
                let mut lp_l = 0.0_f32;
                let mut lp_r = 0.0_f32;
                let mut hp_l = 0.0_f32;
                let mut hp_r = 0.0_f32;
                for i in 0..frames {
                    let l = stereo.get(i * 2).copied().unwrap_or(0.0);
                    let r = stereo.get(i * 2 + 1).copied().unwrap_or(0.0);
                    lp_l += 0.03 * (l - lp_l);
                    lp_r += 0.03 * (r - lp_r);
                    hp_l += 0.35 * (l - hp_l);
                    hp_r += 0.35 * (r - hp_r);
                    let mid_l = hp_l - lp_l;
                    let mid_r = hp_r - lp_r;
                    let hi_l = l - hp_l;
                    let hi_r = r - hp_r;
                    if let Some(sl) = stereo.get_mut(i * 2) {
                        *sl = lp_l * g_low + mid_l * g_mid + hi_l * g_high;
                    }
                    if let Some(sr_slot) = stereo.get_mut(i * 2 + 1) {
                        *sr_slot = lp_r * g_low + mid_r * g_mid + hi_r * g_high;
                    }
                }
            }
            TrackFx::Compressor {
                threshold_db,
                ratio,
                ..
            } => {
                let thresh = db_to_linear(threshold_db).max(1e-4);
                let inv_ratio = 1.0 / ratio.max(1.0);
                let mut env = 0.0_f32;
                for i in 0..frames {
                    let l = stereo.get(i * 2).copied().unwrap_or(0.0);
                    let r = stereo.get(i * 2 + 1).copied().unwrap_or(0.0);
                    let level = l.abs().max(r.abs());
                    let coeff = if level > env { 0.05 } else { 0.002 };
                    env += coeff * (level - env);
                    let gain = if env > thresh {
                        let over = env / thresh;
                        let compressed = thresh * over.powf(inv_ratio);
                        compressed / env
                    } else {
                        1.0
                    };
                    if let Some(sl) = stereo.get_mut(i * 2) {
                        *sl = l * gain;
                    }
                    if let Some(sr_slot) = stereo.get_mut(i * 2 + 1) {
                        *sr_slot = r * gain;
                    }
                }
            }
            TrackFx::Delay {
                time_beats,
                feedback,
                mix,
                ..
            } => {
                let delay_secs = (f64::from(time_beats.max(0.05)) * 60.0) / bpm.max(20.0);
                let delay_frames =
                    ((delay_secs * f64::from(sr)).round() as usize).clamp(1, sr as usize * 2);
                let fb = feedback.clamp(0.0, 0.85);
                let wet = mix.clamp(0.0, 1.0);
                let mut buf_l = vec![0.0_f32; delay_frames];
                let mut buf_r = vec![0.0_f32; delay_frames];
                let mut idx = 0usize;
                for i in 0..frames {
                    let l = stereo.get(i * 2).copied().unwrap_or(0.0);
                    let r = stereo.get(i * 2 + 1).copied().unwrap_or(0.0);
                    let dl = buf_l.get(idx).copied().unwrap_or(0.0);
                    let dr = buf_r.get(idx).copied().unwrap_or(0.0);
                    if let Some(slot) = buf_l.get_mut(idx) {
                        *slot = l + dr * fb;
                    }
                    if let Some(slot) = buf_r.get_mut(idx) {
                        *slot = r + dl * fb;
                    }
                    idx = (idx + 1) % delay_frames;
                    if let Some(sl) = stereo.get_mut(i * 2) {
                        *sl = l * (1.0 - wet * 0.5) + dl * wet;
                    }
                    if let Some(sr_slot) = stereo.get_mut(i * 2 + 1) {
                        *sr_slot = r * (1.0 - wet * 0.5) + dr * wet;
                    }
                }
            }
            TrackFx::Reverb { room_size, mix, .. } => {
                let scale = 0.5 + room_size.clamp(0.0, 1.0);
                let tap1 = ((0.0297 * f64::from(scale) * f64::from(sr)) as usize).max(1);
                let tap2 = ((0.0371 * f64::from(scale) * f64::from(sr)) as usize).max(1);
                let mut d1 = vec![0.0_f32; tap1];
                let mut d2 = vec![0.0_f32; tap2];
                let mut i1 = 0usize;
                let mut i2 = 0usize;
                let wet = mix.clamp(0.0, 1.0);
                for i in 0..frames {
                    let l = stereo.get(i * 2).copied().unwrap_or(0.0);
                    let r = stereo.get(i * 2 + 1).copied().unwrap_or(0.0);
                    let mono = 0.5 * (l + r);
                    let v1 = d1.get(i1).copied().unwrap_or(0.0);
                    let v2 = d2.get(i2).copied().unwrap_or(0.0);
                    if let Some(s) = d1.get_mut(i1) {
                        *s = mono + v1 * 0.62;
                    }
                    if let Some(s) = d2.get_mut(i2) {
                        *s = mono + v2 * 0.58;
                    }
                    i1 = (i1 + 1) % tap1;
                    i2 = (i2 + 1) % tap2;
                    let rev_l = v1 * 0.7 + v2 * 0.3;
                    let rev_r = v2 * 0.7 + v1 * 0.3;
                    if let Some(sl) = stereo.get_mut(i * 2) {
                        *sl = l * (1.0 - wet * 0.4) + rev_l * wet;
                    }
                    if let Some(sr_slot) = stereo.get_mut(i * 2 + 1) {
                        *sr_slot = r * (1.0 - wet * 0.4) + rev_r * wet;
                    }
                }
            }
        }
    }
}

/// Render the entire `SongProject` into an interleaved stereo f32 buffer `(sample_rate, tracks_rendered, samples)`.
pub fn render_project_stereo_samples(project: &SongProject) -> (u32, usize, Vec<f32>) {
    render_project_with_metronome(project, false)
}

/// Render the project with optional live metronome click track.
pub fn render_project_with_metronome(
    project: &SongProject,
    include_metronome: bool,
) -> (u32, usize, Vec<f32>) {
    let sr = project.sample_rate.max(8_000);
    let duration_secs = project.total_seconds().max(0.5);
    let total_frames = ((duration_secs * f64::from(sr)).ceil() as usize).max(1);
    let mut stereo = vec![0.0_f32; total_frames.saturating_mul(2)];

    let any_solo = project.tracks.iter().any(|t| t.solo);
    let beats_per_sec = project.bpm / 60.0;
    let mut tracks_rendered = 0usize;

    for track in &project.tracks {
        if track.muted || (any_solo && !track.solo) {
            continue;
        }
        tracks_rendered = tracks_rendered.saturating_add(1);
        let mut trk_buf = vec![0.0_f32; total_frames.saturating_mul(2)];

        if track.kind == TrackKind::Vocal {
            if let Some(vocal) = &project.vocal_line {
                let mono = render_vocal_mono(vocal, project.bpm, sr, total_frames);
                for (i, &s) in mono.iter().enumerate() {
                    if let Some(l) = trk_buf.get_mut(i * 2) {
                        *l += s;
                    }
                    if let Some(r) = trk_buf.get_mut(i * 2 + 1) {
                        *r += s;
                    }
                }
            }
        } else {
            // Render MIDI notes and imported WAV audio regions
            for region in &track.regions {
                let reg_gain = db_to_linear(region.gain_db);
                if let Some(wav_path) = &region.audio_ref {
                    if let Ok(wav) = read_wav_16bit(Path::new(wav_path)) {
                        let start_frame =
                            ((region.start_beat / beats_per_sec) * f64::from(sr)).round() as usize;
                        let max_frames = ((region.length_beats / beats_per_sec) * f64::from(sr))
                            .round() as usize;
                        let wav_ch = usize::from(wav.channels.max(1));
                        let wav_frames = wav.samples.len() / wav_ch;
                        let sr_ratio = f64::from(wav.sample_rate) / f64::from(sr);
                        for local_f in 0..max_frames {
                            let dst_f = start_frame.saturating_add(local_f);
                            if dst_f >= total_frames {
                                break;
                            }
                            let src_f = ((local_f as f64) * sr_ratio) as usize;
                            if src_f >= wav_frames {
                                break;
                            }
                            let wl = wav.samples.get(src_f * wav_ch).copied().unwrap_or(0.0);
                            let wr = if wav_ch == 2 {
                                wav.samples.get(src_f * wav_ch + 1).copied().unwrap_or(wl)
                            } else {
                                wl
                            };
                            if let Some(dl) = trk_buf.get_mut(dst_f * 2) {
                                *dl += wl * reg_gain;
                            }
                            if let Some(dr) = trk_buf.get_mut(dst_f * 2 + 1) {
                                *dr += wr * reg_gain;
                            }
                        }
                    }
                }

                for note in &region.notes {
                    let abs_beat = region.start_beat + note.start_beat;
                    let start_frame = ((abs_beat / beats_per_sec) * f64::from(sr)).round() as usize;
                    let dur_frames =
                        ((note.duration_beats / beats_per_sec) * f64::from(sr)).round() as usize;
                    if start_frame >= total_frames || dur_frames == 0 {
                        continue;
                    }
                    let end_frame = start_frame.saturating_add(dur_frames).min(total_frames);
                    let freq = midi_to_hz(note.pitch);
                    let vel = f32::from(note.velocity) / 127.0;
                    let attack = (0.012 * (sr as f32)).max(1.0);
                    let release = (0.08 * (sr as f32)).max(1.0);

                    for (local_i, frame_idx) in (start_frame..end_frame).enumerate() {
                        let remain = (end_frame - start_frame - local_i) as f32;
                        let env =
                            ((local_i as f32) / attack).min(1.0) * (remain / release).min(1.0);
                        let t = (local_i as f32) / (sr as f32);
                        let sig = 0.70 * (TAU * freq * t).sin()
                            + 0.22 * (TAU * freq * 2.0 * t).sin()
                            + 0.08 * (TAU * freq * 3.0 * t).sin();
                        let s = sig * env * vel * reg_gain * 0.22;
                        if let Some(l) = trk_buf.get_mut(frame_idx * 2) {
                            *l += s;
                        }
                        if let Some(r) = trk_buf.get_mut(frame_idx * 2 + 1) {
                            *r += s;
                        }
                    }
                }
            }
        }

        // Process per-track FX chain (EQ, Compressor, Delay, Reverb)
        apply_track_fx_chain(&mut trk_buf, &track.fx, sr, project.bpm);

        // Apply track volume & pan into master bus
        let gain = db_to_linear(track.volume_db);
        let pan = track.pan.clamp(-1.0, 1.0);
        let left_gain = gain * ((1.0 - pan) * 0.5).sqrt();
        let right_gain = gain * ((1.0 + pan) * 0.5).sqrt();

        for i in 0..total_frames {
            let tl = trk_buf.get(i * 2).copied().unwrap_or(0.0);
            let tr = trk_buf.get(i * 2 + 1).copied().unwrap_or(0.0);
            if let Some(ml) = stereo.get_mut(i * 2) {
                *ml += tl * left_gain;
            }
            if let Some(mr) = stereo.get_mut(i * 2 + 1) {
                *mr += tr * right_gain;
            }
        }
    }

    // Optional metronome click for live DAW playback
    if include_metronome && project.metronome_enabled {
        let total_beats = project.total_beats().ceil() as usize;
        let click_frames = ((0.035 * f64::from(sr)) as usize).max(1);
        for b in 0..total_beats {
            let start_frame = (((b as f64) / beats_per_sec) * f64::from(sr)).round() as usize;
            let is_bar = b % usize::from(project.time_signature.0.max(1)) == 0;
            let click_hz = if is_bar { 1320.0_f32 } else { 880.0_f32 };
            for i in 0..click_frames {
                let dst = start_frame.saturating_add(i);
                if dst >= total_frames {
                    break;
                }
                let env = (1.0 - (i as f32) / (click_frames as f32)).powi(2);
                let s = (TAU * click_hz * (i as f32) / (sr as f32)).sin() * env * 0.25;
                if let Some(ml) = stereo.get_mut(dst * 2) {
                    *ml += s;
                }
                if let Some(mr) = stereo.get_mut(dst * 2 + 1) {
                    *mr += s;
                }
            }
        }
    }

    // Master gain + optional normalization + soft-clip safety limiter
    let master_lin = db_to_linear(project.mix.master_gain_db);
    for s in &mut stereo {
        *s *= master_lin;
    }

    if project.mix.normalize {
        let (_, _, cur_lufs) = measure_loudness(&stereo);
        if cur_lufs > -65.0 {
            let delta_db = (project.mix.target_lufs - cur_lufs).clamp(-12.0, 18.0);
            let norm_lin = db_to_linear(delta_db);
            for s in &mut stereo {
                *s *= norm_lin;
            }
        }
    }

    for s in &mut stereo {
        *s = s.tanh();
    }

    (sr, tracks_rendered, stereo)
}

/// Offline-render the entire `SongProject` into a stereo 16-bit WAV file.
pub fn render_project_mix(project: &SongProject, out_wav: &Path) -> Result<MixRenderReport> {
    let (sr, tracks_rendered, stereo) = render_project_stereo_samples(project);
    let total_frames = stereo.len() / 2;

    write_wav_16bit(out_wav, sr, 2, &stereo)?;
    let (peak_db, rms_db, integrated_lufs) = measure_loudness(&stereo);

    Ok(MixRenderReport {
        out_path: out_wav.to_string_lossy().to_string(),
        sample_rate: sr,
        channels: 2,
        duration_secs: (total_frames as f64) / f64::from(sr),
        total_frames,
        peak_db,
        rms_db,
        integrated_lufs,
        tracks_rendered,
    })
}

/// Separate a WAV file (or synthesized demo mix if `input_wav` is None) into 4 stem WAV files.
pub fn split_stems(
    input_wav: Option<&Path>,
    out_dir: &Path,
    project_fallback: &SongProject,
) -> Result<Vec<StemAsset>> {
    std::fs::create_dir_all(out_dir)?;

    let buf = match input_wav {
        Some(path) => read_wav_16bit(path)?,
        None => {
            let tmp_mix = out_dir.join("_source_mix.wav");
            render_project_mix(project_fallback, &tmp_mix)?;
            let loaded = read_wav_16bit(&tmp_mix)?;
            let _ = std::fs::remove_file(&tmp_mix);
            loaded
        }
    };

    let ch = usize::from(buf.channels.max(1));
    let frames = buf.samples.len() / ch;
    let sr = buf.sample_rate;
    let duration_secs = (frames as f64) / f64::from(sr);

    let mut vocals = vec![0.0_f32; frames * 2];
    let mut drums = vec![0.0_f32; frames * 2];
    let mut bass = vec![0.0_f32; frames * 2];
    let mut other = vec![0.0_f32; frames * 2];

    // Simple 1-pole lowpass/bandpass state for deterministic 4-band stem extraction
    let mut lp_bass = 0.0_f32;
    let mut lp_mid = 0.0_f32;
    let mut prev_env = 0.0_f32;

    for i in 0..frames {
        let l = buf.samples.get(i * ch).copied().unwrap_or(0.0);
        let r = if ch == 2 {
            buf.samples.get(i * ch + 1).copied().unwrap_or(l)
        } else {
            l
        };
        let mid = 0.5 * (l + r);
        let side = 0.5 * (l - r);

        // Low-pass ~180 Hz for Bass
        lp_bass += 0.025 * (mid - lp_bass);
        // Band-mid ~2.5 kHz
        lp_mid += 0.28 * (mid - lp_mid);
        let high = mid - lp_mid;

        // Transient detector for Drums
        let abs_m = mid.abs();
        let transient = (abs_m - prev_env).max(0.0);
        prev_env = prev_env * 0.995 + abs_m * 0.005;
        let drum_sig = transient * 2.2 + high * 0.35;

        // Center-channel vocal band
        let vocal_sig = (lp_mid - lp_bass) * 0.85;
        // Remaining harmonic/stereo bed
        let other_l = side + (mid - vocal_sig - lp_bass) * 0.5;
        let other_r = -side + (mid - vocal_sig - lp_bass) * 0.5;

        let write_stereo = |buf: &mut [f32], l_val: f32, r_val: f32| {
            if let Some(pair) = buf.get_mut(i * 2..i * 2 + 2) {
                if let Some(l_slot) = pair.first_mut() {
                    *l_slot = l_val.clamp(-1.0, 1.0);
                }
                if let Some(r_slot) = pair.get_mut(1) {
                    *r_slot = r_val.clamp(-1.0, 1.0);
                }
            }
        };

        write_stereo(&mut vocals, vocal_sig, vocal_sig);
        write_stereo(&mut drums, drum_sig, drum_sig);
        write_stereo(&mut bass, lp_bass, lp_bass);
        write_stereo(&mut other, other_l, other_r);
    }

    let specs = [
        (StemKind::Vocals, "vocals.wav", vocals),
        (StemKind::Drums, "drums.wav", drums),
        (StemKind::Bass, "bass.wav", bass),
        (StemKind::Other, "other.wav", other),
    ];

    let mut assets = Vec::with_capacity(4);
    for (kind, fname, data) in specs {
        let out_path = out_dir.join(fname);
        write_wav_16bit(&out_path, sr, 2, &data)?;
        let (peak_db, rms_db, _) = measure_loudness(&data);
        assets.push(StemAsset {
            kind,
            path: out_path.to_string_lossy().to_string(),
            rms_db,
            peak_db,
            duration_secs,
        });
    }

    Ok(assets)
}

/// Map a musical key string to an approximate Camelot code for DJCraft handoff.
pub fn key_to_camelot(key: &str) -> &'static str {
    let k = key.trim().to_ascii_lowercase();
    if k.starts_with("c major") || k == "c" {
        "8B"
    } else if k.starts_with("a minor") || k == "am" {
        "8A"
    } else if k.starts_with("g major") || k == "g" {
        "9B"
    } else if k.starts_with("e minor") || k == "em" {
        "9A"
    } else if k.starts_with("d major") || k == "d" {
        "10B"
    } else if k.starts_with("f major") || k == "f" {
        "7B"
    } else if k.starts_with("d minor") || k == "dm" {
        "7A"
    } else {
        "8B"
    }
}

/// Export stems + master mix + `handoff.json` manifest for `djcraft`.
pub fn export_djcraft_handoff(project: &SongProject, out_dir: &Path) -> Result<DjCraftHandoff> {
    std::fs::create_dir_all(out_dir)?;
    let mix_path = out_dir.join("master_mix.wav");
    let report = render_project_mix(project, &mix_path)?;
    let stems_dir = out_dir.join("stems");
    let stems = split_stems(Some(&mix_path), &stems_dir, project)?;

    let handoff = DjCraftHandoff {
        schema: "musiccraft.djcraft.handoff.v1".to_string(),
        source_app: "musiccraft".to_string(),
        project_name: project.name.clone(),
        bpm: project.bpm,
        key: project.key.clone(),
        camelot: key_to_camelot(&project.key).to_string(),
        duration_secs: report.duration_secs,
        master_mix_path: report.out_path,
        stems,
    };

    let manifest_path = out_dir.join("handoff.json");
    let json = serde_json::to_string_pretty(&handoff)?;
    std::fs::write(&manifest_path, json)?;
    Ok(handoff)
}
