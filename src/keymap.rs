//! FUN60 Ultra matrix-index → HID usage-code mapping and depth normalization.
//!
//! Matrix indices come from the depth server; the DB layout for device
//! 12625:20528:2307 (FUN60 Ultra, `Common61_gk06`) names 60 keys. Names are
//! translated to USB HID Keyboard/Keypad usage codes (page 0x07), which is
//! the Wooting Analog SDK's default keycode mode.
//!
//! Keys without a mapping are reported in the custom-key space (0x0200 +
//! matrix index) so they're still observable by SDK consumers.

/// HID usage code for a named key. None = no standard HID mapping.
///
/// Backed by [`crate::hid::Key`] — the single source of truth for codes.
/// Used by tests to verify the name table agrees with the Key enum.
#[cfg(test)]
pub(crate) fn hid_code_for(name: &str) -> Option<u16> {
    use crate::hid::Key;
    let key = match name {
        "Esc" => Key::Escape,
        "Tab" => Key::Tab,
        "CapsLock" => Key::CapsLock,
        "Space" => Key::Space,
        "Backspace" => Key::Backspace,
        "Enter" => Key::Enter,
        "-" => Key::Minus,
        "=" => Key::Equal,
        "[" => Key::LeftBracket,
        "]" => Key::RightBracket,
        "\\" => Key::Backslash,
        ";" => Key::Semicolon,
        "'" => Key::Apostrophe,
        "`" => Key::Grave,
        "," => Key::Comma,
        "." => Key::Dot,
        "/" => Key::Slash,
        "LShift" => Key::LShift,
        "RShift" => Key::RShift,
        "LCtrl" => Key::LCtrl,
        "RCtrl" => Key::RCtrl,
        "LAlt" => Key::LAlt,
        "RAlt" => Key::RAlt,
        "LMeta" => Key::LMeta,
        "RMeta" => Key::RMeta,
        "Application" => Key::Application,

        // digits
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "0" => Key::Num0,

        // letters
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,

        _ => return None,
    };
    Some(key.code())
}

/// Matrix index → key name for the FUN60 Ultra (device ID 2307).
///
/// Positions without a physical key (matrix placeholders) are omitted.
/// Names correspond to [`crate::hid::Key::db_name`].
/// 
/// **Fn** key does not have a standard HID name, but included here as
/// it can be received too.
pub const FUN60_MATRIX_NAMES: &[(u8, &str)] = &[
    (1, "Esc"),
    (2, "Tab"),
    (3, "CapsLock"),
    (4, "LShift"),
    (5, "LCtrl"),
    (7, "1"),
    (8, "Q"),
    (9, "A"),
    (13, "2"),
    (14, "W"),
    (15, "S"),
    (16, "Z"),
    (17, "LMeta"),
    (19, "3"),
    (20, "E"),
    (21, "D"),
    (22, "X"),
    (23, "LAlt"),
    (25, "4"),
    (26, "R"),
    (27, "F"),
    (28, "C"),
    (31, "5"),
    (32, "T"),
    (33, "G"),
    (34, "V"),
    (37, "6"),
    (38, "Y"),
    (39, "H"),
    (40, "B"),
    (41, "Space"),
    (43, "7"),
    (44, "U"),
    (45, "J"),
    (46, "N"),
    (49, "8"),
    (50, "I"),
    (51, "K"),
    (52, "M"),
    (55, "9"),
    (56, "O"),
    (57, "L"),
    (58, ","),
    (61, "0"),
    (62, "P"),
    (63, ";"),
    (64, "."),
    (65, "RAlt"),
    (67, "-"),
    (68, "["),
    (69, "'"),
    (70, "/"),
    (71, "Fn"),
    (73, "="),
    (74, "]"),
    (76, "RShift"),
    (77, "Application"),
    (79, "Backspace"),
    (80, "\\"),
    (81, "Enter"),
    (83, "RCtrl"),
];

