#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use musiccraft_core::{
    command_catalog, dispatch, dispatch_on_project, read_wav_16bit, write_wav_16bit, Command,
    SongProject,
};
use serde_json::json;

#[tokio::test]
async fn test_dispatch_status_and_catalog() {
    let catalog = command_catalog();
    assert!(catalog.len() >= 15);
    let res = dispatch(Command::SystemStatus).await;
    assert_eq!(res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(res.get("app").and_then(|v| v.as_str()), Some("musiccraft"));
}

#[test]
fn test_project_lifecycle_and_edits() {
    let tmp = std::env::temp_dir().join("musiccraft_test_lifecycle");
    let _ = std::fs::remove_dir_all(&tmp);
    let proj_path = tmp.join("song.mcraft");

    let mut proj = SongProject::new_demo();
    let res = dispatch_on_project(
        &mut proj,
        Command::TrackAdd {
            project_path: None,
            name: "Strings".to_string(),
            kind: "midi".to_string(),
        },
    );
    assert_eq!(res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(proj.tracks.len(), 4);

    let res_t = dispatch_on_project(
        &mut proj,
        Command::ModifyTranspose {
            project_path: None,
            semitones: 2,
            include_vocal: true,
        },
    );
    assert_eq!(res_t.get("ok").and_then(|v| v.as_bool()), Some(true));

    let res_s = dispatch_on_project(
        &mut proj,
        Command::RegionSplit {
            project_path: None,
            region_id: "reg-chords".to_string(),
            split_beat: 8.0,
        },
    );
    assert_eq!(res_s.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(proj.tracks[0].regions.len(), 2);

    let save_res = dispatch_on_project(
        &mut proj,
        Command::ProjectSave {
            path: proj_path.to_string_lossy().to_string(),
        },
    );
    assert_eq!(save_res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(proj_path.exists());

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_wav_roundtrip_and_hostile_input() {
    let tmp = std::env::temp_dir().join("musiccraft_test_wav");
    let _ = std::fs::remove_dir_all(&tmp);
    let wav_path = tmp.join("tone.wav");

    let samples = vec![0.0_f32, 0.25, -0.25, 0.5, -0.5, 0.0];
    assert!(write_wav_16bit(&wav_path, 44_100, 2, &samples).is_ok());

    let loaded = read_wav_16bit(&wav_path);
    assert!(loaded.is_ok());
    if let Ok(buf) = loaded {
        assert_eq!(buf.sample_rate, 44_100);
        assert_eq!(buf.channels, 2);
        assert_eq!(buf.samples.len(), samples.len());
    }

    // Hostile truncated WAV must return Err, never panic
    let bad_path = tmp.join("bad.wav");
    let _ = std::fs::write(&bad_path, b"RIFF\x00\x00\x00\x00WAVEfmt ");
    assert!(read_wav_16bit(&bad_path).is_err());

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_render_stems_vocal_and_djcraft_handoff() {
    let tmp = std::env::temp_dir().join("musiccraft_test_render");
    let _ = std::fs::remove_dir_all(&tmp);
    let mix_wav = tmp.join("mix.wav");
    let vocal_wav = tmp.join("vocal.wav");
    let stems_dir = tmp.join("stems");
    let handoff_dir = tmp.join("handoff");

    let vocal_res = dispatch(Command::VocalSynth {
        project_path: None,
        line: "aa ee oo ah".to_string(),
        voice: "OPERA".to_string(),
        bpm: Some(120.0),
        base_midi: Some(69),
        out_wav: Some(vocal_wav.to_string_lossy().to_string()),
    })
    .await;
    assert_eq!(vocal_res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(vocal_wav.exists());

    let mix_res = dispatch(Command::MixRender {
        project_path: None,
        out_wav: mix_wav.to_string_lossy().to_string(),
    })
    .await;
    assert_eq!(mix_res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(mix_wav.exists());

    let stem_res = dispatch(Command::StemSplit {
        project_path: None,
        in_wav: Some(mix_wav.to_string_lossy().to_string()),
        out_dir: stems_dir.to_string_lossy().to_string(),
    })
    .await;
    assert_eq!(stem_res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(stems_dir.join("vocals.wav").exists());
    assert!(stems_dir.join("drums.wav").exists());
    assert!(stems_dir.join("bass.wav").exists());
    assert!(stems_dir.join("other.wav").exists());

    let export_res = dispatch(Command::ExportStems {
        project_path: None,
        out_dir: handoff_dir.to_string_lossy().to_string(),
    })
    .await;
    assert_eq!(export_res.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(handoff_dir.join("handoff.json").exists());

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_command_from_name_accepts_dotted_and_underscore() {
    let cmd1 = Command::from_name_and_params("project.new", &json!({"name": "Test"}));
    let cmd2 = Command::from_name_and_params("musiccraft_project_new", &json!({"name": "Test"}));
    assert!(cmd1.is_ok());
    assert!(cmd2.is_ok());
}
