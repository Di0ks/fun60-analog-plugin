//! USB HID Keyboard/Keypad usage codes (HID Usage Tables page 0x07).
//!
//! These are the codes the Wooting Analog SDK uses in its default HID keycode
//! mode, and the vocabulary for the plugin's keymap. Only keys present on the
//! FUN60 Ultra layout (plus a few extras likely to be useful for bindings)
//! are named here.

/// A named HID keyboard usage code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Digits
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,

    // Whitespace / control
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    CapsLock,

    // Punctuation
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    NonUsHash,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Dot,
    Slash,

    // Modifiers
    LCtrl,
    LShift,
    LAlt,
    LMeta,
    RCtrl,
    RShift,
    RAlt,
    RMeta,

    // Navigation / misc
    Application,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Arrows
    Left,
    Right,
    Up,
    Down,
}

impl Key {
    /// HID usage code for this key.
    pub const fn code(self) -> u16 {
        match self {
            // Letters (alphabetical by usage code: A=4..Z=29)
            Key::A => 0x04,
            Key::B => 0x05,
            Key::C => 0x06,
            Key::D => 0x07,
            Key::E => 0x08,
            Key::F => 0x09,
            Key::G => 0x0A,
            Key::H => 0x0B,
            Key::I => 0x0C,
            Key::J => 0x0D,
            Key::K => 0x0E,
            Key::L => 0x0F,
            Key::M => 0x10,
            Key::N => 0x11,
            Key::O => 0x12,
            Key::P => 0x13,
            Key::Q => 0x14,
            Key::R => 0x15,
            Key::S => 0x16,
            Key::T => 0x17,
            Key::U => 0x18,
            Key::V => 0x19,
            Key::W => 0x1A,
            Key::X => 0x1B,
            Key::Y => 0x1C,
            Key::Z => 0x1D,

            // Digits (1=0x1E..9,0=0x27)
            Key::Num1 => 0x1E,
            Key::Num2 => 0x1F,
            Key::Num3 => 0x20,
            Key::Num4 => 0x21,
            Key::Num5 => 0x22,
            Key::Num6 => 0x23,
            Key::Num7 => 0x24,
            Key::Num8 => 0x25,
            Key::Num9 => 0x26,
            Key::Num0 => 0x27,

            // Whitespace / control
            Key::Enter => 0x28,
            Key::Escape => 0x29,
            Key::Backspace => 0x2A,
            Key::Tab => 0x2B,
            Key::Space => 0x2C,
            Key::CapsLock => 0x39,

            // Punctuation
            Key::Minus => 0x2D,
            Key::Equal => 0x2E,
            Key::LeftBracket => 0x2F,
            Key::RightBracket => 0x30,
            Key::Backslash => 0x31,
            Key::NonUsHash => 0x32,
            Key::Semicolon => 0x33,
            Key::Apostrophe => 0x34,
            Key::Grave => 0x35,
            Key::Comma => 0x36,
            Key::Dot => 0x37,
            Key::Slash => 0x38,

            // Modifiers
            Key::LCtrl => 0xE0,
            Key::LShift => 0xE1,
            Key::LAlt => 0xE2,
            Key::LMeta => 0xE3,
            Key::RCtrl => 0xE4,
            Key::RShift => 0xE5,
            Key::RAlt => 0xE6,
            Key::RMeta => 0xE7,

            // Navigation / misc
            Key::Application => 0x65,
            Key::F1 => 0x3A,
            Key::F2 => 0x3B,
            Key::F3 => 0x3C,
            Key::F4 => 0x3D,
            Key::F5 => 0x3E,
            Key::F6 => 0x3F,
            Key::F7 => 0x40,
            Key::F8 => 0x41,
            Key::F9 => 0x42,
            Key::F10 => 0x43,
            Key::F11 => 0x44,
            Key::F12 => 0x45,

            // Arrows
            Key::Left => 0x50,
            Key::Right => 0x4F,
            Key::Up => 0x52,
            Key::Down => 0x51,
        }
    }

