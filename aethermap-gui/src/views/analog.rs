use std::sync::Arc;
use std::time::Instant;
use iced::{
    widget::{button, column, container, pick_list, row, scrollable, slider, text, Space, Canvas},
    Element, Length, Alignment, Color,
};
use aethermap_common::{AnalogMode, CameraOutputMode};
use crate::gui::Message;
use crate::theme;
use crate::widgets::{AnalogVisualizer, CurveGraph, analog_visualizer::DeadzoneShape as WidgetDeadzoneShape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadzoneShape {
    Circular,
    Square,
}

impl std::fmt::Display for DeadzoneShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeadzoneShape::Circular => write!(f, "Circular"),
            DeadzoneShape::Square => write!(f, "Square"),
        }
    }
}

impl DeadzoneShape {
    pub const ALL: [DeadzoneShape; 2] = [DeadzoneShape::Circular, DeadzoneShape::Square];
}

impl Default for DeadzoneShape {
    fn default() -> Self {
        Self::Circular
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityCurve {
    Linear,
    Quadratic,
    Exponential,
}

impl std::fmt::Display for SensitivityCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensitivityCurve::Linear => write!(f, "Linear"),
            SensitivityCurve::Quadratic => write!(f, "Quadratic"),
            SensitivityCurve::Exponential => write!(f, "Exponential"),
        }
    }
}

impl SensitivityCurve {
    pub const ALL: [SensitivityCurve; 3] = [
        SensitivityCurve::Linear,
        SensitivityCurve::Quadratic,
        SensitivityCurve::Exponential,
    ];
}

impl Default for SensitivityCurve {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone)]
pub struct CalibrationConfig {
    pub deadzone: f32,
    pub deadzone_shape: String,
    pub sensitivity: String,
    pub sensitivity_multiplier: f32,
    pub range_min: i32,
    pub range_max: i32,
    pub invert_x: bool,
    pub invert_y: bool,
    pub exponent: f32,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            deadzone_shape: "circular".to_string(),
            sensitivity: "linear".to_string(),
            sensitivity_multiplier: 1.0,
            range_min: -32768,
            range_max: 32767,
            invert_x: false,
            invert_y: false,
            exponent: 2.0,
        }
    }
}

#[derive(Debug)]
pub struct AnalogCalibrationView {
    pub device_id: String,
    pub layer_id: usize,
    pub calibration: CalibrationConfig,
    pub deadzone_shape_selected: DeadzoneShape,
    pub sensitivity_curve_selected: SensitivityCurve,
    pub analog_mode_selected: AnalogMode,
    pub camera_mode_selected: CameraOutputMode,
    pub invert_x_checked: bool,
    pub invert_y_checked: bool,
    pub stick_x: f32,
    pub stick_y: f32,
    pub loading: bool,
    pub error: Option<String>,
    pub last_visualizer_update: Instant,
    pub visualizer_cache: Arc<iced::widget::canvas::Cache>,
}

impl Clone for AnalogCalibrationView {
    fn clone(&self) -> Self {
        Self {
            device_id: self.device_id.clone(),
            layer_id: self.layer_id,
            calibration: self.calibration.clone(),
            deadzone_shape_selected: self.deadzone_shape_selected,
            sensitivity_curve_selected: self.sensitivity_curve_selected,
            analog_mode_selected: self.analog_mode_selected,
            camera_mode_selected: self.camera_mode_selected,
            invert_x_checked: self.invert_x_checked,
            invert_y_checked: self.invert_y_checked,
            stick_x: self.stick_x,
            stick_y: self.stick_y,
            loading: self.loading,
            error: self.error.clone(),
            last_visualizer_update: Instant::now(),
            visualizer_cache: Arc::clone(&self.visualizer_cache),
        }
    }
}

impl Default for AnalogCalibrationView {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            layer_id: 0,
            calibration: CalibrationConfig::default(),
            deadzone_shape_selected: DeadzoneShape::Circular,
            sensitivity_curve_selected: SensitivityCurve::Linear,
            analog_mode_selected: AnalogMode::Disabled,
            camera_mode_selected: CameraOutputMode::Scroll,
            invert_x_checked: false,
            invert_y_checked: false,
            stick_x: 0.0,
            stick_y: 0.0,
            loading: false,
            error: None,
            last_visualizer_update: Instant::now(),
            visualizer_cache: Arc::new(iced::widget::canvas::Cache::default()),
        }
    }
}

