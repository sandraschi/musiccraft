//! Pure-Rust real-time audio transport and `cpal` playback engine for MusicCraft.
//! Falls back gracefully to a deterministic wall-clock playhead if no OS audio device is present.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

struct SharedAudio {
    samples: Vec<f32>,
    source_sr: u32,
}

/// Real-time transport controller shared between the `egui` UI and `cpal` output stream.
pub struct TransportPlayer {
    playing: Arc<AtomicBool>,
    looping: Arc<AtomicBool>,
    frame_pos: Arc<AtomicU64>,
    audio: Arc<Mutex<SharedAudio>>,
    _stream: Option<cpal::Stream>,
    has_audio_device: bool,
    last_tick: Instant,
    fallback_beat: f64,
}

impl TransportPlayer {
    pub fn new() -> Self {
        let playing = Arc::new(AtomicBool::new(false));
        let looping = Arc::new(AtomicBool::new(false));
        let frame_pos = Arc::new(AtomicU64::new(0));
        let audio = Arc::new(Mutex::new(SharedAudio {
            samples: Vec::new(),
            source_sr: 44_100,
        }));

        let stream = Self::try_open_stream(
            Arc::clone(&playing),
            Arc::clone(&looping),
            Arc::clone(&frame_pos),
            Arc::clone(&audio),
        );
        let has_audio_device = stream.is_some();

        Self {
            playing,
            looping,
            frame_pos,
            audio,
            _stream: stream,
            has_audio_device,
            last_tick: Instant::now(),
            fallback_beat: 0.0,
        }
    }

    fn try_open_stream(
        playing: Arc<AtomicBool>,
        looping: Arc<AtomicBool>,
        frame_pos: Arc<AtomicU64>,
        audio: Arc<Mutex<SharedAudio>>,
    ) -> Option<cpal::Stream> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let channels = usize::from(supported.channels().max(1));
        let out_sr = supported.sample_rate().0.max(8_000);
        let config: cpal::StreamConfig = supported.into();

        let err_fn = |err: cpal::StreamError| {
            eprintln!("[musiccraft] audio stream error: {err}");
        };

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !playing.load(Ordering::Relaxed) {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                        return;
                    }

                    let guard = audio.lock().unwrap_or_else(PoisonError::into_inner);
                    let total_frames = guard.samples.len() / 2;
                    if total_frames == 0 {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                        return;
                    }

                    let sr_ratio = f64::from(guard.source_sr) / f64::from(out_sr);
                    let mut pos = frame_pos.load(Ordering::Relaxed);
                    let is_loop = looping.load(Ordering::Relaxed);

                    for out_frame in data.chunks_mut(channels) {
                        let src_idx = ((pos as f64) * sr_ratio) as usize;
                        if src_idx >= total_frames {
                            if is_loop {
                                pos = 0;
                            } else {
                                playing.store(false, Ordering::Relaxed);
                                for s in out_frame.iter_mut() {
                                    *s = 0.0;
                                }
                                continue;
                            }
                        }
                        let idx = ((pos as f64) * sr_ratio) as usize;
                        let l = guard.samples.get(idx * 2).copied().unwrap_or(0.0);
                        let r = guard.samples.get(idx * 2 + 1).copied().unwrap_or(l);
                        if let Some(ch0) = out_frame.first_mut() {
                            *ch0 = l;
                        }
                        if let Some(ch1) = out_frame.get_mut(1) {
                            *ch1 = r;
                        }
                        for extra in out_frame.iter_mut().skip(2) {
                            *extra = 0.5 * (l + r);
                        }
                        pos = pos.saturating_add(1);
                    }
                    frame_pos.store(pos, Ordering::Relaxed);
                },
                err_fn,
                None,
            )
            .ok()?;

        if stream.play().is_err() {
            return None;
        }
        Some(stream)
    }

    /// Load newly rendered stereo mix samples into the transport buffer.
    pub fn load_buffer(&mut self, sample_rate: u32, samples: Vec<f32>) {
        let mut guard = self.audio.lock().unwrap_or_else(PoisonError::into_inner);
        guard.source_sr = sample_rate.max(8_000);
        guard.samples = samples;
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn is_looping(&self) -> bool {
        self.looping.load(Ordering::Relaxed)
    }

    pub fn set_looping(&self, loop_on: bool) {
        self.looping.store(loop_on, Ordering::Relaxed);
    }

    pub fn play(&mut self) {
        self.last_tick = Instant::now();
        self.playing.store(true, Ordering::Relaxed);
    }

    pub fn pause(&mut self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.playing.store(false, Ordering::Relaxed);
        self.frame_pos.store(0, Ordering::Relaxed);
        self.fallback_beat = 0.0;
    }

    pub fn rewind(&mut self) {
        self.frame_pos.store(0, Ordering::Relaxed);
        self.fallback_beat = 0.0;
        self.last_tick = Instant::now();
    }

    pub fn seek_beat(&mut self, beat: f64, bpm: f64) {
        let clamped_beat = beat.max(0.0);
        self.fallback_beat = clamped_beat;
        let secs = (clamped_beat * 60.0) / bpm.max(20.0);
        let sr = {
            let guard = self.audio.lock().unwrap_or_else(PoisonError::into_inner);
            guard.source_sr
        };
        let frame = (secs * f64::from(sr)).round() as u64;
        self.frame_pos.store(frame, Ordering::Relaxed);
        self.last_tick = Instant::now();
    }

    /// Advance fallback timer if no audio device is active, and return current playhead beat.
    pub fn tick_and_beat(&mut self, bpm: f64, total_beats: f64) -> f64 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        if self.has_audio_device {
            let sr = {
                let guard = self.audio.lock().unwrap_or_else(PoisonError::into_inner);
                guard.source_sr
            };
            let frames = self.frame_pos.load(Ordering::Relaxed);
            let secs = (frames as f64) / f64::from(sr.max(8_000));
            return (secs * bpm.max(20.0)) / 60.0;
        }

        if self.is_playing() {
            self.fallback_beat += (dt * bpm.max(20.0)) / 60.0;
            if self.fallback_beat >= total_beats {
                if self.is_looping() {
                    self.fallback_beat = 0.0;
                } else {
                    self.fallback_beat = 0.0;
                    self.playing.store(false, Ordering::Relaxed);
                }
            }
        }
        self.fallback_beat
    }
}