    /// The DB name for this key (as used by `FUN60_MATRIX_NAMES`), if any.
    ///
    /// Returns None for keys not on the FUN60 Ultra layout.
    pub const fn db_name(self) -> Option<&'static str> {
        match self {
            Key::Escape => Some("Esc"),
            Key::Tab => Some("Tab"),
            Key::CapsLock => Some("CapsLock"),
            Key::LShift => Some("LShift"),
            Key::LCtrl => Some("LCtrl"),
            Key::Num1 => Some("1"),
            Key::Q => Some("Q"),
            Key::A => Some("A"),
            Key::Num2 => Some("2"),
            Key::W => Some("W"),
            Key::S => Some("S"),
            Key::Z => Some("Z"),
            Key::LMeta => Some("LMeta"),
            Key::Num3 => Some("3"),
            Key::E => Some("E"),
            Key::D => Some("D"),
            Key::X => Some("X"),
            Key::LAlt => Some("LAlt"),
            Key::Num4 => Some("4"),
            Key::R => Some("R"),
            Key::F => Some("F"),
            Key::C => Some("C"),
            Key::Num5 => Some("5"),
            Key::T => Some("T"),
            Key::G => Some("G"),
            Key::V => Some("V"),
            Key::Num6 => Some("6"),
            Key::Y => Some("Y"),
            Key::H => Some("H"),
            Key::B => Some("B"),
            Key::Space => Some("Space"),
            Key::Num7 => Some("7"),
            Key::U => Some("U"),
            Key::J => Some("J"),
            Key::N => Some("N"),
            Key::Num8 => Some("8"),
            Key::I => Some("I"),
            Key::K => Some("K"),
            Key::M => Some("M"),
            Key::Num9 => Some("9"),
            Key::O => Some("O"),
            Key::L => Some("L"),
            Key::Comma => Some(","),
            Key::Num0 => Some("0"),
            Key::P => Some("P"),
            Key::Semicolon => Some(";"),
            Key::Dot => Some("."),
            Key::RAlt => Some("RAlt"),
            Key::Minus => Some("-"),
            Key::LeftBracket => Some("["),
            Key::Apostrophe => Some("'"),
            Key::Slash => Some("/"),
            Key::Equal => Some("="),
            Key::RightBracket => Some("]"),
            Key::RShift => Some("RShift"),
            Key::Application => Some("Application"),
            Key::Backspace => Some("Backspace"),
            Key::Backslash => Some("\\"),
            Key::Enter => Some("Enter"),
            Key::RCtrl => Some("RCtrl"),
            _ => None,
        }
    }

    /// Inverse of [`Key::db_name`]: the Key for a DB name, if any.
    ///
    /// This is the authoritative name → Key mapping used by the keymap.
    pub fn db_key_of(name: &str) -> Option<Self> {
        match name {
            "Esc" => Some(Key::Escape),
            "Tab" => Some(Key::Tab),
            "CapsLock" => Some(Key::CapsLock),
            "LShift" => Some(Key::LShift),
            "LCtrl" => Some(Key::LCtrl),
            "1" => Some(Key::Num1),
            "Q" => Some(Key::Q),
            "A" => Some(Key::A),
            "2" => Some(Key::Num2),
            "W" => Some(Key::W),
            "S" => Some(Key::S),
            "Z" => Some(Key::Z),
            "LMeta" => Some(Key::LMeta),
            "3" => Some(Key::Num3),
            "E" => Some(Key::E),
            "D" => Some(Key::D),
            "X" => Some(Key::X),
            "LAlt" => Some(Key::LAlt),
            "4" => Some(Key::Num4),
            "R" => Some(Key::R),
            "F" => Some(Key::F),
            "C" => Some(Key::C),
            "5" => Some(Key::Num5),
            "T" => Some(Key::T),
            "G" => Some(Key::G),
            "V" => Some(Key::V),
            "6" => Some(Key::Num6),
            "Y" => Some(Key::Y),
            "H" => Some(Key::H),
            "B" => Some(Key::B),
            "Space" => Some(Key::Space),
            "7" => Some(Key::Num7),
            "U" => Some(Key::U),
            "J" => Some(Key::J),
            "N" => Some(Key::N),
            "8" => Some(Key::Num8),
            "I" => Some(Key::I),
            "K" => Some(Key::K),
            "M" => Some(Key::M),
            "9" => Some(Key::Num9),
            "O" => Some(Key::O),
            "L" => Some(Key::L),
            "," => Some(Key::Comma),
            "0" => Some(Key::Num0),
            "P" => Some(Key::P),
            ";" => Some(Key::Semicolon),
            "." => Some(Key::Dot),
            "RAlt" => Some(Key::RAlt),
            "-" => Some(Key::Minus),
            "[" => Some(Key::LeftBracket),
            "'" => Some(Key::Apostrophe),
            "/" => Some(Key::Slash),
            "=" => Some(Key::Equal),
            "]" => Some(Key::RightBracket),
            "RShift" => Some(Key::RShift),
            "Application" => Some(Key::Application),
            "Backspace" => Some(Key::Backspace),
            "\\" => Some(Key::Backslash),
            "Enter" => Some(Key::Enter),
            "RCtrl" => Some(Key::RCtrl),
            "`" => Some(Key::Grave),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_check_usage_codes() {
        assert_eq!(Key::A.code(), 0x04);
        assert_eq!(Key::D.code(), 0x07);
        assert_eq!(Key::S.code(), 0x16);
        assert_eq!(Key::W.code(), 0x1A);
        assert_eq!(Key::Space.code(), 0x2C);
        assert_eq!(Key::LCtrl.code(), 0xE0);
        assert_eq!(Key::RCtrl.code(), 0xE4);
        assert_eq!(Key::Num1.code(), 0x1E);
        assert_eq!(Key::Num0.code(), 0x27);
        assert_eq!(Key::Escape.code(), 0x29);
    }

    #[test]
    fn codes_are_unique() {
        use std::collections::HashSet;
        let all = [
            Key::A,
            Key::B,
            Key::C,
            Key::D,
            Key::E,
            Key::F,
            Key::G,
            Key::H,
            Key::I,
            Key::J,
            Key::K,
            Key::L,
            Key::M,
            Key::N,
            Key::O,
            Key::P,
            Key::Q,
            Key::R,
            Key::S,
            Key::T,
            Key::U,
            Key::V,
            Key::W,
            Key::X,
            Key::Y,
            Key::Z,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
            Key::Num0,
            Key::Enter,
            Key::Escape,
            Key::Backspace,
            Key::Tab,
            Key::Space,
            Key::CapsLock,
            Key::Minus,
            Key::Equal,
            Key::LeftBracket,
            Key::RightBracket,
            Key::Backslash,
            Key::NonUsHash,
            Key::Semicolon,
            Key::Apostrophe,
            Key::Grave,
            Key::Comma,
            Key::Dot,
            Key::Slash,
            Key::LCtrl,
            Key::LShift,
            Key::LAlt,
            Key::LMeta,
            Key::RCtrl,
            Key::RShift,
            Key::RAlt,
            Key::RMeta,
            Key::Application,
            Key::F1,
            Key::F12,
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
        ];
        let set: HashSet<u16> = all.iter().map(|&k| k.code()).collect();
        assert_eq!(set.len(), all.len(), "usage codes must be unique");
    }

    #[test]
    fn db_names_round_trip_through_codes() {
        // Every Key that has a db_name must produce the same code the name
        // maps to in the legacy name table — single source of truth check.
        for key in [
            Key::Escape,
            Key::Tab,
            Key::CapsLock,
            Key::LShift,
            Key::LCtrl,
            Key::Num1,
            Key::Q,
            Key::A,
            Key::W,
            Key::S,
            Key::Z,
            Key::LMeta,
            Key::Num3,
            Key::E,
            Key::D,
            Key::X,
            Key::LAlt,
            Key::Num4,
            Key::R,
            Key::F,
            Key::C,
            Key::Num5,
            Key::T,
            Key::G,
            Key::V,
            Key::Num6,
            Key::Y,
            Key::H,
            Key::B,
            Key::Space,
            Key::Num7,
            Key::U,
            Key::J,
            Key::N,
            Key::Num8,
            Key::I,
            Key::K,
            Key::M,
            Key::Num9,
            Key::O,
            Key::L,
            Key::Comma,
            Key::Num0,
            Key::P,
            Key::Semicolon,
            Key::Dot,
            Key::RAlt,
            Key::Minus,
            Key::LeftBracket,
            Key::Apostrophe,
            Key::Slash,
            Key::Equal,
            Key::RightBracket,
            Key::RShift,
            Key::Application,
            Key::Backspace,
            Key::Backslash,
            Key::Enter,
            Key::RCtrl,
        ] {
            let name = key.db_name().expect("test list contains only mapped keys");
            assert_eq!(
                crate::keymap::hid_code_for(name),
                Some(key.code()),
                "name/code mismatch for {name}"
            );
        }
    }
}
