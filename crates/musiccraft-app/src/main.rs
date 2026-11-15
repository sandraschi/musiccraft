//! MusicCraft L3 Pure-Rust Desktop Studio Workstation (`eframe` / `egui 0.37`).
//!
//! Pattern 2 Viewport-Centric DAW Workstation calling `musiccraft_core::dispatch_on_project`
//! directly in-process (zero HTTP sidecars, zero WebView2, zero child CLI processes).
//! `--smoke` executes headless parity verification without window initialization.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

mod player;
mod tofu_guard;

use eframe::egui;
use musiccraft_core::{
    command_catalog, compute_waveform_peaks, dispatch_on_project, key_to_camelot, read_wav_16bit,
    render_project_with_metronome, Command, CommandSpec, Region, SongProject, TrackFx, TrackKind,
    APP_NAME, APP_VERSION,
};
use serde_json::Value;
use std::path::PathBuf;

/// High-contrast slate-950 dark theme per EGUI_DESKTOP_UI_AND_PARITY.md.
fn harsh_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(10, 13, 18);
    v.window_fill = egui::Color32::from_rgb(16, 26, 46);
    v.extreme_bg_color = egui::Color32::from_rgb(6, 9, 14);
    v.faint_bg_color = egui::Color32::from_rgb(20, 30, 50);
    v.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(16, 26, 46);
    v.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 52, 82));
    v.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(245, 247, 250));
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(24, 37, 64);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 66, 102));
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(225, 232, 242));
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(38, 58, 98);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(96, 165, 250));
    v.widgets.active.bg_fill = egui::Color32::from_rgb(59, 130, 246);
    v.selection.bg_fill = egui::Color32::from_rgb(37, 99, 235);
    v
}

