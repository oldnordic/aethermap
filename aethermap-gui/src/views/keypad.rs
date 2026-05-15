use crate::gui::{Message, State};
use crate::theme;
use iced::{
    widget::{button, column, container, row, text, Space},
    Alignment, Element, Length,
};

#[derive(Debug, Clone)]
pub struct KeypadButton {
    pub id: String,
    pub label: String,
    pub row: usize,
    #[allow(dead_code)]
    pub col: usize,
    pub current_remap: Option<String>,
}

pub fn azeron_keypad_layout() -> Vec<KeypadButton> {
    vec![
        KeypadButton {
            id: "JOY_BTN_0".to_string(),
            label: "1".to_string(),
            row: 0,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_1".to_string(),
            label: "2".to_string(),
            row: 0,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_2".to_string(),
            label: "3".to_string(),
            row: 0,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_3".to_string(),
            label: "4".to_string(),
            row: 0,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_4".to_string(),
            label: "5".to_string(),
            row: 0,
            col: 4,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_5".to_string(),
            label: "Q".to_string(),
            row: 2,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_6".to_string(),
            label: "W".to_string(),
            row: 2,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_7".to_string(),
            label: "E".to_string(),
            row: 2,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_8".to_string(),
            label: "R".to_string(),
            row: 2,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_9".to_string(),
            label: "A".to_string(),
            row: 3,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_10".to_string(),
            label: "S".to_string(),
            row: 3,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_11".to_string(),
            label: "D".to_string(),
            row: 3,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_12".to_string(),
            label: "F".to_string(),
            row: 3,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_13".to_string(),
            label: "Z".to_string(),
            row: 4,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_14".to_string(),
            label: "X".to_string(),
            row: 4,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_15".to_string(),
            label: "C".to_string(),
            row: 4,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_16".to_string(),
            label: "V".to_string(),
            row: 4,
            col: 3,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_17".to_string(),
            label: "6".to_string(),
            row: 0,
            col: 5,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_18".to_string(),
            label: "7".to_string(),
            row: 1,
            col: 5,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_19".to_string(),
            label: "8".to_string(),
            row: 2,
            col: 5,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_20".to_string(),
            label: "9".to_string(),
            row: 3,
            col: 5,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_21".to_string(),
            label: "0".to_string(),
            row: 4,
            col: 5,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_22".to_string(),
            label: "TL".to_string(),
            row: 6,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_23".to_string(),
            label: "TM".to_string(),
            row: 6,
            col: 1,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_24".to_string(),
            label: "TR".to_string(),
            row: 6,
            col: 2,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_25".to_string(),
            label: "BL".to_string(),
            row: 7,
            col: 0,
            current_remap: None,
        },
        KeypadButton {
            id: "JOY_BTN_26".to_string(),
            label: "BR".to_string(),
            row: 7,
            col: 1,
            current_remap: None,
        },
    ]
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

pub fn view(state: &State) -> Element<'_, Message> {
    let layout = azeron_keypad_layout();

    let mut rows: Vec<Vec<Element<'_, Message>>> = Vec::with_capacity(10);
    for _ in 0..10 {
        rows.push(Vec::new());
    }

    for keypad_button in &layout {
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

        if rows.get_mut(keypad_button.row).is_some() {
            rows[keypad_button.row].push(btn);
        }
    }

    let hat_switch = container(text("Hat\n\u{2195}").size(10))
        .width(iced::Length::Fixed(54.0))
        .height(iced::Length::Fixed(54.0))
        .center_x()
        .center_y()
        .style(theme::styles::card)
        .into();

    if rows.get_mut(5).is_some() {
        rows[5].push(hat_switch);
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

    let keypad_content = column![
        text("Azeron Keypad Layout").size(20),
        Space::with_height(10),
        text("Click a button to configure remapping").size(12),
        Space::with_height(20),
    ]
    .spacing(10)
    .align_items(Alignment::Center)
    .push(
        column(keypad_rows)
            .spacing(4)
            .align_items(Alignment::Center),
    );

    container(keypad_content)
        .padding(24)
        .width(Length::Fill)
        .center_x()
        .into()
}
