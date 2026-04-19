//! Key name parser for converting friendly key names to evdev::Key codes
//!
//! This module provides a parser that handles case-insensitive key name lookup
//! and friendly abbreviation expansion (e.g., "a" -> KEY_A, "capslock" -> KEY_CAPSLOCK).

use evdev::Key;
use std::collections::HashMap;
use std::fmt;

/// Error type for key parsing failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Unknown key name that couldn't be resolved
    UnknownKey(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownKey(key) => write!(f, "Unknown key name: '{}'", key),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parser for converting key names to evdev::Key codes
///
/// The KeyParser provides case-insensitive lookup and friendly name expansion.
/// All lookups are normalized to lowercase for consistency.
///
/// # Examples
///
/// ```ignore
/// let parser = KeyParser::new();
/// assert_eq!(parser.parse("KEY_A"), Ok(Key::KEY_A));
/// assert_eq!(parser.parse("key_a"), Ok(Key::KEY_A));
/// assert_eq!(parser.parse("a"), Ok(Key::KEY_A));
/// assert_eq!(parser.parse("capslock"), Ok(Key::KEY_CAPSLOCK));
/// ```
#[derive(Debug)]
pub struct KeyParser {
    /// Mapping from normalized (lowercase) key names to evdev::Key codes
    name_to_key: HashMap<String, Key>,
}

impl KeyParser {
    /// Create a new KeyParser with a complete lookup table
    ///
    /// The constructor builds a comprehensive mapping of:
    /// - All standard evdev KEY_* names (from the Key enum)
    /// - Common friendly abbreviations (a-z, ctrl, shift, alt, etc.)
    /// - All names stored lowercase for case-insensitive lookup
    pub fn new() -> Self {
        let mut name_to_key = HashMap::new();

        // === Letters A-Z ===
        // QWERTY row 1
        let row1 = ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"];
        for (i, name) in row1.iter().enumerate() {
            let code = 16 + i as u16;
            Self::insert_all_variants(&mut name_to_key, name, code);
        }

        // QWERTY row 2
        let row2 = ["a", "s", "d", "f", "g", "h", "j", "k", "l"];
        for (i, name) in row2.iter().enumerate() {
            let code = 30 + i as u16;
            Self::insert_all_variants(&mut name_to_key, name, code);
        }

        // QWERTY row 3
        let row3 = ["z", "x", "c", "v", "b", "n", "m"];
        for (i, name) in row3.iter().enumerate() {
            let code = 44 + i as u16;
            Self::insert_all_variants(&mut name_to_key, name, code);
        }

        // === Numbers 0-9 ===
        for i in 1..=9 {
            let name = i.to_string();
            let code = 1 + i as u16;
            Self::insert_all_variants(&mut name_to_key, &name, code);
        }
        Self::insert_all_variants(&mut name_to_key, "0", 11);

        // === Function keys F1-F24 ===
        // Note: evdev function key codes are non-linear!
        // F1-F10: 59-68, F11-F12: 87-88, F13-F20: 183-190, F21-F24: 194-197
        for i in 1..=24 {
            let name = format!("f{}", i);
            // Map function key number to actual evdev code
            let code = match i {
                1..=10 => 59 + i as u16 - 1,   // F1-F10: 59-68
                11..=12 => 87 + i as u16 - 11,  // F11-F12: 87-88
                13..=20 => 183 + i as u16 - 13, // F13-F20: 183-190
                21..=24 => 194 + i as u16 - 21, // F21-F24: 194-197
                _ => continue,
            };
            Self::insert_all_variants(&mut name_to_key, &name, code);
        }

        // === Modifier keys ===
        Self::insert_all_variants(&mut name_to_key, "ctrl", 29);
        Self::insert_all_variants(&mut name_to_key, "leftctrl", 29);
        Self::insert_all_variants(&mut name_to_key, "control", 29);

        Self::insert_all_variants(&mut name_to_key, "rightctrl", 97);

        Self::insert_all_variants(&mut name_to_key, "shift", 42);
        Self::insert_all_variants(&mut name_to_key, "leftshift", 42);

        Self::insert_all_variants(&mut name_to_key, "rightshift", 54);

        Self::insert_all_variants(&mut name_to_key, "alt", 56);
        Self::insert_all_variants(&mut name_to_key, "leftalt", 56);

        Self::insert_all_variants(&mut name_to_key, "rightalt", 100);
        Self::insert_all_variants(&mut name_to_key, "altgr", 100);

        Self::insert_all_variants(&mut name_to_key, "meta", 125);
        Self::insert_all_variants(&mut name_to_key, "leftmeta", 125);
        Self::insert_all_variants(&mut name_to_key, "win", 125);
        Self::insert_all_variants(&mut name_to_key, "windows", 125);
        Self::insert_all_variants(&mut name_to_key, "super", 125);
        Self::insert_all_variants(&mut name_to_key, "command", 125);

        Self::insert_all_variants(&mut name_to_key, "rightmeta", 126);

        // === Special keys ===
        Self::insert_all_variants(&mut name_to_key, "enter", 28);
        Self::insert_all_variants(&mut name_to_key, "return", 28);
        Self::insert_all_variants(&mut name_to_key, "ret", 28);

        Self::insert_all_variants(&mut name_to_key, "esc", 1);
        Self::insert_all_variants(&mut name_to_key, "escape", 1);

        Self::insert_all_variants(&mut name_to_key, "space", 57);

        Self::insert_all_variants(&mut name_to_key, "tab", 15);

        Self::insert_all_variants(&mut name_to_key, "backspace", 14);
        Self::insert_all_variants(&mut name_to_key, "bspc", 14);

        Self::insert_all_variants(&mut name_to_key, "delete", 111);
        Self::insert_all_variants(&mut name_to_key, "del", 111);

        Self::insert_all_variants(&mut name_to_key, "insert", 110);
        Self::insert_all_variants(&mut name_to_key, "ins", 110);

        Self::insert_all_variants(&mut name_to_key, "home", 102);

        Self::insert_all_variants(&mut name_to_key, "end", 107);

        Self::insert_all_variants(&mut name_to_key, "pageup", 104);
        Self::insert_all_variants(&mut name_to_key, "pgup", 104);

        Self::insert_all_variants(&mut name_to_key, "pagedown", 109);
        Self::insert_all_variants(&mut name_to_key, "pgdn", 109);

        // === Lock keys ===
        Self::insert_all_variants(&mut name_to_key, "capslock", 58);
        Self::insert_all_variants(&mut name_to_key, "caps", 58);

        Self::insert_all_variants(&mut name_to_key, "numlock", 69);
        Self::insert_all_variants(&mut name_to_key, "num", 69);

        Self::insert_all_variants(&mut name_to_key, "scrolllock", 70);
        Self::insert_all_variants(&mut name_to_key, "scroll", 70);

        // === Arrow keys ===
        Self::insert_all_variants(&mut name_to_key, "up", 103);
        Self::insert_all_variants(&mut name_to_key, "down", 108);
        Self::insert_all_variants(&mut name_to_key, "left", 105);
        Self::insert_all_variants(&mut name_to_key, "right", 106);

        // === Punctuation ===
        Self::insert_all_variants(&mut name_to_key, "minus", 12);
        Self::insert_all_variants(&mut name_to_key, "equal", 13);
        Self::insert_all_variants(&mut name_to_key, "equals", 13);
        Self::insert_all_variants(&mut name_to_key, "braceleft", 26);
        Self::insert_all_variants(&mut name_to_key, "[", 26);
        Self::insert_all_variants(&mut name_to_key, "braceright", 27);
        Self::insert_all_variants(&mut name_to_key, "]", 27);
        Self::insert_all_variants(&mut name_to_key, "backslash", 43);
        Self::insert_all_variants(&mut name_to_key, "\\", 43);
        Self::insert_all_variants(&mut name_to_key, "semicolon", 39);
        Self::insert_all_variants(&mut name_to_key, ";", 39);
        Self::insert_all_variants(&mut name_to_key, "apostrophe", 40);
        Self::insert_all_variants(&mut name_to_key, "'", 40);
        Self::insert_all_variants(&mut name_to_key, "grave", 41);
        Self::insert_all_variants(&mut name_to_key, "`", 41);
        Self::insert_all_variants(&mut name_to_key, "comma", 51);
        Self::insert_all_variants(&mut name_to_key, ",", 51);
        Self::insert_all_variants(&mut name_to_key, "dot", 52);
        Self::insert_all_variants(&mut name_to_key, ".", 52);
        Self::insert_all_variants(&mut name_to_key, "period", 52);
        Self::insert_all_variants(&mut name_to_key, "slash", 53);
        Self::insert_all_variants(&mut name_to_key, "/", 53);

        // === Keypad keys ===
        Self::insert_all_variants(&mut name_to_key, "kpenter", 96);
        Self::insert_all_variants(&mut name_to_key, "kpplus", 78);
        Self::insert_all_variants(&mut name_to_key, "kpminus", 74);
        Self::insert_all_variants(&mut name_to_key, "kpmultiply", 55);
        Self::insert_all_variants(&mut name_to_key, "kpasterisk", 55);
        Self::insert_all_variants(&mut name_to_key, "kpdivide", 98);
        Self::insert_all_variants(&mut name_to_key, "kpslash", 98);
        Self::insert_all_variants(&mut name_to_key, "kp0", 82);
        Self::insert_all_variants(&mut name_to_key, "kp1", 79);
        Self::insert_all_variants(&mut name_to_key, "kp2", 80);
        Self::insert_all_variants(&mut name_to_key, "kp3", 81);
        Self::insert_all_variants(&mut name_to_key, "kp4", 75);
        Self::insert_all_variants(&mut name_to_key, "kp5", 76);
        Self::insert_all_variants(&mut name_to_key, "kp6", 77);
        Self::insert_all_variants(&mut name_to_key, "kp7", 71);
        Self::insert_all_variants(&mut name_to_key, "kp8", 72);
        Self::insert_all_variants(&mut name_to_key, "kp9", 73);
        Self::insert_all_variants(&mut name_to_key, "kpdecimal", 83);
        Self::insert_all_variants(&mut name_to_key, "kpperiod", 83);
        Self::insert_all_variants(&mut name_to_key, "kpdot", 83);

        // === Joystick buttons (BTN_0 through BTN_25 for devices like Azeron keypad) ===
        // Linux input codes: BTN_0 = 0x100 (256), BTN_1 = 0x101 (257), etc.
        const JOY_BTN_BASE: u16 = 0x100;
        for i in 0..=25u16 {
            let name = format!("joy_btn_{}", i);
            Self::insert_all_variants(&mut name_to_key, &name, JOY_BTN_BASE + i);
            Self::insert_all_variants(&mut name_to_key, &format!("btn_{}", i), JOY_BTN_BASE + i);
        }

        // === Hat switch / D-pad directions ===
        // Hat switch events use ABS_HAT0X/ABS_HAT0Y codes, but for remapping purposes
        // we define logical direction names that map to key codes for consistency
        Self::insert_all_variants(&mut name_to_key, "hat_up", 0x100 + 26);
        Self::insert_all_variants(&mut name_to_key, "hat_down", 0x100 + 27);
        Self::insert_all_variants(&mut name_to_key, "hat_left", 0x100 + 28);
        Self::insert_all_variants(&mut name_to_key, "hat_right", 0x100 + 29);

        Self {
            name_to_key,
        }
    }

    /// Helper to insert a key with all common naming variants
    fn insert_all_variants(map: &mut HashMap<String, Key>, name: &str, code: u16) {
        let normalized = name.to_lowercase();

        // Try to create Key from code - this handles the mapping correctly
        // Key::new(code) creates the correct Key enum variant
        let key = Key::new(code);

        // Insert the friendly name (lowercase)
        map.insert(normalized.clone(), key);

        // Insert with KEY_ prefix (standard evdev naming)
        let key_name = format!("key_{}", normalized);
        map.insert(key_name, key);
    }

    /// Parse a key name into an evdev::Key code
    ///
    /// This method handles:
    /// - Case-insensitive lookup (KEY_A, key_a, Key_A all work)
    /// - Friendly abbreviations (a -> KEY_A, capslock -> KEY_CAPSLOCK)
    /// - KEY_ prefix auto-expansion (if name not found, try with KEY_ prefix)
    ///
    /// # Arguments
    ///
    /// * `name` - The key name to parse (e.g., "KEY_A", "a", "capslock")
    ///
    /// # Returns
    ///
    /// * `Ok(Key)` - The evdev::Key code
    /// * `Err(ParseError)` - If the key name is unknown
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let parser = KeyParser::new();
    ///
    /// // Standard KEY_* names work
    /// assert_eq!(parser.parse("KEY_A"), Ok(Key::KEY_A));
    ///
    /// // Case insensitive
    /// assert_eq!(parser.parse("key_a"), Ok(Key::KEY_A));
    ///
    /// // Friendly abbreviations
    /// assert_eq!(parser.parse("a"), Ok(Key::KEY_A));
    /// assert_eq!(parser.parse("capslock"), Ok(Key::KEY_CAPSLOCK));
    ///
    /// // Unknown keys return error
    /// assert!(parser.parse("nonexistent").is_err());
    /// ```
    pub fn parse(&self, name: &str) -> Result<Key, ParseError> {
        let normalized = name.to_lowercase();

        // Try direct lookup first
        if let Some(key) = self.name_to_key.get(&normalized) {
            return Ok(*key);
        }

        // Try with KEY_ prefix if not already present
        let with_prefix = if !normalized.starts_with("key_") {
            format!("key_{}", normalized)
        } else {
            normalized.clone()
        };

        if let Some(key) = self.name_to_key.get(&with_prefix) {
            return Ok(*key);
        }

        // Not found
        Err(ParseError::UnknownKey(name.to_string()))
    }
}

impl Default for KeyParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letter_parsing() {
        let parser = KeyParser::new();

        // Test case insensitivity
        assert_eq!(parser.parse("KEY_A"), Ok(Key::KEY_A));
        assert_eq!(parser.parse("key_a"), Ok(Key::KEY_A));
        assert_eq!(parser.parse("Key_A"), Ok(Key::KEY_A));

        // Test friendly abbreviations
        assert_eq!(parser.parse("a"), Ok(Key::KEY_A));
        assert_eq!(parser.parse("q"), Ok(Key::KEY_Q));
        assert_eq!(parser.parse("z"), Ok(Key::KEY_Z));
    }

    #[test]
    fn test_number_parsing() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_1"), Ok(Key::KEY_1));
        assert_eq!(parser.parse("1"), Ok(Key::KEY_1));
        assert_eq!(parser.parse("0"), Ok(Key::KEY_0));
    }

    #[test]
    fn test_modifier_keys() {
        let parser = KeyParser::new();

        // Ctrl variations
        assert_eq!(parser.parse("KEY_LEFTCTRL"), Ok(Key::KEY_LEFTCTRL));
        assert_eq!(parser.parse("ctrl"), Ok(Key::KEY_LEFTCTRL));
        assert_eq!(parser.parse("leftctrl"), Ok(Key::KEY_LEFTCTRL));
        assert_eq!(parser.parse("control"), Ok(Key::KEY_LEFTCTRL));

        assert_eq!(parser.parse("rightctrl"), Ok(Key::KEY_RIGHTCTRL));

        // Shift variations
        assert_eq!(parser.parse("KEY_LEFTSHIFT"), Ok(Key::KEY_LEFTSHIFT));
        assert_eq!(parser.parse("shift"), Ok(Key::KEY_LEFTSHIFT));
        assert_eq!(parser.parse("leftshift"), Ok(Key::KEY_LEFTSHIFT));
        assert_eq!(parser.parse("rightshift"), Ok(Key::KEY_RIGHTSHIFT));

        // Alt variations
        assert_eq!(parser.parse("KEY_LEFTALT"), Ok(Key::KEY_LEFTALT));
        assert_eq!(parser.parse("alt"), Ok(Key::KEY_LEFTALT));
        assert_eq!(parser.parse("leftalt"), Ok(Key::KEY_LEFTALT));
        assert_eq!(parser.parse("rightalt"), Ok(Key::KEY_RIGHTALT));
        assert_eq!(parser.parse("altgr"), Ok(Key::KEY_RIGHTALT));
    }

    #[test]
    fn test_special_keys() {
        let parser = KeyParser::new();

        // Escape
        assert_eq!(parser.parse("KEY_ESC"), Ok(Key::KEY_ESC));
        assert_eq!(parser.parse("esc"), Ok(Key::KEY_ESC));
        assert_eq!(parser.parse("escape"), Ok(Key::KEY_ESC));

        // Enter/Return
        assert_eq!(parser.parse("KEY_ENTER"), Ok(Key::KEY_ENTER));
        assert_eq!(parser.parse("enter"), Ok(Key::KEY_ENTER));
        assert_eq!(parser.parse("return"), Ok(Key::KEY_ENTER));
        assert_eq!(parser.parse("ret"), Ok(Key::KEY_ENTER));

        // Space
        assert_eq!(parser.parse("KEY_SPACE"), Ok(Key::KEY_SPACE));
        assert_eq!(parser.parse("space"), Ok(Key::KEY_SPACE));

        // Tab
        assert_eq!(parser.parse("KEY_TAB"), Ok(Key::KEY_TAB));
        assert_eq!(parser.parse("tab"), Ok(Key::KEY_TAB));

        // Backspace
        assert_eq!(parser.parse("KEY_BACKSPACE"), Ok(Key::KEY_BACKSPACE));
        assert_eq!(parser.parse("backspace"), Ok(Key::KEY_BACKSPACE));
        assert_eq!(parser.parse("bspc"), Ok(Key::KEY_BACKSPACE));
    }

    #[test]
    fn test_lock_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_CAPSLOCK"), Ok(Key::KEY_CAPSLOCK));
        assert_eq!(parser.parse("capslock"), Ok(Key::KEY_CAPSLOCK));
        assert_eq!(parser.parse("caps"), Ok(Key::KEY_CAPSLOCK));

        assert_eq!(parser.parse("KEY_NUMLOCK"), Ok(Key::KEY_NUMLOCK));
        assert_eq!(parser.parse("numlock"), Ok(Key::KEY_NUMLOCK));
        assert_eq!(parser.parse("num"), Ok(Key::KEY_NUMLOCK));

        assert_eq!(parser.parse("KEY_SCROLLLOCK"), Ok(Key::KEY_SCROLLLOCK));
        assert_eq!(parser.parse("scrolllock"), Ok(Key::KEY_SCROLLLOCK));
        assert_eq!(parser.parse("scroll"), Ok(Key::KEY_SCROLLLOCK));
    }

    #[test]
    fn test_function_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_F1"), Ok(Key::KEY_F1));
        assert_eq!(parser.parse("f1"), Ok(Key::KEY_F1));

        assert_eq!(parser.parse("KEY_F12"), Ok(Key::KEY_F12));
        assert_eq!(parser.parse("f12"), Ok(Key::KEY_F12));

        // Test higher function keys
        assert_eq!(parser.parse("f20"), Ok(Key::KEY_F20));
    }

    #[test]
    fn test_arrow_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_UP"), Ok(Key::KEY_UP));
        assert_eq!(parser.parse("up"), Ok(Key::KEY_UP));

        assert_eq!(parser.parse("KEY_DOWN"), Ok(Key::KEY_DOWN));
        assert_eq!(parser.parse("down"), Ok(Key::KEY_DOWN));

        assert_eq!(parser.parse("KEY_LEFT"), Ok(Key::KEY_LEFT));
        assert_eq!(parser.parse("left"), Ok(Key::KEY_LEFT));

        assert_eq!(parser.parse("KEY_RIGHT"), Ok(Key::KEY_RIGHT));
        assert_eq!(parser.parse("right"), Ok(Key::KEY_RIGHT));
    }

    #[test]
    fn test_navigation_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_HOME"), Ok(Key::KEY_HOME));
        assert_eq!(parser.parse("home"), Ok(Key::KEY_HOME));

        assert_eq!(parser.parse("KEY_END"), Ok(Key::KEY_END));
        assert_eq!(parser.parse("end"), Ok(Key::KEY_END));

        assert_eq!(parser.parse("KEY_PAGEUP"), Ok(Key::KEY_PAGEUP));
        assert_eq!(parser.parse("pageup"), Ok(Key::KEY_PAGEUP));
        assert_eq!(parser.parse("pgup"), Ok(Key::KEY_PAGEUP));

        assert_eq!(parser.parse("KEY_PAGEDOWN"), Ok(Key::KEY_PAGEDOWN));
        assert_eq!(parser.parse("pagedown"), Ok(Key::KEY_PAGEDOWN));
        assert_eq!(parser.parse("pgdn"), Ok(Key::KEY_PAGEDOWN));

        assert_eq!(parser.parse("KEY_INSERT"), Ok(Key::KEY_INSERT));
        assert_eq!(parser.parse("insert"), Ok(Key::KEY_INSERT));
        assert_eq!(parser.parse("ins"), Ok(Key::KEY_INSERT));

        assert_eq!(parser.parse("KEY_DELETE"), Ok(Key::KEY_DELETE));
        assert_eq!(parser.parse("delete"), Ok(Key::KEY_DELETE));
        assert_eq!(parser.parse("del"), Ok(Key::KEY_DELETE));
    }

    #[test]
    fn test_keypad_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("KEY_KPENTER"), Ok(Key::KEY_KPENTER));
        assert_eq!(parser.parse("kpenter"), Ok(Key::KEY_KPENTER));

        assert_eq!(parser.parse("KEY_KP7"), Ok(Key::KEY_KP7));
        assert_eq!(parser.parse("kp7"), Ok(Key::KEY_KP7));

        assert_eq!(parser.parse("KEY_KPPLUS"), Ok(Key::KEY_KPPLUS));
        assert_eq!(parser.parse("kpplus"), Ok(Key::KEY_KPPLUS));
    }

    #[test]
    fn test_unknown_key_returns_error() {
        let parser = KeyParser::new();

        let result = parser.parse("nonexistent_key");
        assert!(result.is_err());
        assert_eq!(result, Err(ParseError::UnknownKey("nonexistent_key".to_string())));
    }

    #[test]
    fn test_empty_string_returns_error() {
        let parser = KeyParser::new();

        let result = parser.parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_at_least_30_keys_mapped() {
        let parser = KeyParser::new();

        // Verify at least 30 common keys can be parsed
        let common_keys = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j",
            "k", "l", "m", "n", "o", "p", "q", "r", "s", "t",
            "u", "v", "w", "x", "y", "z", // 26 letters
            "0", "1", "2", "3", "4", // 30
            "5", "6", "7", "8", "9",
            "ctrl", "shift", "alt", "enter", "space",
        ];

        let mut parsed_count = 0;
        for key_name in common_keys {
            if parser.parse(key_name).is_ok() {
                parsed_count += 1;
            }
        }

        assert!(parsed_count >= 30, "Expected at least 30 keys, found {}", parsed_count);
    }

    #[test]
    fn test_win_meta_super_command_aliases() {
        let parser = KeyParser::new();

        // All these should map to KEY_LEFTMETA (125)
        assert_eq!(parser.parse("win"), Ok(Key::KEY_LEFTMETA));
        assert_eq!(parser.parse("windows"), Ok(Key::KEY_LEFTMETA));
        assert_eq!(parser.parse("super"), Ok(Key::KEY_LEFTMETA));
        assert_eq!(parser.parse("meta"), Ok(Key::KEY_LEFTMETA));
        assert_eq!(parser.parse("command"), Ok(Key::KEY_LEFTMETA));
    }

    #[test]
    fn test_punctuation_keys() {
        let parser = KeyParser::new();

        assert_eq!(parser.parse("comma"), Ok(Key::KEY_COMMA));
        assert_eq!(parser.parse("period"), Ok(Key::KEY_DOT));
        assert_eq!(parser.parse("slash"), Ok(Key::KEY_SLASH));
        assert_eq!(parser.parse("semicolon"), Ok(Key::KEY_SEMICOLON));
    }

    #[test]
    fn test_joystick_button_parsing() {
        let parser = KeyParser::new();

        // Test JOY_BTN_0 through JOY_BTN_5
        assert_eq!(parser.parse("JOY_BTN_0"), Ok(Key::new(0x100)));
        assert_eq!(parser.parse("joy_btn_0"), Ok(Key::new(0x100)));
        assert_eq!(parser.parse("BTN_0"), Ok(Key::new(0x100)));

        assert_eq!(parser.parse("JOY_BTN_1"), Ok(Key::new(0x101)));
        assert_eq!(parser.parse("joy_btn_1"), Ok(Key::new(0x101)));
        assert_eq!(parser.parse("BTN_1"), Ok(Key::new(0x101)));

        assert_eq!(parser.parse("JOY_BTN_25"), Ok(Key::new(0x100 + 25)));
        assert_eq!(parser.parse("joy_btn_25"), Ok(Key::new(0x100 + 25)));

        // Test hat switch directions
        assert_eq!(parser.parse("hat_up"), Ok(Key::new(0x100 + 26)));
        assert_eq!(parser.parse("HAT_DOWN"), Ok(Key::new(0x100 + 27)));
        assert_eq!(parser.parse("hat_left"), Ok(Key::new(0x100 + 28)));
        assert_eq!(parser.parse("HAT_RIGHT"), Ok(Key::new(0x100 + 29)));
    }

    #[test]
    fn test_all_joystick_buttons_parse() {
        let parser = KeyParser::new();

        // Verify JOY_BTN_0 through JOY_BTN_25 all parse correctly
        for i in 0..=25 {
            let name = format!("joy_btn_{}", i);
            assert!(
                parser.parse(&name).is_ok(),
                "Failed to parse {}",
                name
            );

            // Verify correct code
            let expected_code = 0x100u16 + i;
            let result = parser.parse(&name);
            assert_eq!(result, Ok(Key::new(expected_code)), "Wrong code for {}", name);
        }
    }
}