/// Craft Menu Bar Standard `switch_on_hover`: hovering another top menu while one is open switches cleanly.
fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let is_open =
        |t: &egui::Response| egui::Popup::is_id_open(ctx, egui::Popup::default_response_id(t));
    if !titles.iter().any(is_open) {
        return;
    }
    if let Some(title) = titles
        .iter()
        .find(|t| t.hovered() && !t.clicked() && !is_open(t))
    {
        egui::Popup::open_id(ctx, egui::Popup::default_response_id(title));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CenterTab {
    Timeline,
    PianoAndVocal,
    Stems,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpTab {
    GetStarted,
    Functions,
    Usage,
    TipsAndTricks,
    OtherCrafts,
}

struct MusicCraftApp {
    project: SongProject,
    undo_stack: Vec<SongProject>,
    redo_stack: Vec<SongProject>,
    current_path: Option<PathBuf>,
    recent_files: Vec<PathBuf>,
    dark_mode: bool,
    zoom_px_per_beat: f32,
    center_tab: CenterTab,

    // Panel visibility toggles (Window / View menus)
    show_toolbox: bool,
    show_timeline: bool,
    show_mixer: bool,
    show_vocal_inspector: bool,
    show_loudness_hud: bool,

    // Mandatory 5 Help dialogs + Preferences
    show_command_palette: bool,
    palette_query: String,
    show_shortcuts: bool,
    show_docs: bool,
    help_tab: HelpTab,
    show_licenses: bool,
    show_about: bool,
    show_preferences: bool,

    // Interactive studio inputs & transport
    player: player::TransportPlayer,
    audio_dirty: bool,
    playhead_beat: f64,
    snap_beats: f64,
    selected_track_id: Option<String>,
    selected_region_id: Option<String>,
    vocal_line_input: String,
    vocal_voice_input: String,
    new_track_name: String,
    last_status: String,
    last_json: Value,
}

impl MusicCraftApp {
    fn new() -> Self {
        let project = SongProject::new_demo();
        let selected_track_id = project.tracks.first().map(|t| t.id.clone());
        let selected_region_id = project
            .tracks
            .first()
            .and_then(|t| t.regions.first())
            .map(|r| r.id.clone());
        let vocal_line_input = project
            .vocal_line
            .as_ref()
            .map(|v| {
                v.phonemes
                    .iter()
                    .map(|p| p.phoneme.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| "aa ee oo ah".to_string());

        Self {
            project,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_path: None,
            recent_files: Vec::new(),
            dark_mode: true,
            zoom_px_per_beat: 48.0,
            center_tab: CenterTab::Timeline,
            show_toolbox: true,
            show_timeline: true,
            show_mixer: true,
            show_vocal_inspector: true,
            show_loudness_hud: true,
            show_command_palette: false,
            palette_query: String::new(),
            show_shortcuts: false,
            show_docs: false,
            help_tab: HelpTab::GetStarted,
            show_licenses: false,
            show_about: false,
            show_preferences: false,
            player: player::TransportPlayer::new(),
            audio_dirty: true,
            playhead_beat: 0.0,
            snap_beats: 0.5,
            selected_track_id,
            selected_region_id,
            vocal_line_input,
            vocal_voice_input: "OPERA".to_string(),
            new_track_name: "Synth Pad".to_string(),
            last_status: "Ready (Studio Demo Loaded)".to_string(),
            last_json: serde_json::json!({ "ok": true, "status": "ready" }),
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.project.clone());
        if self.undo_stack.len() > 32 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.project.clone());
            self.project = prev;
            self.audio_dirty = true;
            self.last_status = "Undo applied".to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.project.clone());
            self.project = next;
            self.audio_dirty = true;
            self.last_status = "Redo applied".to_string();
        }
    }

    fn toggle_play(&mut self) {
        if self.player.is_playing() {
            self.player.pause();
            self.last_status = "Transport: Paused".to_string();
        } else {
            if self.audio_dirty {
                let (sr, _, samples) =
                    render_project_with_metronome(&self.project, self.project.metronome_enabled);
                self.player.load_buffer(sr, samples);
                self.audio_dirty = false;
            }
            self.player.play();
            self.last_status = "Transport: Playing".to_string();
        }
    }

    fn run_cmd(&mut self, label: &str, cmd: Command) {
        self.push_undo();
        self.audio_dirty = true;
        let res = dispatch_on_project(&mut self.project, cmd);
        let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if ok {
            self.last_status = format!("{label}: OK");
        } else {
            let err = res
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            self.last_status = format!("{label}: {err}");
        }
        self.last_json = res;
    }

    fn default_out_dir(&self) -> PathBuf {
        std::env::temp_dir().join("musiccraft_studio_out")
    }

    fn execute_palette_spec(&mut self, spec: &CommandSpec) {
        let out_dir = self.default_out_dir();
        let cmd = match spec.id {
            "system.status" => Command::SystemStatus,
            "project.new" => Command::ProjectNew {
                name: "New Studio Song".to_string(),
                bpm: 120.0,
                key: "C major".to_string(),
                out_path: None,
            },
            "project.info" => Command::ProjectInfo { path: None },
            "track.add" => Command::TrackAdd {
                project_path: None,
                name: self.new_track_name.clone(),
                kind: "midi".to_string(),
            },
            "track.list" => Command::TrackList { project_path: None },
            "modify.quantize" => Command::ModifyQuantize {
                project_path: None,
                track_id: None,
                grid_beats: 0.5,
            },
            "modify.transpose" => Command::ModifyTranspose {
                project_path: None,
                semitones: 1,
                include_vocal: true,
            },
            "vocal.synth" => Command::VocalSynth {
                project_path: None,
                line: self.vocal_line_input.clone(),
                voice: self.vocal_voice_input.clone(),
                bpm: Some(self.project.bpm),
                base_midi: Some(69),
                out_wav: Some(out_dir.join("vocal.wav").to_string_lossy().to_string()),
            },
            "stem.split" => Command::StemSplit {
                project_path: None,
                in_wav: None,
                out_dir: out_dir.join("stems").to_string_lossy().to_string(),
            },
            "mix.render" => Command::MixRender {
                project_path: None,
                out_wav: out_dir.join("master_mix.wav").to_string_lossy().to_string(),
            },
            "export.stems" => Command::ExportStems {
                project_path: None,
                out_dir: out_dir
                    .join("djcraft_handoff")
                    .to_string_lossy()
                    .to_string(),
            },
            "export.song" => Command::ExportSong {
                project_path: None,
                out_dir: out_dir.join("song_bundle").to_string_lossy().to_string(),
            },
            _ => Command::SystemStatus,
        };
        self.run_cmd(spec.label, cmd);
    }

    fn reset_layout(&mut self) {
        self.show_toolbox = true;
        self.show_timeline = true;
        self.show_mixer = true;
        self.show_vocal_inspector = true;
        self.show_loudness_hud = true;
        self.zoom_px_per_beat = 48.0;
        self.center_tab = CenterTab::Timeline;
        self.last_status = "Panel layout reset".to_string();
    }

    fn snap_beat(&self, beat: f64) -> f64 {
        if self.snap_beats <= 0.0 {
            beat.max(0.0)
        } else {
            ((beat / self.snap_beats).round() * self.snap_beats).max(0.0)
        }
    }

    fn import_audio_wav(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .pick_file()
        {
            match read_wav_16bit(&path) {
                Ok(buf) => {
                    self.push_undo();
                    let beats_per_sec = self.project.bpm / 60.0;
                    let frames = buf.samples.len() / usize::from(buf.channels.max(1));
                    let duration_secs = frames as f64 / f64::from(buf.sample_rate.max(1));
                    let length_beats = (duration_secs * beats_per_sec).max(1.0);
                    let peaks = compute_waveform_peaks(&buf.samples, buf.channels, 96);
                    let file_stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Audio Clip")
                        .to_string();

                    let target_track_id = if let Some(tid) = self.selected_track_id.clone() {
                        tid
                    } else {
                        let res = dispatch_on_project(
                            &mut self.project,
                            Command::TrackAdd {
                                project_path: None,
                                name: file_stem.clone(),
                                kind: "audio".to_string(),
                            },
                        );
                        res.get("track")
                            .and_then(|t| t.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("trk-audio")
                            .to_string()
                    };

                    let reg_id = format!("reg-wav-{}", self.project.tracks.len() + 1);
                    let region = Region {
                        id: reg_id.clone(),
                        name: file_stem,
                        start_beat: self.snap_beat(self.playhead_beat),
                        length_beats,
                        notes: Vec::new(),
                        audio_ref: Some(path.to_string_lossy().to_string()),
                        waveform_peaks: peaks,
                        gain_db: 0.0,
                    };
                    if let Err(e) = self.project.add_region(&target_track_id, region) {
                        self.last_status = format!("Import WAV failed: {e}");
                    } else {
                        self.selected_track_id = Some(target_track_id);
                        self.selected_region_id = Some(reg_id);
                        self.audio_dirty = true;
                        self.last_status = format!("Imported WAV: {}", path.display());
                    }
                }
                Err(e) => {
                    self.last_status = format!("Import WAV error: {e}");
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_kb = ctx.egui_wants_keyboard_input();
        let mut dup_sel = false;
        let mut del_sel = false;
        let mut split_sel = false;
        let mut add_mk = false;
        ctx.input(|i| {
            if !wants_kb && i.key_pressed(egui::Key::Space) {
                self.toggle_play();
            }
            if !wants_kb && i.key_pressed(egui::Key::Home) {
                self.player.rewind();
                self.playhead_beat = 0.0;
                self.last_status = "Transport: Rewound to Bar 1".to_string();
            }
            if !wants_kb && i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                dup_sel = true;
            }
            if !wants_kb
                && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            {
                del_sel = true;
            }
            if !wants_kb && i.key_pressed(egui::Key::S) && !i.modifiers.ctrl {
                split_sel = true;
            }
            if !wants_kb && i.key_pressed(egui::Key::M) && !i.modifiers.ctrl {
                add_mk = true;
            }
            if (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::P))
                || (i.modifiers.command && i.key_pressed(egui::Key::K))
            {
                self.show_command_palette = !self.show_command_palette;
            }
            if i.key_pressed(egui::Key::F1)
                || (i.modifiers.command && i.key_pressed(egui::Key::Slash))
            {
                self.show_shortcuts = !self.show_shortcuts;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Z) {
                self.undo();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Y) {
                self.redo();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Equals) {
                self.zoom_px_per_beat = (self.zoom_px_per_beat * 1.15).min(160.0);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Minus) {
                self.zoom_px_per_beat = (self.zoom_px_per_beat / 1.15).max(16.0);
            }
        });

        if dup_sel {
            if let Some(rid) = self.selected_region_id.clone() {
                self.run_cmd(
                    "Duplicate Region",
                    Command::RegionDuplicate {
                        project_path: None,
                        region_id: rid,
                    },
                );
            }
        }
        if del_sel {
            if let Some(rid) = self.selected_region_id.clone() {
                self.run_cmd(
                    "Delete Region",
                    Command::RegionDelete {
                        project_path: None,
                        region_id: rid,
                    },
                );
                self.selected_region_id = None;
            }
        }
        if split_sel {
            if let Some(rid) = self.selected_region_id.clone() {
                self.run_cmd(
                    "Split Region at Playhead",
                    Command::RegionSplit {
                        project_path: None,
                        region_id: rid,
                        split_beat: self.playhead_beat,
                    },
                );
            }
        }
        if add_mk {
            let name = format!("Marker {}", self.project.markers.len() + 1);
            self.run_cmd(
                "Add Marker",
                Command::MarkerAdd {
                    project_path: None,
                    name,
                    beat: self.snap_beat(self.playhead_beat),
                },
            );
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut menu_titles: Vec<egui::Response> = Vec::with_capacity(8);

        egui::MenuBar::new().ui(ui, |ui| {
            // 1. File
            let r_file = ui.menu_button("File", |ui| {
                if ui.button("New... (Ctrl+N)").clicked() {
                    self.run_cmd(
                        "New Project",
                        Command::ProjectNew {
                            name: "Untitled Song".to_string(),
                            bpm: 120.0,
                            key: "C major".to_string(),
                            out_path: None,
                        },
                    );
                    self.current_path = None;
                    ui.close();
                }
                if ui.button("Open... (Ctrl+O)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("MusicCraft Project", &["mcraft", "json"])
                        .pick_file()
                    {
                        let p_str = path.to_string_lossy().to_string();
                        self.run_cmd("Open Project", Command::ProjectOpen { path: p_str });
                        self.current_path = Some(path.clone());
                        if !self.recent_files.contains(&path) {
                            self.recent_files.insert(0, path);
                        }
                    }
                    ui.close();
                }
                ui.menu_button("Open Recent", |ui| {
                    if self.recent_files.is_empty() {
                        ui.label("(No recent projects)");
                    } else {
                        let recents = self.recent_files.clone();
                        for path in recents {
                            let label = path.to_string_lossy().to_string();
                            if ui.button(&label).clicked() {
                                self.run_cmd("Open Recent", Command::ProjectOpen { path: label });
                                self.current_path = Some(path);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Clear Recent Files").clicked() {
                            self.recent_files.clear();
                            ui.close();
                        }
                    }
                });
                if ui.button("Save (Ctrl+S)").clicked() {
                    let target = self
                        .current_path
                        .clone()
                        .unwrap_or_else(|| self.default_out_dir().join("project.mcraft"));
                    self.run_cmd(
                        "Save Project",
                        Command::ProjectSave {
                            path: target.to_string_lossy().to_string(),
                        },
                    );
                    self.current_path = Some(target);
                    ui.close();
                }
                if ui.button("Save As... (Ctrl+Shift+S)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("MusicCraft Project", &["mcraft"])
                        .save_file()
                    {
                        self.run_cmd(
                            "Save As",
                            Command::ProjectSave {
                                path: path.to_string_lossy().to_string(),
                            },
                        );
                        self.current_path = Some(path);
                    }
                    ui.close();
                }
                ui.separator();
                if ui.button("Export Mix / Stems... (Ctrl+E)").clicked() {
                    let dir = self.default_out_dir().join("djcraft_handoff");
                    self.run_cmd(
                        "Export DJCraft Handoff",
                        Command::ExportStems {
                            project_path: None,
                            out_dir: dir.to_string_lossy().to_string(),
                        },
                    );
                    ui.close();
                }
                ui.separator();
                if ui.button("Close Project (Ctrl+W)").clicked() {
                    self.project = SongProject::new_demo();
                    self.current_path = None;
                    self.last_status = "Project closed (reset to demo)".to_string();
                    ui.close();
                }
                if ui.button("Quit (Ctrl+Q)").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            menu_titles.push(r_file.response);

            // 2. Edit
            let r_edit = ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        !self.undo_stack.is_empty(),
                        egui::Button::new("Undo (Ctrl+Z)"),
                    )
                    .clicked()
                {
                    self.undo();
                    ui.close();
                }
                if ui
                    .add_enabled(
                        !self.redo_stack.is_empty(),
                        egui::Button::new("Redo (Ctrl+Y)"),
                    )
                    .clicked()
                {
                    self.redo();
                    ui.close();
                }
                ui.separator();
                if ui.button("Select First Track").clicked() {
                    self.selected_track_id = self.project.tracks.first().map(|t| t.id.clone());
                    ui.close();
                }
                ui.separator();
                if ui.button("Preferences... (Ctrl+,)").clicked() {
                    self.show_preferences = true;
                    ui.close();
                }
            });
            menu_titles.push(r_edit.response);

            // 3. View
            let r_view = ui.menu_button("View", |ui| {
                if ui.button("Zoom In (Ctrl+=)").clicked() {
                    self.zoom_px_per_beat = (self.zoom_px_per_beat * 1.2).min(160.0);
                    ui.close();
                }
                if ui.button("Zoom Out (Ctrl+-)").clicked() {
                    self.zoom_px_per_beat = (self.zoom_px_per_beat / 1.2).max(16.0);
                    ui.close();
                }
                if ui.button("Reset Zoom (100%)").clicked() {
                    self.zoom_px_per_beat = 48.0;
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut self.show_timeline, "Multi-Track Timeline");
                ui.checkbox(&mut self.show_mixer, "Mixer Console");
                if ui.button("Piano Roll / Vocal Curve").clicked() {
                    self.center_tab = CenterTab::PianoAndVocal;
                    ui.close();
                }
                if ui.button("Stem Separator View").clicked() {
                    self.center_tab = CenterTab::Stems;
                    ui.close();
                }
                ui.checkbox(&mut self.show_loudness_hud, "Master Loudness HUD");
                ui.separator();
                ui.menu_button("Interface Theme", |ui| {
                    if ui
                        .selectable_label(self.dark_mode, "Dark (Slate-950)")
                        .clicked()
                    {
                        self.dark_mode = true;
                        ctx.set_visuals(harsh_visuals());
                        ui.close();
                    }
                    if ui.selectable_label(!self.dark_mode, "Light").clicked() {
                        self.dark_mode = false;
                        ctx.set_visuals(egui::Visuals::light());
                        ui.close();
                    }
                });
            });
            menu_titles.push(r_view.response);

            // 4. Insert
            let r_insert = ui.menu_button("Insert", |ui| {
                if ui.button("Add Audio Track").clicked() {
                    self.run_cmd(
                        "Add Audio Track",
                        Command::TrackAdd {
                            project_path: None,
                            name: format!("Audio {}", self.project.tracks.len() + 1),
                            kind: "audio".to_string(),
                        },
                    );
                    ui.close();
                }
                if ui.button("Add MIDI Track").clicked() {
                    self.run_cmd(
                        "Add MIDI Track",
                        Command::TrackAdd {
                            project_path: None,
                            name: format!("MIDI {}", self.project.tracks.len() + 1),
                            kind: "midi".to_string(),
                        },
                    );
                    ui.close();
                }
                if ui.button("Insert Virtual Vocal Line").clicked() {
                    self.run_cmd(
                        "Synthesize Vocal Line",
                        Command::VocalSynth {
                            project_path: None,
                            line: self.vocal_line_input.clone(),
                            voice: self.vocal_voice_input.clone(),
                            bpm: Some(self.project.bpm),
                            base_midi: Some(69),
                            out_wav: None,
                        },
                    );
                    self.center_tab = CenterTab::PianoAndVocal;
                    ui.close();
                }
                if ui.button("Add Region to Selected Track").clicked() {
                    if let Some(tid) = self.selected_track_id.clone() {
                        self.run_cmd(
                            "Add Region",
                            Command::RegionAdd {
                                project_path: None,
                                track_id: tid,
                                name: "New Clip".to_string(),
                                start_beat: self.snap_beat(self.playhead_beat),
                                length_beats: 4.0,
                            },
                        );
                    }
                    ui.close();
                }
                if ui.button("Import Audio File (.wav)...").clicked() {
                    self.import_audio_wav();
                    ui.close();
                }
                if ui.button("Add Marker at Playhead (M)").clicked() {
                    let name = format!("Marker {}", self.project.markers.len() + 1);
                    self.run_cmd(
                        "Add Marker",
                        Command::MarkerAdd {
                            project_path: None,
                            name,
                            beat: self.snap_beat(self.playhead_beat),
                        },
                    );
                    ui.close();
                }
            });
            menu_titles.push(r_insert.response);

            // 5. Modify
            let r_modify = ui.menu_button("Modify", |ui| {
                if ui.button("Quantize Notes (1/2 Beat)").clicked() {
                    self.run_cmd(
                        "Quantize Notes",
                        Command::ModifyQuantize {
                            project_path: None,
                            track_id: None,
                            grid_beats: 0.5,
                        },
                    );
                    ui.close();
                }
                if ui.button("Transpose Pitch (+1 Semitone)").clicked() {
                    self.run_cmd(
                        "Transpose +1",
                        Command::ModifyTranspose {
                            project_path: None,
                            semitones: 1,
                            include_vocal: true,
                        },
                    );
                    ui.close();
                }
                if ui.button("Transpose Pitch (-1 Semitone)").clicked() {
                    self.run_cmd(
                        "Transpose -1",
                        Command::ModifyTranspose {
                            project_path: None,
                            semitones: -1,
                            include_vocal: true,
                        },
                    );
                    ui.close();
                }
                ui.separator();
                if ui.button("Duplicate Selected Region (Ctrl+D)").clicked() {
                    if let Some(rid) = self.selected_region_id.clone() {
                        self.run_cmd(
                            "Duplicate Region",
                            Command::RegionDuplicate {
                                project_path: None,
                                region_id: rid,
                            },
                        );
                    }
                    ui.close();
                }
                if ui.button("Split Selected Region at Playhead (S)").clicked() {
                    if let Some(rid) = self.selected_region_id.clone() {
                        self.run_cmd(
                            "Split Region",
                            Command::RegionSplit {
                                project_path: None,
                                region_id: rid,
                                split_beat: self.playhead_beat,
                            },
                        );
                    }
                    ui.close();
                }
                if ui.button("Delete Selected Region (Del)").clicked() {
                    if let Some(rid) = self.selected_region_id.clone() {
                        self.run_cmd(
                            "Delete Region",
                            Command::RegionDelete {
                                project_path: None,
                                region_id: rid,
                            },
                        );
                        self.selected_region_id = None;
                    }
                    ui.close();
                }
                if ui.button("Split Selected Region").clicked() {
                    if let Some(rid) = self.selected_region_id.clone() {
                        self.run_cmd(
                            "Split Region",
                            Command::RegionSplit {
                                project_path: None,
                                region_id: rid,
                                split_beat: 4.0,
                            },
                        );
                    }
                    ui.close();
                }
            });
            menu_titles.push(r_modify.response);

            // 6. Commands
            let r_commands = ui.menu_button("Commands", |ui| {
                let out_dir = self.default_out_dir();
                if ui.button("Render Master Mix").clicked() {
                    self.run_cmd(
                        "Render Master Mix",
                        Command::MixRender {
                            project_path: None,
                            out_wav: out_dir.join("master_mix.wav").to_string_lossy().to_string(),
                        },
                    );
                    ui.close();
                }
                if ui.button("Run Stem Separation").clicked() {
                    self.run_cmd(
                        "Stem Separation",
                        Command::StemSplit {
                            project_path: None,
                            in_wav: None,
                            out_dir: out_dir.join("stems").to_string_lossy().to_string(),
                        },
                    );
                    self.center_tab = CenterTab::Stems;
                    ui.close();
                }
                if ui.button("Synthesize Vocal Line").clicked() {
                    self.run_cmd(
                        "Synthesize Vocal",
                        Command::VocalSynth {
                            project_path: None,
                            line: self.vocal_line_input.clone(),
                            voice: self.vocal_voice_input.clone(),
                            bpm: Some(self.project.bpm),
                            base_midi: Some(69),
                            out_wav: Some(out_dir.join("vocal.wav").to_string_lossy().to_string()),
                        },
                    );
                    self.center_tab = CenterTab::PianoAndVocal;
                    ui.close();
                }
                ui.separator();
                if ui.button("Export Handoff Package to DJCraft").clicked() {
                    self.run_cmd(
                        "Export DJCraft Handoff",
                        Command::ExportStems {
                            project_path: None,
                            out_dir: out_dir
                                .join("djcraft_handoff")
                                .to_string_lossy()
                                .to_string(),
                        },
                    );
                    ui.close();
                }
            });
            menu_titles.push(r_commands.response);

            // 7. Window
            let r_window = ui.menu_button("Window", |ui| {
                if ui.button("Reset Panel Layout").clicked() {
                    self.reset_layout();
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut self.show_toolbox, "Toolbox");
                ui.checkbox(&mut self.show_timeline, "Arrangement Viewport");
                ui.checkbox(&mut self.show_mixer, "Mixer Drawer");
                ui.checkbox(&mut self.show_vocal_inspector, "Vocal Inspector");
                ui.checkbox(&mut self.show_loudness_hud, "Loudness HUD");
            });
            menu_titles.push(r_window.response);

            // 8. Help (Mandatory 5-Item Transparency Suite)
            let r_help = ui.menu_button("Help", |ui| {
                if ui.button("Command Palette... (Ctrl+Shift+P)").clicked() {
                    self.show_command_palette = true;
                    ui.close();
                }
                if ui.button("Keyboard Shortcuts... (F1)").clicked() {
                    self.show_shortcuts = true;
                    ui.close();
                }
                if ui.button("Project Files & Docs...").clicked() {
                    self.show_docs = true;
                    ui.close();
                }
                if ui.button("Licenses & Notices...").clicked() {
                    self.show_licenses = true;
                    ui.close();
                }
                if ui.button("About MusicCraft").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
            menu_titles.push(r_help.response);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(74, 222, 128),
                    "Safe Rust (0 unsafe)",
                );
            });
        });

        switch_on_hover(ctx, &menu_titles);
    }

    fn render_quick_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Transport Controls (Rewind, Play/Pause, Stop, Loop, Position)
            if ui
                .button("|< Rewind")
                .on_hover_text("Rewind to Bar 1 (Home)")
                .clicked()
            {
                self.player.rewind();
                self.playhead_beat = 0.0;
                self.last_status = "Transport: Rewound".to_string();
            }
            let is_playing = self.player.is_playing();
            let play_label = if is_playing { "Pause" } else { "Play" };
            let play_btn = if is_playing {
                egui::Button::new(egui::RichText::new(play_label).color(egui::Color32::BLACK))
                    .fill(egui::Color32::from_rgb(74, 222, 128))
            } else {
                egui::Button::new(play_label)
            };
            if ui
                .add(play_btn)
                .on_hover_text("Play / Pause (Space)")
                .clicked()
            {
                self.toggle_play();
            }
            if ui
                .button("Stop")
                .on_hover_text("Stop and return to Bar 1")
                .clicked()
            {
                self.player.stop();
                self.playhead_beat = 0.0;
                self.last_status = "Transport: Stopped".to_string();
            }
            let mut looping = self.player.is_looping();
            if ui.toggle_value(&mut looping, "Loop").clicked() {
                self.player.set_looping(looping);
            }
            if ui
                .toggle_value(&mut self.project.metronome_enabled, "Click")
                .on_hover_text("Toggle Metronome Click")
                .clicked()
            {
                self.audio_dirty = true;
            }
            egui::ComboBox::from_id_salt("snap_combo")
                .width(64.0)
                .selected_text(match self.snap_beats {
                    x if x <= 0.0 => "Snap: Off",
                    x if (x - 0.25).abs() < 0.01 => "Snap: 1/4",
                    x if (x - 0.5).abs() < 0.01 => "Snap: 1/2",
                    x if (x - 1.0).abs() < 0.01 => "Snap: 1B",
                    _ => "Snap: Bar",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.snap_beats, 0.0, "Off");
                    ui.selectable_value(&mut self.snap_beats, 0.25, "1/4 Beat");
                    ui.selectable_value(&mut self.snap_beats, 0.5, "1/2 Beat");
                    ui.selectable_value(&mut self.snap_beats, 1.0, "1 Beat");
                    ui.selectable_value(&mut self.snap_beats, 4.0, "1 Bar");
                });
            let bar = (self.playhead_beat / 4.0).floor() as i32 + 1;
            let beat_in_bar = (self.playhead_beat % 4.0) + 1.0;
            let secs = (self.playhead_beat * 60.0) / self.project.bpm.max(20.0);
            let mins = (secs / 60.0).floor() as u32;
            let rem_s = secs % 60.0;
            ui.monospace(format!("B{bar}.{beat_in_bar:.1} | {mins:02}:{rem_s:04.1}"));
            ui.separator();

            ui.strong(&self.project.name);
            ui.separator();
            ui.label(format!(
                "{:.1} BPM | {} (Camelot {})",
                self.project.bpm,
                self.project.key,
                key_to_camelot(&self.project.key)
            ));
            ui.separator();

            if ui
                .selectable_label(self.center_tab == CenterTab::Timeline, "Timeline")
                .clicked()
            {
                self.center_tab = CenterTab::Timeline;
            }
            if ui
                .selectable_label(
                    self.center_tab == CenterTab::PianoAndVocal,
                    "Piano Roll & Vocal",
                )
                .clicked()
            {
                self.center_tab = CenterTab::PianoAndVocal;
            }
            if ui
                .selectable_label(self.center_tab == CenterTab::Stems, "Stems")
                .clicked()
            {
                self.center_tab = CenterTab::Stems;
            }

            ui.separator();
            let out_dir = self.default_out_dir();
            if ui.button("Render Mix").clicked() {
                self.run_cmd(
                    "Render Master Mix",
                    Command::MixRender {
                        project_path: None,
                        out_wav: out_dir.join("master_mix.wav").to_string_lossy().to_string(),
                    },
                );
            }
            if ui.button("Synth Vocal").clicked() {
                self.run_cmd(
                    "Synthesize Vocal",
                    Command::VocalSynth {
                        project_path: None,
                        line: self.vocal_line_input.clone(),
                        voice: self.vocal_voice_input.clone(),
                        bpm: Some(self.project.bpm),
                        base_midi: Some(69),
                        out_wav: Some(out_dir.join("vocal.wav").to_string_lossy().to_string()),
                    },
                );
            }
            if ui.button("Split Stems").clicked() {
                self.run_cmd(
                    "Split Stems",
                    Command::StemSplit {
                        project_path: None,
                        in_wav: None,
                        out_dir: out_dir.join("stems").to_string_lossy().to_string(),
                    },
                );
                self.center_tab = CenterTab::Stems;
            }
            if ui.button("Export to DJCraft").clicked() {
                self.run_cmd(
                    "Export DJCraft Handoff",
                    Command::ExportStems {
                        project_path: None,
                        out_dir: out_dir
                            .join("djcraft_handoff")
                            .to_string_lossy()
                            .to_string(),
                    },
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Vector-drawn theme toggle button (no emoji -> zero tofu)
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(26.0, 22.0), egui::Sense::click());
                if resp.clicked() {
                    self.dark_mode = !self.dark_mode;
                    if self.dark_mode {
                        ctx.set_visuals(harsh_visuals());
                    } else {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                }
                let center = rect.center();
                let color = if self.dark_mode {
                    egui::Color32::from_rgb(250, 204, 21)
                } else {
                    egui::Color32::from_rgb(30, 41, 59)
                };
                ui.painter().circle_filled(center, 6.0, color);
                if self.dark_mode {
                    for k in 0..8 {
                        let a = (k as f32) * std::f32::consts::FRAC_PI_4;
                        let p1 = center + egui::vec2(a.cos(), a.sin()) * 8.0;
                        let p2 = center + egui::vec2(a.cos(), a.sin()) * 10.5;
                        ui.painter()
                            .line_segment([p1, p2], egui::Stroke::new(1.5, color));
                    }
                }
                resp.on_hover_text("Toggle Dark / Light Theme");
            });
        });
    }

    fn render_timeline_viewport(&mut self, ui: &mut egui::Ui) {
        ui.heading("Multi-Track Arrangement Stage");
        ui.add_space(4.0);

        let px_per_beat = self.zoom_px_per_beat;
        let total_beats = self.project.total_beats().max(16.0);
        let header_w = 170.0_f32;
        let lane_h = 54.0_f32;

        egui::ScrollArea::both().show(ui, |ui| {
            // Beat ruler (click or drag to scrub playhead)
            let ruler_w = (total_beats as f32) * px_per_beat;
            let (ruler_rect, ruler_resp) = ui.allocate_exact_size(
                egui::vec2(header_w + ruler_w, 24.0),
                egui::Sense::click_and_drag(),
            );
            if ruler_resp.clicked() || ruler_resp.dragged() {
                if let Some(pos) = ruler_resp.interact_pointer_pos() {
                    let local_x = (pos.x - (ruler_rect.left() + header_w)).max(0.0);
                    let beat = f64::from(local_x / px_per_beat).min(total_beats);
                    self.player.seek_beat(beat, self.project.bpm);
                    self.playhead_beat = beat;
                }
            }
            let painter = ui.painter();
            painter.rect_filled(ruler_rect, 2.0, egui::Color32::from_rgb(15, 23, 42));
            let beat_count = total_beats.ceil() as i32;
            for b in 0..=beat_count {
                let x = ruler_rect.left() + header_w + (b as f32) * px_per_beat;
                let is_bar = b % 4 == 0;
                let stroke = if is_bar {
                    egui::Stroke::new(1.2, egui::Color32::from_rgb(148, 163, 184))
                } else {
                    egui::Stroke::new(0.6, egui::Color32::from_rgb(51, 65, 85))
                };
                painter.line_segment(
                    [
                        egui::pos2(x, ruler_rect.top()),
                        egui::pos2(x, ruler_rect.bottom()),
                    ],
                    stroke,
                );
                if is_bar {
                    painter.text(
                        egui::pos2(x + 4.0, ruler_rect.top() + 4.0),
                        egui::Align2::LEFT_TOP,
                        format!("Bar {}", b / 4 + 1),
                        egui::FontId::monospace(11.0),
                        egui::Color32::from_rgb(203, 213, 225),
                    );
                }
            }
            let ph_ruler_x =
                ruler_rect.left() + header_w + (self.playhead_beat as f32) * px_per_beat;
            // Loop range highlight bar on ruler
            let ls = self.project.loop_start_beat;
            let le = self.project.loop_end_beat;
            if le > ls {
                let lx0 = ruler_rect.left() + header_w + (ls as f32) * px_per_beat;
                let lx1 = ruler_rect.left() + header_w + (le as f32) * px_per_beat;
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(lx0, ruler_rect.bottom() - 4.0),
                        egui::pos2(lx1, ruler_rect.bottom()),
                    ),
                    1.0,
                    egui::Color32::from_rgb(56, 189, 248),
                );
            }
            // Timeline markers
            for mk in &self.project.markers {
                let mx = ruler_rect.left() + header_w + (mk.beat as f32) * px_per_beat;
                if mx >= ruler_rect.left() + header_w && mx <= ruler_rect.right() {
                    painter.line_segment(
                        [
                            egui::pos2(mx, ruler_rect.top()),
                            egui::pos2(mx, ruler_rect.bottom()),
                        ],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(244, 114, 182)),
                    );
                    painter.text(
                        egui::pos2(mx + 3.0, ruler_rect.bottom() - 12.0),
                        egui::Align2::LEFT_TOP,
                        &mk.name,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(244, 114, 182),
                    );
                }
            }
            if ph_ruler_x >= ruler_rect.left() + header_w && ph_ruler_x <= ruler_rect.right() {
                painter.line_segment(
                    [
                        egui::pos2(ph_ruler_x, ruler_rect.top()),
                        egui::pos2(ph_ruler_x, ruler_rect.bottom()),
                    ],
                    egui::Stroke::new(2.5, egui::Color32::from_rgb(251, 191, 36)),
                );
            }

            ui.add_space(4.0);

            let snap = self.snap_beats;
            let mut open_piano = false;
            let mut dirty = false;

            for trk in &mut self.project.tracks {
                let is_sel = self.selected_track_id.as_deref() == Some(&trk.id);
                let row_res = ui.horizontal(|ui| {
                    // Track header card
                    ui.allocate_ui(egui::vec2(header_w - 6.0, lane_h), |ui| {
                        let frame_stroke = if is_sel {
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(96, 165, 250))
                        } else {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 52, 82))
                        };
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(16, 26, 46))
                            .stroke(frame_stroke)
                            .corner_radius(4.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(is_sel, &trk.name).clicked() {
                                        self.selected_track_id = Some(trk.id.clone());
                                    }
                                });
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} | {:.1} dB{} | FX:{}",
                                        trk.kind.as_str(),
                                        trk.volume_db,
                                        if trk.muted { " [M]" } else { "" },
                                        trk.fx.len()
                                    ))
                                    .small(),
                                );
                            });
                    });

                    // Track lane canvas
                    let (lane_rect, _) =
                        ui.allocate_exact_size(egui::vec2(ruler_w, lane_h), egui::Sense::click());
                    {
                        let p = ui.painter();
                        p.rect_filled(lane_rect, 3.0, egui::Color32::from_rgb(12, 18, 32));
                        p.rect_stroke(
                            lane_rect,
                            3.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 41, 59)),
                            egui::StrokeKind::Inside,
                        );
                    }

                    let base_color = match trk.kind {
                        TrackKind::Midi => egui::Color32::from_rgb(37, 99, 235),
                        TrackKind::Vocal => egui::Color32::from_rgb(168, 85, 247),
                        TrackKind::Audio => egui::Color32::from_rgb(16, 185, 129),
                        TrackKind::Stem => egui::Color32::from_rgb(245, 158, 11),
                    };

                    for reg in &mut trk.regions {
                        let rx = lane_rect.left() + (reg.start_beat as f32) * px_per_beat;
                        let rw = ((reg.length_beats as f32) * px_per_beat).max(20.0);
                        let reg_rect = egui::Rect::from_min_size(
                            egui::pos2(rx, lane_rect.top() + 4.0),
                            egui::vec2(rw, lane_h - 8.0),
                        );

                        // Interactive region body (click to select, drag to move, drag right edge to resize)
                        let reg_resp = ui.interact(
                            reg_rect,
                            ui.scope_id().with(("reg", &trk.id, &reg.id)),
                            egui::Sense::click_and_drag(),
                        );
                        if reg_resp.clicked() || reg_resp.drag_started() {
                            self.selected_track_id = Some(trk.id.clone());
                            self.selected_region_id = Some(reg.id.clone());
                        }
                        if reg_resp.double_clicked() {
                            self.selected_track_id = Some(trk.id.clone());
                            self.selected_region_id = Some(reg.id.clone());
                            open_piano = true;
                        }
                        if reg_resp.dragged() {
                            let delta_beats = f64::from(reg_resp.drag_delta().x / px_per_beat);
                            let near_right = reg_resp
                                .interact_pointer_pos()
                                .map(|pt| (reg_rect.right() - pt.x).abs() <= 12.0)
                                .unwrap_or(false);
                            if near_right {
                                reg.length_beats = (reg.length_beats + delta_beats).max(0.5);
                            } else {
                                reg.start_beat = (reg.start_beat + delta_beats).max(0.0);
                            }
                            dirty = true;
                        }
                        if reg_resp.drag_stopped() && snap > 0.0 {
                            reg.start_beat = ((reg.start_beat / snap).round() * snap).max(0.0);
                            reg.length_beats = ((reg.length_beats / snap).round() * snap).max(0.5);
                            dirty = true;
                        }

                        let p = ui.painter();
                        let is_reg_sel = self.selected_region_id.as_deref() == Some(&reg.id);
                        p.rect_filled(reg_rect, 4.0, base_color.linear_multiply(0.35));
                        p.rect_stroke(
                            reg_rect,
                            4.0,
                            egui::Stroke::new(
                                if is_reg_sel { 2.0 } else { 1.2 },
                                if is_reg_sel {
                                    egui::Color32::from_rgb(250, 204, 21)
                                } else {
                                    base_color
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        // Resize handle indicator on right edge
                        p.line_segment(
                            [
                                egui::pos2(reg_rect.right() - 3.0, reg_rect.top() + 8.0),
                                egui::pos2(reg_rect.right() - 3.0, reg_rect.bottom() - 8.0),
                            ],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(203, 213, 225)),
                        );
                        p.text(
                            reg_rect.min + egui::vec2(6.0, 4.0),
                            egui::Align2::LEFT_TOP,
                            &reg.name,
                            egui::FontId::proportional(11.5),
                            egui::Color32::WHITE,
                        );

                        // Real waveform peaks inside audio regions
                        if !reg.waveform_peaks.is_empty() {
                            let n_peaks = reg.waveform_peaks.len();
                            let mid_y = reg_rect.center().y + 4.0;
                            let half_h = (reg_rect.height() - 22.0) * 0.5;
                            for (idx, &pk) in reg.waveform_peaks.iter().enumerate() {
                                let t = (idx as f32 + 0.5) / (n_peaks as f32);
                                let wx = reg_rect.left() + 4.0 + t * (reg_rect.width() - 8.0);
                                let amp = pk.clamp(0.05, 1.0) * half_h;
                                p.line_segment(
                                    [egui::pos2(wx, mid_y - amp), egui::pos2(wx, mid_y + amp)],
                                    egui::Stroke::new(1.2, egui::Color32::from_rgb(167, 243, 208)),
                                );
                            }
                        }

                        // Mini MIDI note bars inside region
                        for n in &reg.notes {
                            let nx = reg_rect.left() + (n.start_beat as f32) * px_per_beat;
                            let nw = ((n.duration_beats as f32) * px_per_beat).max(4.0);
                            let norm_y = 1.0 - ((f32::from(n.pitch) - 36.0) / 48.0).clamp(0.0, 1.0);
                            let ny = reg_rect.top() + 18.0 + norm_y * (reg_rect.height() - 24.0);
                            p.line_segment(
                                [egui::pos2(nx, ny), egui::pos2(nx + nw, ny)],
                                egui::Stroke::new(2.5, egui::Color32::from_rgb(248, 250, 252)),
                            );
                        }
                    }

                    // Live moving playhead line across this track lane
                    let ph_x = lane_rect.left() + (self.playhead_beat as f32) * px_per_beat;
                    if ph_x >= lane_rect.left() && ph_x <= lane_rect.right() {
                        ui.painter().line_segment(
                            [
                                egui::pos2(ph_x, lane_rect.top()),
                                egui::pos2(ph_x, lane_rect.bottom()),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(251, 191, 36)),
                        );
                    }
                });
                let _ = row_res;
                ui.add_space(4.0);
            }
            if open_piano {
                self.center_tab = CenterTab::PianoAndVocal;
            }
            if dirty {
                self.audio_dirty = true;
            }
        });
    }

    fn render_piano_and_vocal(&mut self, ui: &mut egui::Ui) {
        ui.heading("MIDI Piano Roll Editor");
        ui.label("Left-click grid to draw note | Right-click note to erase (Snap active)");
        ui.add_space(4.0);

        let sel_reg_id = self.selected_region_id.clone().or_else(|| {
            self.project
                .tracks
                .first()
                .and_then(|t| t.regions.first())
                .map(|r| r.id.clone())
        });

        let mut add_note: Option<(String, u8, f64)> = None;
        let mut rem_note: Option<(String, u8, f64)> = None;

        if let Some(ref rid) = sel_reg_id {
            let found_reg = self
                .project
                .tracks
                .iter()
                .flat_map(|t| t.regions.iter())
                .find(|r| &r.id == rid)
                .cloned();
            if let Some(reg) = found_reg {
                ui.label(format!(
                    "Editing Region: {} ({:.1} beats, {} notes)",
                    reg.name,
                    reg.length_beats,
                    reg.notes.len()
                ));
                let low_pitch: u8 = 48; // C3
                let high_pitch: u8 = 72; // C5
                let num_rows = usize::from(high_pitch - low_pitch + 1);
                let row_h = 10.0_f32;
                let key_w = 38.0_f32;
                let total_h = (num_rows as f32) * row_h;
                let grid_beats = reg.length_beats.max(8.0) as f32;
                let px_per_beat = self.zoom_px_per_beat.max(40.0);

                egui::ScrollArea::horizontal().show(ui, |ui| {
                    let (roll_rect, roll_resp) = ui.allocate_exact_size(
                        egui::vec2(key_w + grid_beats * px_per_beat, total_h),
                        egui::Sense::click(),
                    );
                    let p = ui.painter();
                    p.rect_filled(roll_rect, 3.0, egui::Color32::from_rgb(12, 18, 32));

                    // Piano keys on left + horizontal pitch lanes
                    for idx in 0..num_rows {
                        let pitch = high_pitch - (idx as u8);
                        let y = roll_rect.top() + (idx as f32) * row_h;
                        let is_black = matches!(pitch % 12, 1 | 3 | 6 | 8 | 10);
                        let key_rect = egui::Rect::from_min_size(
                            egui::pos2(roll_rect.left(), y),
                            egui::vec2(key_w, row_h),
                        );
                        p.rect_filled(
                            key_rect,
                            0.0,
                            if is_black {
                                egui::Color32::from_rgb(30, 41, 59)
                            } else {
                                egui::Color32::from_rgb(226, 232, 240)
                            },
                        );
                        if pitch.is_multiple_of(12) {
                            p.text(
                                key_rect.left_center() + egui::vec2(3.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                format!("C{}", (pitch / 12).saturating_sub(1)),
                                egui::FontId::monospace(8.5),
                                egui::Color32::from_rgb(15, 23, 42),
                            );
                        }
                        p.line_segment(
                            [
                                egui::pos2(roll_rect.left() + key_w, y),
                                egui::pos2(roll_rect.right(), y),
                            ],
                            egui::Stroke::new(0.5, egui::Color32::from_rgb(30, 41, 59)),
                        );
                    }

                    // Beat grid lines
                    for b in 0..=(grid_beats.ceil() as i32) {
                        let bx = roll_rect.left() + key_w + (b as f32) * px_per_beat;
                        p.line_segment(
                            [
                                egui::pos2(bx, roll_rect.top()),
                                egui::pos2(bx, roll_rect.bottom()),
                            ],
                            egui::Stroke::new(
                                if b % 4 == 0 { 1.0 } else { 0.5 },
                                egui::Color32::from_rgb(51, 65, 85),
                            ),
                        );
                    }

                    // Draw MIDI notes
                    for n in &reg.notes {
                        if n.pitch >= low_pitch && n.pitch <= high_pitch {
                            let row = usize::from(high_pitch - n.pitch);
                            let nx = roll_rect.left() + key_w + (n.start_beat as f32) * px_per_beat;
                            let nw = ((n.duration_beats as f32) * px_per_beat).max(6.0);
                            let ny = roll_rect.top() + (row as f32) * row_h + 1.0;
                            let note_rect = egui::Rect::from_min_size(
                                egui::pos2(nx, ny),
                                egui::vec2(nw, row_h - 2.0),
                            );
                            p.rect_filled(note_rect, 2.0, egui::Color32::from_rgb(56, 189, 248));
                        }
                    }

                    if let Some(pos) = roll_resp.interact_pointer_pos() {
                        if pos.x > roll_rect.left() + key_w {
                            let row = ((pos.y - roll_rect.top()) / row_h)
                                .floor()
                                .clamp(0.0, (num_rows - 1) as f32)
                                as u8;
                            let pitch = high_pitch.saturating_sub(row);
                            let raw_beat =
                                f64::from((pos.x - (roll_rect.left() + key_w)) / px_per_beat);
                            let snapped = self.snap_beat(raw_beat);
                            if roll_resp.clicked() {
                                add_note = Some((rid.clone(), pitch, snapped));
                            } else if roll_resp.secondary_clicked() {
                                rem_note = Some((rid.clone(), pitch, raw_beat));
                            }
                        }
                    }
                });
            }
        } else {
            ui.label("Select a MIDI region in the Timeline to edit notes.");
        }

        if let Some((rid, pitch, beat)) = add_note {
            let dur = if self.snap_beats > 0.0 {
                self.snap_beats
            } else {
                0.5
            };
            self.run_cmd(
                "Add MIDI Note",
                Command::NoteAdd {
                    project_path: None,
                    region_id: rid,
                    pitch,
                    start_beat: beat,
                    duration_beats: dur,
                    velocity: Some(100),
                },
            );
        }
        if let Some((rid, pitch, beat)) = rem_note {
            self.run_cmd(
                "Remove MIDI Note",
                Command::NoteRemove {
                    project_path: None,
                    region_id: rid,
                    pitch,
                    start_beat: beat,
                },
            );
        }

        ui.separator();
        ui.heading("Vocal Phoneme & Pitch Curve Editor");
        ui.add_space(6.0);
        if let Some(vocal) = &self.project.vocal_line {
            ui.label(format!(
                "Voice Archetype: {} | Phonemes: {}",
                vocal.voice.as_str(),
                vocal.phonemes.len()
            ));
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 220.0),
                egui::Sense::hover(),
            );
            let p = ui.painter();
            p.rect_filled(rect, 4.0, egui::Color32::from_rgb(12, 18, 32));
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 52, 82)),
                egui::StrokeKind::Inside,
            );

            let total_beats = vocal
                .phonemes
                .iter()
                .map(|ph| ph.start_beat + ph.duration_beats)
                .fold(4.0_f64, f64::max) as f32;

            for ph in &vocal.phonemes {
                let x0 = rect.left() + ((ph.start_beat as f32) / total_beats) * rect.width();
                let w = (((ph.duration_beats as f32) / total_beats) * rect.width()).max(24.0);
                let norm_y = 1.0 - ((f32::from(ph.midi_note) - 48.0) / 36.0).clamp(0.0, 1.0);
                let y = rect.top() + 20.0 + norm_y * (rect.height() - 40.0);
                let bar = egui::Rect::from_min_size(
                    egui::pos2(x0 + 2.0, y - 10.0),
                    egui::vec2(w - 4.0, 20.0),
                );
                p.rect_filled(bar, 4.0, egui::Color32::from_rgb(147, 51, 234));
                p.text(
                    bar.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} (M{})", ph.phoneme, ph.midi_note),
                    egui::FontId::monospace(11.0),
                    egui::Color32::WHITE,
                );
            }
        } else {
            ui.label("No vocal line synthesized yet. Use Insert -> Insert Virtual Vocal Line or Synth Vocal.");
        }
    }

    fn render_stems_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("4-Band Stem Separator & DJCraft Handoff");
        ui.add_space(6.0);
        if self.project.stems.is_empty() {
            ui.label(
                "No stems separated yet. Click 'Split Stems' or Commands -> Run Stem Separation.",
            );
        } else {
            egui::Grid::new("stems_grid").striped(true).show(ui, |ui| {
                ui.strong("Stem");
                ui.strong("RMS (dB)");
                ui.strong("Peak (dB)");
                ui.strong("Duration (s)");
                ui.strong("WAV Path");
                ui.end_row();
                for s in &self.project.stems {
                    ui.label(s.kind.as_str());
                    ui.label(format!("{:.1}", s.rms_db));
                    ui.label(format!("{:.1}", s.peak_db));
                    ui.label(format!("{:.2}", s.duration_secs));
                    ui.label(&s.path);
                    ui.end_row();
                }
            });
        }
    }

    fn render_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_command_palette {
            let mut open = self.show_command_palette;
            let mut clicked_spec: Option<CommandSpec> = None;
            egui::Window::new("Command Palette (Ctrl/Cmd+Shift+P)")
                .open(&mut open)
                .collapsible(false)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.palette_query);
                    ui.separator();
                    let q = self.palette_query.trim().to_ascii_lowercase();
                    let specs = command_catalog();
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            for spec in specs {
                                let hay = format!("{} {} {}", spec.id, spec.label, spec.menu)
                                    .to_ascii_lowercase();
                                if !q.is_empty() && !hay.contains(&q) {
                                    continue;
                                }
                                let label = match spec.shortcut {
                                    Some(sc) => {
                                        format!(
                                            "[{}] {} ({}) - {}",
                                            spec.menu, spec.label, spec.id, sc
                                        )
                                    }
                                    None => format!("[{}] {} ({})", spec.menu, spec.label, spec.id),
                                };
                                if ui.button(label).clicked() {
                                    clicked_spec = Some(spec);
                                }
                            }
                        });
                });
            self.show_command_palette = open;
            if let Some(spec) = clicked_spec {
                self.execute_palette_spec(&spec);
            }
        }

        if self.show_shortcuts {
            egui::Window::new("Keyboard Shortcuts (F1)")
                .open(&mut self.show_shortcuts)
                .collapsible(false)
                .show(ctx, |ui| {
                    egui::Grid::new("shortcuts_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Shortcut");
                            ui.strong("Action");
                            ui.end_row();
                            for (k, a) in [
                                ("Space", "Play / Pause Transport"),
                                ("Home", "Rewind to Bar 1"),
                                ("Ctrl+Shift+P / Ctrl+K", "Open Command Palette"),
                                ("F1 / Ctrl+/", "Keyboard Shortcuts"),
                                ("Ctrl+Z / Ctrl+Y", "Undo / Redo"),
                                ("Ctrl+= / Ctrl+-", "Zoom In / Zoom Out"),
                                ("Ctrl+S / Ctrl+E", "Save / Export"),
                            ] {
                                ui.monospace(k);
                                ui.label(a);
                                ui.end_row();
                            }
                        });
                });
        }

        if self.show_docs {
            let mut open = self.show_docs;
            egui::Window::new("MusicCraft Help & Docs")
                .open(&mut open)
                .collapsible(false)
                .default_width(640.0)
                .default_height(440.0)
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.help_tab, HelpTab::GetStarted, "Get Started");
                        ui.selectable_value(&mut self.help_tab, HelpTab::Functions, "Functions");
                        ui.selectable_value(&mut self.help_tab, HelpTab::Usage, "Usage (GUI/CLI/MCP)");
                        ui.selectable_value(&mut self.help_tab, HelpTab::TipsAndTricks, "Tips & Tricks");
                        ui.selectable_value(&mut self.help_tab, HelpTab::OtherCrafts, "Other Crafts");
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        match self.help_tab {
                            HelpTab::GetStarted => {
                                ui.heading("Get Started in 5 Steps");
                                ui.add_space(4.0);
                                ui.label("1. Explore the pre-loaded 'Vienna Aria (Demo)' project (120 BPM, C major, Camelot 8B).");
                                ui.label("2. Create your own song via File > New... (Ctrl+N) or CLI: musiccraft-mcp project new.");
                                ui.label("3. Add MIDI, Audio, or Vocal tracks from the Insert menu or left Studio Toolbox.");
                                ui.label("4. Enter vowel phonemes (e.g. 'aa ee oo ah') in the Vocal Inspector and click Synthesize Vocal.");
                                ui.label("5. Click Render Mix for a mastered -14 LUFS stereo WAV, or Export to DJCraft for stems + handoff.json.");
                            }
                            HelpTab::Functions => {
                                ui.heading("Core Studio Functions");
                                ui.add_space(4.0);
                                ui.strong("Multi-Track Arrangement (.mcraft)");
                                ui.label("JSON project AST with Audio, MIDI, Vocal, and Stem tracks, beat-indexed regions, and 32-step undo/redo.");
                                ui.add_space(4.0);
                                ui.strong("Virtual Vocal Synthesizer");
                                ui.label("Formant-filtered harmonic glottal synth with vibrato LFO across 5 voices: OPERA, SOPRANO, TENOR, CHOIR, SYNTH.");
                                ui.add_space(4.0);
                                ui.strong("4-Band Stem Separator");
                                ui.label("Splits any 16-bit PCM WAV into vocals.wav, drums.wav, bass.wav, and other.wav with RMS/peak telemetry.");
                                ui.add_space(4.0);
                                ui.strong("Offline Mix Renderer & DJCraft Handoff");
                                ui.label("Equal-power stereo panning, soft-clip tanh limiting, LUFS normalization, and handoff.json export.");
                            }
                            HelpTab::Usage => {
                                ui.heading("GUI, CLI & MCP Usage");
                                ui.add_space(4.0);
                                ui.label("Window: Use the 8-menu bar (File..Help), Ctrl+K Command Palette, and Timeline / Piano Roll / Stems tabs.");
                                ui.separator();
                                ui.strong("CLI Examples (every verb supports --json):");
                                ui.monospace("musiccraft-mcp commands --json");
                                ui.monospace("musiccraft-mcp project new --name \"Aria\" --bpm 118 --key \"A minor\" --out song.mcraft --json");
                                ui.monospace("musiccraft-mcp vocal synth --line \"aa ee oo ah\" --voice OPERA --out vocal.wav --json");
                                ui.monospace("musiccraft-mcp mix render --project song.mcraft --out mix.wav --json");
                                ui.monospace("musiccraft-mcp export stems --project song.mcraft --out-dir ./djcraft_handoff --json");
                                ui.separator();
                                ui.strong("MCP Server (2024-11-05 stdio):");
                                ui.monospace("musiccraft-mcp mcp");
                            }
                            HelpTab::TipsAndTricks => {
                                ui.heading("Tips & Tricks");
                                ui.add_space(4.0);
                                ui.label("- Vowel Formants: 'ee' boosts upper brightness (~2.25 kHz); 'oo'/'oh' give warm operatic phrasing; 'aa' opens chest resonance.");
                                ui.label("- Camelot Key Matching: Check the Camelot code in the top bar (e.g. 8B) and match adjacent keys before exporting to djcraft.");
                                ui.label("- Gain Staging: Keep track faders around -6 dB to -1 dB so the -14.0 LUFS master normalizer stays transparent.");
                                ui.label("- Non-Destructive Batching: Omit --save in 'musiccraft-mcp run' to dry-run edits against the demo project.");
                                ui.label("- Fast Navigation: Ctrl+K opens Command Palette, F1 opens Shortcuts, Ctrl+=/Ctrl+- zooms the beat grid.");
                            }
                            HelpTab::OtherCrafts => {
                                ui.heading("Other Storytold Craft Apps");
                                ui.add_space(4.0);
                                egui::Grid::new("other_crafts_grid").striped(true).show(ui, |ui| {
                                    ui.strong("App");
                                    ui.strong("Role & Link to MusicCraft");
                                    ui.end_row();
                                    for (app, desc) in [
                                        ("djcraft", "Live 4-deck DJ performance & Camelot mixing. Loads MusicCraft's handoff.json + stems."),
                                        ("speechcraft", "Wake-word & neural voice bus. Handles spoken commands; MusicCraft handles sung vocals."),
                                        ("celcraft", "2D cel animation & lip-sync X-sheet. Score scenes with MusicCraft WAVs."),
                                        ("smallvideocraft", "Short-form video editor. Use MusicCraft -14 LUFS mixes as music beds."),
                                        ("gamecraft", "3D scene & game workstation. Export looping stems for game audio."),
                                        ("craft-deck", "Unified Craft launcher for installing and starting all Craft apps."),
                                    ] {
                                        ui.monospace(app);
                                        ui.label(desc);
                                        ui.end_row();
                                    }
                                });
                                ui.separator();
                                ui.label("Docs on disk: HELP.md, docs/PRD.md, docs/TIERED_PLAN.md, docs/TOOLS.md, CHANGELOG.md");
                            }
                        }
                    });
                });
            self.show_docs = open;
        }

        if self.show_licenses {
            egui::Window::new("Licenses & Notices")
                .open(&mut self.show_licenses)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("License: MIT OR Apache-2.0");
                    ui.label("Memory Safety: #![forbid(unsafe_code)] across all workspace crates.");
                    ui.label("Zero-Panic Policy: unwrap_used, expect_used, panic, todo, unimplemented denied.");
                    ui.label("Built with Pure-Rust eframe / egui 0.37 + wgpu.");
                });
        }

        if self.show_about {
            egui::Window::new("About MusicCraft")
                .open(&mut self.show_about)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.heading(format!("{APP_NAME} v{APP_VERSION}"));
                    ui.label("Pure-Rust ArtCraft DAW, Virtual Vocal Synthesizer & Stem Studio.");
                    ui.separator();
                    ui.label("Studio-to-Stage Boundary: MusicCraft renders mixes & stems; djcraft performs them.");
                });
        }

        if self.show_preferences {
            egui::Window::new("Preferences")
                .open(&mut self.show_preferences)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.project.bpm, 40.0..=240.0).text("Tempo (BPM)"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.project.mix.target_lufs, -24.0..=-6.0)
                            .text("Target LUFS"),
                    );
                    ui.checkbox(&mut self.project.mix.normalize, "Normalize Master Mix");
                });
        }
    }
}