/// Matrix index → [`crate::hid::Key`] for the FUN60 Ultra.
///
/// Derived from [`FUN60_MATRIX_NAMES`]; positions whose names have no `Key`
/// mapping are omitted.
pub fn build_matrix_to_key() -> Vec<(u8, crate::hid::Key)> {
    FUN60_MATRIX_NAMES
        .iter()
        .filter_map(|&(idx, name)| crate::hid::Key::db_key_of(name).map(|key| (idx, key)))
        .collect()
}

/// Custom-key prefix for matrix indices without a standard HID mapping.
pub const CUSTOM_KEY_BASE: u16 = 0x0200;

/// Build the matrix-index → HID-code table.
///
/// The returned table has one entry per named matrix position; keys with no
/// standard HID mapping get `CUSTOM_KEY_BASE + matrix_index`.
pub fn build_matrix_to_hid() -> Vec<(u8, u16)> {
    build_matrix_to_key()
        .into_iter()
        .map(|(idx, key)| (idx, key.code()))
        .chain(
            // Positions whose names have no standard HID mapping fall back to
            // the custom-key space. (Currently none, but keep the fallback.)
            FUN60_MATRIX_NAMES
                .iter()
                .filter(|&&(_, name)| crate::hid::Key::db_key_of(name).is_none())
                .map(|&(idx, _)| (idx, CUSTOM_KEY_BASE + idx as u16)),
        )
        .collect()
}

/// Full travel of the FUN60 Ultra switches in depth_raw units (0.01mm).
/// Observed full-press max was 350; can be overridden at runtime.
pub const DEFAULT_FULL_TRAVEL: u16 = 350;

/// Normalize a raw depth to the SDK's 0.0–1.0 analog range.
pub fn normalize(depth_raw: u16, full_travel: u16) -> f32 {
    if full_travel == 0 {
        return 0.0;
    }
    (depth_raw as f32 / full_travel as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::collections::HashSet;

    #[test]
    fn all_60_keys_present() {
        assert_eq!(FUN60_MATRIX_NAMES.len(), 60);
        let idxs: HashSet<u8> = FUN60_MATRIX_NAMES.iter().map(|&(i, _)| i).collect();
        assert_eq!(idxs.len(), 60, "matrix indices must be unique");
    }

    #[test]
    fn letters_map_to_expected_codes() {
        let table: HashMap<u8, u16> = build_matrix_to_hid().into_iter().collect();
        assert_eq!(table[&9], 0x04); // A
        assert_eq!(table[&8], 0x14); // Q
        assert_eq!(table[&41], 0x2C); // Space
        assert_eq!(table[&83], 0xE4); // RCtrl
    }

    #[test]
    fn digits_map_correctly() {
        let table: HashMap<u8, u16> = build_matrix_to_hid().into_iter().collect();
        assert_eq!(table[&7], 0x1E); // 1
        assert_eq!(table[&61], 0x27); // 0
    }

    #[test]
    fn all_mapped_codes_are_valid_hid() {
        for (_, code) in build_matrix_to_hid() {
            assert!(
                (0x04..=0x65).contains(&code) || (0xE0..=0xE7).contains(&code),
                "code {code:#04x} outside HID keyboard range"
            );
        }
    }

    #[test]
    fn normalize_zero_and_full() {
        assert_eq!(normalize(0, DEFAULT_FULL_TRAVEL), 0.0);
        assert_eq!(normalize(DEFAULT_FULL_TRAVEL, DEFAULT_FULL_TRAVEL), 1.0);
        assert_eq!(normalize(175, DEFAULT_FULL_TRAVEL), 0.5);
    }

    #[test]
    fn normalize_clamps_overtravel() {
        assert_eq!(normalize(400, DEFAULT_FULL_TRAVEL), 1.0);
        assert_eq!(normalize(100, 0), 0.0);
    }
}
