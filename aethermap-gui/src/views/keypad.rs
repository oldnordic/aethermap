use crate::gui::{Message, State};
use crate::theme;
use iced::{
    widget::{button, column, container, row, svg, text, Space},
    Alignment, Element, Length,
};

/// Known device vendor IDs
const AZERON_VENDOR_ID: u16 = 0x16d0;
const RAZER_VENDOR_ID: u16 = 0x1532;

/// Razer Tartarus Chroma product ID
const RAZER_TARTARUS_CHROMA_PID: u16 = 0x0208;

/// Recognized device profiles for keypad layout rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceProfile {
    AzeronCyborg2,
    RazerTartarusChroma,
    Generic,
}

impl DeviceProfile {
    /// Detect device profile from vendor/product IDs
    pub fn from_vid_pid(vendor_id: u16, product_id: u16) -> Self {
        match (vendor_id, product_id) {
            (AZERON_VENDOR_ID, _) => Self::AzeronCyborg2,
            (RAZER_VENDOR_ID, RAZER_TARTARUS_CHROMA_PID) => Self::RazerTartarusChroma,
            // Other Razer keypads (Tartarus v2: 0x0045, Orbweaver: 0x0113, etc.)
            // fall through to generic for now — add specific profiles as needed
            _ => Self::Generic,
        }
    }