impl eframe::App for MusicCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);
        let total_beats = self.project.total_beats().max(16.0);
        self.playhead_beat = self.player.tick_and_beat(self.project.bpm, total_beats);
        if self.player.is_playing() {
            ctx.request_repaint();
        }

        egui::Panel::top("menu_bar").show(ui, |ui| {
            self.render_menu_bar(&ctx, ui);
        });

        egui::Panel::top("quick_toolbar").show(ui, |ui| {
            self.render_quick_toolbar(&ctx, ui);
        });

        if self.show_toolbox {
            egui::Panel::left("toolbox")
                .default_size(210.0)
                .show(ui, |ui| {
                    ui.heading("Studio Toolbox");
                    ui.separator();
                    ui.label("New Track Name:");
                    ui.text_edit_singleline(&mut self.new_track_name);
                    if ui.button("Add MIDI Track").clicked() {
                        self.run_cmd(
                            "Add MIDI Track",
                            Command::TrackAdd {
                                project_path: None,
                                name: self.new_track_name.clone(),
                                kind: "midi".to_string(),
                            },
                        );
                    }
                    if ui.button("Add Vocal Track").clicked() {
                        self.run_cmd(
                            "Add Vocal Track",
                            Command::TrackAdd {
                                project_path: None,
                                name: "Lead Vocal".to_string(),
                                kind: "vocal".to_string(),
                            },
                        );
                    }
                    ui.separator();
                    ui.label("Quick Modifiers:");
                    if ui.button("Quantize (1/2 beat)").clicked() {
                        self.run_cmd(
                            "Quantize",
                            Command::ModifyQuantize {
                                project_path: None,
                                track_id: None,
                                grid_beats: 0.5,
                            },
                        );
                    }
                    if ui.button("Transpose +1").clicked() {
                        self.run_cmd(
                            "Transpose +1",
                            Command::ModifyTranspose {
                                project_path: None,
                                semitones: 1,
                                include_vocal: true,
                            },
                        );
                    }
                });
        }

        if self.show_vocal_inspector {
            egui::Panel::right("vocal_inspector")
                .default_size(250.0)
                .show(ui, |ui| {
                    ui.heading("Vocal & Mix Inspector");
                    ui.separator();
                    ui.label("Phoneme Line:");
                    ui.text_edit_singleline(&mut self.vocal_line_input);
                    ui.label("Voice Archetype:");
                    egui::ComboBox::from_id_salt("voice_combo")
                        .selected_text(&self.vocal_voice_input)
                        .show_ui(ui, |ui| {
                            for v in ["OPERA", "SOPRANO", "TENOR", "CHOIR", "SYNTH"] {
                                ui.selectable_value(&mut self.vocal_voice_input, v.to_string(), v);
                            }
                        });
                    if ui.button("Synthesize Vocal").clicked() {
                        let wav = self.default_out_dir().join("vocal.wav");
                        self.run_cmd(
                            "Synth Vocal",
                            Command::VocalSynth {
                                project_path: None,
                                line: self.vocal_line_input.clone(),
                                voice: self.vocal_voice_input.clone(),
                                bpm: Some(self.project.bpm),
                                base_midi: Some(69),
                                out_wav: Some(wav.to_string_lossy().to_string()),
                            },
                        );
                    }

                    if self.show_loudness_hud {
                        ui.separator();
                        ui.heading("Master Loudness HUD");
                        let lufs = self
                            .project
                            .mix
                            .last_lufs
                            .map(|v| format!("{v:.1} LUFS"))
                            .unwrap_or_else(|| "Not rendered".to_string());
                        ui.label(format!("Integrated: {lufs}"));
                        ui.label(format!("Target: {:.1} LUFS", self.project.mix.target_lufs));
                    }

                    ui.separator();
                    ui.heading("Track FX Rack");
                    if let Some(tid) = self.selected_track_id.clone() {
                        ui.horizontal_wrapped(|ui| {
                            for (lbl, kind) in [
                                ("+EQ", "eq"),
                                ("+Comp", "compressor"),
                                ("+Delay", "delay"),
                                ("+Rev", "reverb"),
                            ] {
                                if ui.small_button(lbl).clicked() {
                                    self.run_cmd(
                                        "Add Track FX",
                                        Command::TrackAddFx {
                                            project_path: None,
                                            track_id: tid.clone(),
                                            fx_kind: kind.to_string(),
                                        },
                                    );
                                }
                            }
                        });

                        let mut remove_idx: Option<usize> = None;
                        let mut fx_dirty = false;
                        if let Some(trk) = self.project.tracks.iter_mut().find(|t| t.id == tid) {
                            for (idx, fx) in trk.fx.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    match fx {
                                        TrackFx::Eq3Band {
                                            enabled,
                                            low_db,
                                            high_db,
                                            ..
                                        } => {
                                            fx_dirty |= ui.checkbox(enabled, "EQ3").changed();
                                            fx_dirty |= ui
                                                .add(
                                                    egui::Slider::new(low_db, -12.0..=12.0)
                                                        .text("L"),
                                                )
                                                .changed();
                                            fx_dirty |= ui
                                                .add(
                                                    egui::Slider::new(high_db, -12.0..=12.0)
                                                        .text("H"),
                                                )
                                                .changed();
                                        }
                                        TrackFx::Compressor {
                                            enabled,
                                            threshold_db,
                                            ratio,
                                        } => {
                                            fx_dirty |= ui.checkbox(enabled, "Comp").changed();
                                            fx_dirty |= ui
                                                .add(
                                                    egui::Slider::new(threshold_db, -36.0..=0.0)
                                                        .text("Thr"),
                                                )
                                                .changed();
                                            fx_dirty |= ui
                                                .add(egui::Slider::new(ratio, 1.0..=10.0).text("R"))
                                                .changed();
                                        }
                                        TrackFx::Delay {
                                            enabled,
                                            time_beats,
                                            mix,
                                            ..
                                        } => {
                                            fx_dirty |= ui.checkbox(enabled, "Dly").changed();
                                            fx_dirty |= ui
                                                .add(
                                                    egui::Slider::new(time_beats, 0.125..=2.0)
                                                        .text("B"),
                                                )
                                                .changed();
                                            fx_dirty |= ui
                                                .add(egui::Slider::new(mix, 0.0..=1.0).text("Mix"))
                                                .changed();
                                        }
                                        TrackFx::Reverb {
                                            enabled,
                                            room_size,
                                            mix,
                                        } => {
                                            fx_dirty |= ui.checkbox(enabled, "Rev").changed();
                                            fx_dirty |= ui
                                                .add(
                                                    egui::Slider::new(room_size, 0.1..=1.0)
                                                        .text("Rm"),
                                                )
                                                .changed();
                                            fx_dirty |= ui
                                                .add(egui::Slider::new(mix, 0.0..=1.0).text("Mix"))
                                                .changed();
                                        }
                                    }
                                    if ui.small_button("X").clicked() {
                                        remove_idx = Some(idx);
                                    }
                                });
                            }
                            if let Some(ri) = remove_idx {
                                trk.fx.remove(ri);
                                fx_dirty = true;
                            }
                        }
                        if fx_dirty {
                            self.audio_dirty = true;
                        }
                    } else {
                        ui.label("Select a track to edit FX.");
                    }

                    ui.separator();
                    ui.label("Last Dispatch Output:");
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            let text = serde_json::to_string_pretty(&self.last_json)
                                .unwrap_or_else(|_| "{}".to_string());
                            ui.monospace(text);
                        });
                });
        }

        if self.show_mixer {
            egui::Panel::bottom("mixer_console")
                .default_size(145.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Mixer Console");
                        ui.separator();
                        ui.label(&self.last_status);
                    });
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for trk in &mut self.project.tracks {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(16, 26, 46))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(38, 52, 82),
                                    ))
                                    .corner_radius(4.0)
                                    .inner_margin(6.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(120.0);
                                        ui.strong(&trk.name);
                                        ui.add(
                                            egui::Slider::new(&mut trk.volume_db, -48.0..=6.0)
                                                .text("dB"),
                                        );
                                        ui.add(
                                            egui::Slider::new(&mut trk.pan, -1.0..=1.0).text("Pan"),
                                        );
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut trk.muted, "M");
                                            ui.checkbox(&mut trk.solo, "S");
                                        });
                                    });
                            }
                        });
                    });
                });
        }

        egui::CentralPanel::default().show(ui, |ui| match self.center_tab {
            CenterTab::Timeline => {
                if self.show_timeline {
                    self.render_timeline_viewport(ui);
                } else {
                    ui.label("Timeline hidden (re-enable via View -> Multi-Track Timeline).");
                }
            }
            CenterTab::PianoAndVocal => self.render_piano_and_vocal(ui),
            CenterTab::Stems => self.render_stems_view(ui),
        });

        self.render_dialogs(&ctx);
    }
}