impl AnalogCalibrationView {
    fn checkbox_button<'a>(&'a self, label: &str, is_checked: bool, msg: fn(bool) -> Message) -> Element<'a, Message> {
        let btn = if is_checked {
            button(text(format!("[X] {}", label)).size(14))
        } else {
            button(text(format!("[ ] {}", label)).size(14))
        };
        btn.on_press(msg(is_checked))
            .style(iced::theme::Button::Text)
            .into()
    }

    pub fn view(&self) -> Element<Message> {
        use iced::widget::{horizontal_rule as rule, Row, Column, container, Canvas};

        let title = text("Analog Calibration").size(24);
        let info = Column::new()
            .spacing(5)
            .push(text(format!("Device: {}", self.device_id)).size(14))
            .push(text(format!("Layer: {}", self.layer_id)).size(14));

        let visualizer_section = Column::new()
            .spacing(10)
            .push(text("Stick Position").size(18))
            .push(
                container(
                    Canvas::new(AnalogVisualizer {
                        stick_x: self.stick_x,
                        stick_y: self.stick_y,
                        deadzone: self.calibration.deadzone,
                        deadzone_shape: match self.deadzone_shape_selected {
                            DeadzoneShape::Circular => WidgetDeadzoneShape::Circular,
                            DeadzoneShape::Square => WidgetDeadzoneShape::Square,
                        },
                        range_min: self.calibration.range_min,
                        range_max: self.calibration.range_max,
                        cache: Arc::clone(&self.visualizer_cache),
                    })
                    .width(Length::Fixed(250.0))
                    .height(Length::Fixed(250.0))
                )
                .width(Length::Fixed(270.0))
                .height(Length::Fixed(270.0))
                .center_x()
                .center_y()
            );

        let mode_section = Column::new()
            .spacing(10)
            .push(text("Output Mode").size(18))
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Mode:"))
                    .push(pick_list(
                        &AnalogMode::ALL[..],
                        Some(self.analog_mode_selected),
                        Message::AnalogModeChanged,
                    ))
            );

        let mode_section = if self.analog_mode_selected == AnalogMode::Camera {
            mode_section.push(
                Row::new()
                    .spacing(10)
                    .push(text("Camera:"))
                    .push(pick_list(
                        &CameraOutputMode::ALL[..],
                        Some(self.camera_mode_selected),
                        Message::CameraModeChanged,
                    ))
            )
        } else {
            mode_section
        };

        let deadzone_section = Column::new()
            .spacing(10)
            .push(text("Deadzone").size(18))
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Size:"))
                    .push(text(format!("{:.0}%", self.calibration.deadzone * 100.0)))
                    .push(slider(0.0..=1.0, self.calibration.deadzone, Message::AnalogDeadzoneChanged).step(0.01))
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Shape:"))
                    .push(pick_list(
                        &DeadzoneShape::ALL[..],
                        Some(self.deadzone_shape_selected),
                        Message::AnalogDeadzoneShapeChanged,
                    ))
            );

        let sensitivity_section = Column::new()
            .spacing(10)
            .push(text("Sensitivity").size(18))
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Multiplier:"))
                    .push(text(format!("{:.1}", self.calibration.sensitivity_multiplier)))
                    .push(slider(0.1..=5.0, self.calibration.sensitivity_multiplier, Message::AnalogSensitivityChanged).step(0.1))
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Curve:"))
                    .push(pick_list(
                        &SensitivityCurve::ALL[..],
                        Some(self.sensitivity_curve_selected),
                        Message::AnalogSensitivityCurveChanged,
                    ))
            )
            .push(text(format!("Curve: {}", self.sensitivity_curve_selected)).size(14))
            .push(
                container(
                    Canvas::new(CurveGraph {
                        curve: self.sensitivity_curve_selected,
                        multiplier: self.calibration.sensitivity_multiplier,
                    })
                    .width(Length::Fixed(300.0))
                    .height(Length::Fixed(200.0))
                )
                .width(Length::Fixed(320.0))
                .center_x()
            );

        let range_section = Column::new()
            .spacing(10)
            .push(text("Output Range").size(18))
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Min:"))
                    .push(text(self.calibration.range_min.to_string()))
                    .push(slider(-32768..=0, self.calibration.range_min, Message::AnalogRangeMinChanged))
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(text("Max:"))
                    .push(text(self.calibration.range_max.to_string()))
                    .push(slider(0..=32767, self.calibration.range_max, Message::AnalogRangeMaxChanged))
            );

        let inversion_section = Column::new()
            .spacing(10)
            .push(text("Axis Inversion").size(18))
            .push(
                Row::new()
                    .spacing(20)
                    .push(self.checkbox_button("Invert X", self.invert_x_checked, Message::AnalogInvertXToggled))
                    .push(self.checkbox_button("Invert Y", self.invert_y_checked, Message::AnalogInvertYToggled))
            );

        let buttons = Row::new()
            .spacing(10)
            .push(button("Apply").on_press(Message::ApplyAnalogCalibration))
            .push(
                button("Close")
                    .on_press(Message::CloseAnalogCalibration)
                    .style(iced::theme::Button::Secondary)
            );

        let content = if let Some(error) = &self.error {
            Column::new()
                .spacing(20)
                .push(title)
                .push(info)
                .push(rule(1))
                .push(text(format!("Error: {}", error)).style(Color::from_rgb(1.0, 0.4, 0.4)))
                .push(buttons)
        } else {
            Column::new()
                .spacing(20)
                .push(title)
                .push(info)
                .push(rule(1))
                .push(visualizer_section)
                .push(rule(1))
                .push(mode_section)
                .push(rule(1))
                .push(deadzone_section)
                .push(rule(1))
                .push(sensitivity_section)
                .push(rule(1))
                .push(range_section)
                .push(rule(1))
                .push(inversion_section)
                .push(rule(1))
                .push(buttons)
        };

        scrollable(content).height(Length::Fill).into()
    }
}

pub fn overlay_view(state: &crate::gui::State) -> Option<Element<'_, Message>> {
    if let Some(ref view) = state.analog_calibration_view {
        let dialog = container(view.view())
            .max_width(600)
            .max_height(800)
            .style(theme::styles::card);

        Some(
            container(dialog)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .padding(40)
                .style(iced::theme::Container::Transparent)
                .into(),
        )
    } else {
        None
    }
}
