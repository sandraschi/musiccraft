//! Tofu glyph guard ensuring all UI labels and source strings remain 100% renderable in egui.
#![forbid(unsafe_code)]

/// Check a string for any non-ASCII characters outside the verified safe set, or control chars.
pub fn check_string_for_tofu(s: &str) -> Result<(), String> {
    for (idx, ch) in s.char_indices() {
        if ch == '\u{FFFD}' {
            return Err(format!(
                "Unicode replacement character (U+FFFD) at byte {idx} in {s:?}"
            ));
        }
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            return Err(format!(
                "Control character (U+{:04X}) at byte {idx} in {s:?}",
                ch as u32
            ));
        }
        if !ch.is_ascii() {
            let safe = matches!(
                ch,
                '\u{00B0}' | '\u{00D7}' | '\u{00B7}' | '\u{2026}' | '\u{2014}'
            );
            if !safe {
                return Err(format!(
                    "Disallowed non-ASCII glyph '{ch}' (U+{:04X}) at byte {idx}",
                    ch as u32
                ));
            }
        }
    }
    Ok(())
}

pub const REQUIRED_UI_LABELS: &[&str] = &[
    "MusicCraft",
    "Safe Rust (0 unsafe)",
    "File",
    "Edit",
    "View",
    "Insert",
    "Modify",
    "Commands",
    "Window",
    "Help",
    "Command Palette...",
    "Keyboard Shortcuts...",
    "Project Files & Docs...",
    "Licenses & Notices...",
    "About MusicCraft",
    "Multi-Track Timeline",
    "Mixer Console",
    "Vocal & Pitch Curve",
    "Stem Separator",
    "Master Loudness HUD",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_labels_have_no_tofu() {
        for label in REQUIRED_UI_LABELS {
            assert!(
                check_string_for_tofu(label).is_ok(),
                "tofu risk in label: {label}"
            );
        }
    }

    #[test]
    fn test_tofu_guard_catches_emoji_and_replacement() {
        assert!(check_string_for_tofu("bad \u{FFFD}").is_err());
        assert!(check_string_for_tofu("bad \u{25CF} dot").is_err());
    }
}