/// Headless `--smoke` verification: exercises L1 dispatch parity + Tofu Guard without opening a window.
fn run_smoke() -> Result<(), String> {
    for label in tofu_guard::REQUIRED_UI_LABELS {
        tofu_guard::check_string_for_tofu(label)?;
    }

    let tmp = std::env::temp_dir().join("musiccraft_app_smoke");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let mut proj = SongProject::new_demo();
    let cmds = [
        Command::SystemStatus,
        Command::TrackAdd {
            project_path: None,
            name: "Smoke Synth".to_string(),
            kind: "midi".to_string(),
        },
        Command::ModifyQuantize {
            project_path: None,
            track_id: None,
            grid_beats: 0.5,
        },
        Command::ModifyTranspose {
            project_path: None,
            semitones: 2,
            include_vocal: true,
        },
        Command::VocalSynth {
            project_path: None,
            line: "aa ee oo ah".to_string(),
            voice: "OPERA".to_string(),
            bpm: Some(120.0),
            base_midi: Some(69),
            out_wav: Some(tmp.join("vocal.wav").to_string_lossy().to_string()),
        },
        Command::MixRender {
            project_path: None,
            out_wav: tmp.join("mix.wav").to_string_lossy().to_string(),
        },
        Command::ExportStems {
            project_path: None,
            out_dir: tmp.join("handoff").to_string_lossy().to_string(),
        },
    ];

    for cmd in cmds {
        let res = dispatch_on_project(&mut proj, cmd);
        if res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(format!("smoke dispatch failed: {res}"));
        }
    }

    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