    /// Human-readable device name for the header
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AzeronCyborg2 => "Azeron Cyborg 2",
            Self::RazerTartarusChroma => "Razer Tartarus Chroma",
            Self::Generic => "Keypad",
        }
    }

    /// Path to device silhouette SVG in assets/
    pub fn svg_path(&self) -> Option<&'static str> {
        match self {
            Self::AzeronCyborg2 => Some("aethermap-gui/assets/device-azeron-cyborg2.svg"),
            Self::RazerTartarusChroma => {
                Some("aethermap-gui/assets/device-razer-tartarus-chroma.svg")
            }
            Self::Generic => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeypadButton {
    pub id: String,
    pub label: String,
    pub row: usize,
    pub col: usize,
    pub current_remap: Option<String>,
}

/// Select the appropriate keypad layout for a device profile
pub fn layout_for_profile(profile: DeviceProfile) -> Vec<KeypadButton> {
    match profile {
        DeviceProfile::AzeronCyborg2 => azeron_keypad_layout(),
        DeviceProfile::RazerTartarusChroma => razer_tartarus_chroma_layout(),
        DeviceProfile::Generic => azeron_keypad_layout(), // fallback
    }
}

/// Legacy entry point — callers that don't know the device type yet
pub fn azeron_keypad_layout() -> Vec<KeypadButton> {
    vec![
        // Row 0: Left Cluster (top)
        KeypadButton {
            id: "JOY_BTN_7".into(),
            label: "C1".into(),
            row: 0,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_8".into(),
            label: "C2".into(),
            row: 0,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_9".into(),
            label: "C3".into(),
            row: 0,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_10".into(),
            label: "C4".into(),
            row: 0,
            col: 3,
            current_remap: None,
        },
        // Row 1: Main Keypad (top)
        KeypadButton {
            id: "JOY_BTN_11".into(),
            label: "K1".into(),
            row: 1,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_12".into(),
            label: "K2".into(),
            row: 1,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_13".into(),
            label: "K3".into(),
            row: 1,
            col: 2,
            current_remap: None,
        },
        // Row 2: Main Keypad (bottom)
        KeypadButton {
            id: "JOY_BTN_14".into(),
            label: "K4".into(),
            row: 2,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_15".into(),
            label: "K5".into(),
            row: 2,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_16".into(),
            label: "K6".into(),
            row: 2,
            col: 2,
            current_remap: None,
        },
        // Row 3: Right Cluster
        KeypadButton {
            id: "JOY_BTN_17".into(),
            label: "C5".into(),
            row: 3,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_18".into(),
            label: "C6".into(),
            row: 3,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_19".into(),
            label: "C7".into(),
            row: 3,
            col: 2,
            current_remap: None,
        },
        // Row 4: Thumb buttons
        KeypadButton {
            id: "JOY_BTN_4".into(),
            label: "TT".into(),
            row: 4,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_5".into(),
            label: "TM".into(),
            row: 4,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_6".into(),
            label: "TB".into(),
            row: 4,
            col: 2,
            current_remap: None,
        },
        // Row 5: D-Pad
        KeypadButton {
            id: "JOY_BTN_0".into(),
            label: "DU".into(),
            row: 5,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_3".into(),
            label: "DL".into(),
            row: 6,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_1".into(),
            label: "DR".into(),
            row: 6,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_2".into(),
            label: "DD".into(),
            row: 7,
            col: 1,
            current_remap: None,
        },
        // Row 8: Modifiers
        KeypadButton {
            id: "JOY_BTN_20".into(),
            label: "M1".into(),
            row: 8,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_21".into(),
            label: "M2".into(),
            row: 8,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_22".into(),
            label: "M3".into(),
            row: 8,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_23".into(),
            label: "M4".into(),
            row: 8,
            col: 3,
            current_remap: None,
        },
        // Row 9: Actions
        KeypadButton {
            id: "JOY_BTN_24".into(),
            label: "A1".into(),
            row: 9,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_25".into(),
            label: "A2".into(),
            row: 9,
            col: 1,
            current_remap: None,
        },
    ]
}

/// Razer Tartarus Chroma layout
///
/// 25 anti-ghosted programmable keys (KEY_1..KEY_25) arranged as:
/// - 4 rows x 5 keys (01-20)
/// - Thumb pad: 8-way directional + scroll wheel
/// - Mode switch + Function button
///
/// In evdev, keys report as KEY_1 through KEY_25 for the main grid,
/// BTN_TRIGGER_HAPPY for thumb pad directions, and additional keys
/// for Mode/Fn.
pub fn razer_tartarus_chroma_layout() -> Vec<KeypadButton> {
    // Row 0: Keys 01-05
    let mut buttons = vec![
        KeypadButton {
            id: "KEY_1".into(),
            label: "01".into(),
            row: 0,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_2".into(),
            label: "02".into(),
            row: 0,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_3".into(),
            label: "03".into(),
            row: 0,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_4".into(),
            label: "04".into(),
            row: 0,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_5".into(),
            label: "05".into(),
            row: 0,
            col: 4,
            current_remap: None,
        },
    ];
    // Row 1: Keys 06-10
    buttons.extend([
        KeypadButton {
            id: "KEY_6".into(),
            label: "06".into(),
            row: 1,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_7".into(),
            label: "07".into(),
            row: 1,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_8".into(),
            label: "08".into(),
            row: 1,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_9".into(),
            label: "09".into(),
            row: 1,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_10".into(),
            label: "10".into(),
            row: 1,
            col: 4,
            current_remap: None,
        },
    ]);
    // Row 2: Keys 11-15
    buttons.extend([
        KeypadButton {
            id: "KEY_11".into(),
            label: "11".into(),
            row: 2,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_12".into(),
            label: "12".into(),
            row: 2,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_13".into(),
            label: "13".into(),
            row: 2,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_14".into(),
            label: "14".into(),
            row: 2,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_15".into(),
            label: "15".into(),
            row: 2,
            col: 4,
            current_remap: None,
        },
    ]);
    // Row 3: Keys 16-20
    buttons.extend([
        KeypadButton {
            id: "KEY_16".into(),
            label: "16".into(),
            row: 3,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_17".into(),
            label: "17".into(),
            row: 3,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_18".into(),
            label: "18".into(),
            row: 3,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_19".into(),
            label: "19".into(),
            row: 3,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_20".into(),
            label: "20".into(),
            row: 3,
            col: 4,
            current_remap: None,
        },
    ]);
    // Row 4: Thumb pad (8-way directional)
    buttons.extend([
        KeypadButton {
            id: "BTN_TRIGGER_HAPPY1".into(),
            label: "T\u{2191}".into(),
            row: 4,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "BTN_TRIGGER_HAPPY2".into(),
            label: "T\u{2190}".into(),
            row: 4,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "BTN_TRIGGER_HAPPY3".into(),
            label: "T\u{2193}".into(),
            row: 4,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "BTN_TRIGGER_HAPPY4".into(),
            label: "T\u{2192}".into(),
            row: 4,
            col: 3,
            current_remap: None,
        },
    ]);
    // Row 5: Mode + Fn buttons
    buttons.extend([
        KeypadButton {
            id: "KEY_21".into(),
            label: "Mode".into(),
            row: 5,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "KEY_22".into(),
            label: "Fn".into(),
            row: 5,
            col: 2,
            current_remap: None,
        },
    ]);

    buttons
}

pub fn format_remap_target(target: &str) -> String {
    if let Some(rest) = target.strip_prefix("KEY_") {
        match rest {
            "LEFTCTRL" => "LCtrl".to_string(),
            "RIGHTCTRL" => "RCtrl".to_string(),
            "LEFTSHIFT" => "LShft".to_string(),
            "RIGHTSHIFT" => "RShft".to_string(),
            "LEFTALT" => "LAlt".to_string(),
            "RIGHTALT" => "RAlt".to_string(),
            "LEFTMETA" => "LMeta".to_string(),
            "RIGHTMETA" => "RMeta".to_string(),
            "SPACE" => "Space".to_string(),
            "TAB" => "Tab".to_string(),
            "ENTER" => "Enter".to_string(),
            "ESC" => "Esc".to_string(),
            "BACKSPACE" => "Bksp".to_string(),
            "DELETE" => "Del".to_string(),
            "INSERT" => "Ins".to_string(),
            "HOME" => "Home".to_string(),
            "END" => "End".to_string(),
            "PAGEUP" => "PgUp".to_string(),
            "PAGEDOWN" => "PgDn".to_string(),
            "UP" => "\u{2191}".to_string(),
            "DOWN" => "\u{2193}".to_string(),
            "LEFT" => "\u{2190}".to_string(),
            "RIGHT" => "\u{2192}".to_string(),
            s if s.len() == 1 => s.to_uppercase(),
            s if s.starts_with('F') => format!("F{}", &s[1..]),
            _ => rest.to_string(),
        }
    } else if let Some(rest) = target.strip_prefix("BTN_") {
        match rest {
            "LEFT" => "LMB".to_string(),
            "RIGHT" => "RMB".to_string(),
            "MIDDLE" => "Mid".to_string(),
            "SIDE" => "Side".to_string(),
            "EXTRA" => "Extra".to_string(),
            "FORWARD" => "Fwd".to_string(),
            "BACK" => "Back".to_string(),
            _ => rest.to_string(),
        }
    } else if let Some(rest) = target.strip_prefix("REL_") {
        match rest {
            "WHEEL" => "Wheel".to_string(),
            "HWHEEL" => "HWheel".to_string(),
            _ => rest.to_string(),
        }
    } else {
        if target.len() > 6 {
            format!("{}...", &target[..6])
        } else {
            target.to_string()
        }
    }
}

/// Render the device silhouette SVG if available for the current profile
fn device_image(profile: DeviceProfile) -> Option<Element<'static, Message>> {
    let svg_path = profile.svg_path()?;
    let handle = svg::Handle::from_path(svg_path);
    Some(
        container(
            svg(handle)
                .width(Length::Fixed(280.0))
                .height(Length::Fixed(240.0)),
        )
        .center_x()
        .into(),
    )
}