fn main() {
    if std::env::args().any(|a| a == "--smoke") {
        match run_smoke() {
            Ok(()) => {
                println!(
                    "{{\"ok\":true,\"smoke\":\"passed\",\"app\":\"musiccraft\",\"version\":\"{APP_VERSION}\"}}"
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("[musiccraft] smoke failed: {e}");
                std::process::exit(1);
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([960.0, 620.0])
            .with_title(format!("{APP_NAME} v{APP_VERSION} -- Studio Workstation")),
        ..Default::default()
    };

    if let Err(e) = eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(harsh_visuals());
            Ok(Box::new(MusicCraftApp::new()))
        }),
    ) {
        eprintln!("[musiccraft] failed to launch desktop UI: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_core::dispatch;

    #[test]
    fn test_smoke_passes() {
        assert!(run_smoke().is_ok());
    }

    #[test]
    fn test_glyph_allowlist_source_scan() {
        let src = include_str!("main.rs");
        assert!(
            tofu_guard::check_string_for_tofu(src).is_ok(),
            "main.rs contains a disallowed non-ASCII glyph"
        );
    }

    #[tokio::test]
    async fn test_gui_mcp_dispatch_parity() {
        let mut gui_proj = SongProject::new_demo();
        let gui_res = dispatch_on_project(&mut gui_proj, Command::SystemStatus);
        let mcp_res = dispatch(Command::SystemStatus).await;
        assert_eq!(gui_res, mcp_res);
    }
}