pub fn view(state: &State) -> Element<'_, Message> {
    let layout = &state.keypad_layout;
    let profile = state.keypad_device_profile;

    // Count rows needed
    let max_row = layout.iter().map(|b| b.row).max().unwrap_or(0);
    let mut rows: Vec<Vec<Element<'_, Message>>> = (0..=max_row).map(|_| Vec::new()).collect();

    for keypad_button in layout {
        let button_id = keypad_button.id.clone();
        let label = keypad_button.label.clone();
        let remap = keypad_button.current_remap.clone();
        let is_selected = state.selected_button
            == Some(
                layout
                    .iter()
                    .position(|b| b.id == keypad_button.id)
                    .unwrap_or(usize::MAX),
            );

        let button_style = if is_selected {
            iced::theme::Button::Primary
        } else if remap.is_some() {
            iced::theme::Button::Secondary
        } else {
            iced::theme::Button::Text
        };

        let button_content: Element<'_, Message> = if let Some(ref target) = remap {
            let display_name = format_remap_target(target);
            container(
                column![
                    text(label)
                        .size(8)
                        .style(iced::theme::Text::Color(iced::Color::from_rgb(
                            0.5, 0.5, 0.5
                        ))),
                    text(display_name).size(11).width(Length::Fixed(45.0)),
                ]
                .spacing(2)
                .align_items(Alignment::Center),
            )
            .center_x()
            .center_y()
            .into()
        } else {
            container(text(label).size(12)).center_x().center_y().into()
        };

        let btn = button(button_content)
            .on_press(Message::SelectKeypadButton(button_id.clone()))
            .style(button_style)
            .padding([6, 8])
            .width(iced::Length::Fixed(54.0))
            .height(iced::Length::Fixed(54.0))
            .into();

        if let Some(row) = rows.get_mut(keypad_button.row) {
            row.push(btn);
        }
    }

    // Add hat switch indicator for Azeron (if profile uses analog hat)
    if profile == DeviceProfile::AzeronCyborg2 {
        let hat_switch = container(text("Hat\n\u{2195}").size(10))
            .width(iced::Length::Fixed(54.0))
            .height(iced::Length::Fixed(54.0))
            .center_x()
            .center_y()
            .style(theme::styles::card)
            .into();

        if let Some(row) = rows.get_mut(5) {
            row.push(hat_switch);
        }
    }

    let keypad_rows: Vec<Element<'_, Message>> = rows
        .into_iter()
        .filter(|r| !r.is_empty())
        .map(|row_elements| {
            row(row_elements)
                .spacing(4)
                .align_items(Alignment::Center)
                .into()
        })
        .collect();

    let mut content = column![
        text(format!("{} Layout", profile.display_name())).size(20),
        Space::with_height(10),
        text("Click a button to configure remapping").size(12),
    ]
    .spacing(10)
    .align_items(Alignment::Center);

    // Add device image if available
    if let Some(image) = device_image(profile) {
        content = content.push(Space::with_height(10)).push(image);
    }

    content = content.push(Space::with_height(20)).push(
        column(keypad_rows)
            .spacing(4)
            .align_items(Alignment::Center),
    );

    container(content)
        .padding(24)
        .width(Length::Fill)
        .center_x()
        .into()
}
