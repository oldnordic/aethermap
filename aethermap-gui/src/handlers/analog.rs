use std::sync::Arc;
use std::time::{Duration, Instant};
use iced::Command;
use aethermap_common::{AnalogMode, CameraOutputMode};
use crate::gui::{State, Message};
use crate::views::analog::{AnalogCalibrationView, CalibrationConfig, DeadzoneShape, SensitivityCurve};

pub fn open(state: &mut State, device_id: String, layer_id: usize) -> Command<Message> {
    state.analog_calibration_view = Some(AnalogCalibrationView {
        device_id: device_id.clone(),
        layer_id,
        calibration: CalibrationConfig::default(),
        deadzone_shape_selected: DeadzoneShape::Circular,
        sensitivity_curve_selected: SensitivityCurve::Linear,
        analog_mode_selected: AnalogMode::Disabled,
        camera_mode_selected: CameraOutputMode::Scroll,
        invert_x_checked: false,
        invert_y_checked: false,
        stick_x: 0.0,
        stick_y: 0.0,
        loading: true,
        error: None,
        last_visualizer_update: Instant::now(),
        visualizer_cache: Arc::new(iced::widget::canvas::Cache::default()),
    });

    let device_id_clone = device_id.clone();
    let socket_path = state.socket_path.clone();
    let device_id_subscribe = device_id.clone();
    let socket_path_subscribe = state.socket_path.clone();

    Command::batch(vec![
        Command::perform(
            async move {
                let client = crate::ipc::IpcClient::new(socket_path_subscribe);
                client.subscribe_analog_input(&device_id_subscribe).await
            },
            |result| match result {
                Ok(_) => Message::ShowNotification("Subscribed to analog input".to_string(), false),
                Err(e) => Message::ShowNotification(format!("Subscription failed: {}", e), true),
            },
        ),
        Command::perform(
            async move {
                let client = crate::ipc::IpcClient::new(socket_path);
                client.get_analog_calibration(&device_id_clone, layer_id).await
            },
            Message::AnalogCalibrationLoaded,
        ),
    ])
}

pub fn loaded(state: &mut State, calibration: aethermap_common::AnalogCalibrationConfig) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.calibration = CalibrationConfig {
            deadzone: calibration.deadzone,
            deadzone_shape: calibration.deadzone_shape.clone(),
            sensitivity: calibration.sensitivity.clone(),
            sensitivity_multiplier: calibration.sensitivity_multiplier,
            range_min: calibration.range_min,
            range_max: calibration.range_max,
            invert_x: calibration.invert_x,
            invert_y: calibration.invert_y,
            exponent: calibration.exponent,
        };
        view.loading = false;
        view.deadzone_shape_selected = match calibration.deadzone_shape.as_str() {
            "circular" => DeadzoneShape::Circular,
            "square" => DeadzoneShape::Square,
            _ => DeadzoneShape::Circular,
        };
        view.sensitivity_curve_selected = match calibration.sensitivity.as_str() {
            "linear" => SensitivityCurve::Linear,
            "quadratic" => SensitivityCurve::Quadratic,
            "exponential" => SensitivityCurve::Exponential,
            _ => SensitivityCurve::Linear,
        };
        view.invert_x_checked = calibration.invert_x;
        view.invert_y_checked = calibration.invert_y;
    }
    Command::none()
}

pub fn load_error(state: &mut State, error: String) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.error = Some(error);
        view.loading = false;
    }
    Command::none()
}

pub fn deadzone_changed(state: &mut State, value: f32) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.calibration.deadzone = value;
        view.visualizer_cache.clear();
    }
    Command::none()
}

pub fn deadzone_shape_changed(state: &mut State, shape: DeadzoneShape) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.deadzone_shape_selected = shape;
        view.calibration.deadzone_shape = shape.to_string().to_lowercase();
        view.visualizer_cache.clear();
    }
    Command::none()
}

pub fn sensitivity_changed(state: &mut State, value: f32) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.calibration.sensitivity_multiplier = value;
    }
    Command::none()
}

pub fn sensitivity_curve_changed(state: &mut State, curve: SensitivityCurve) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.sensitivity_curve_selected = curve;
        view.calibration.sensitivity = curve.to_string().to_lowercase();
    }
    Command::none()
}

pub fn range_min_changed(state: &mut State, value: i32) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.calibration.range_min = value;
    }
    Command::none()
}

pub fn range_max_changed(state: &mut State, value: i32) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.calibration.range_max = value;
    }
    Command::none()
}

pub fn invert_x_toggled(state: &mut State, checked: bool) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.invert_x_checked = checked;
        view.calibration.invert_x = checked;
    }
    Command::none()
}

pub fn invert_y_toggled(state: &mut State, checked: bool) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.invert_y_checked = checked;
        view.calibration.invert_y = checked;
    }
    Command::none()
}

pub fn analog_mode_changed(state: &mut State, mode: AnalogMode) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.analog_mode_selected = mode;
    }
    Command::none()
}

pub fn camera_mode_changed(state: &mut State, mode: CameraOutputMode) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        view.camera_mode_selected = mode;
    }
    Command::none()
}

pub fn apply(state: &mut State) -> Command<Message> {
    if let Some(view) = state.analog_calibration_view.clone() {
        let device_id = view.device_id.clone();
        let layer_id = view.layer_id;
        let calibration = aethermap_common::AnalogCalibrationConfig {
            deadzone: view.calibration.deadzone,
            deadzone_shape: view.calibration.deadzone_shape.clone(),
            sensitivity: view.calibration.sensitivity.clone(),
            sensitivity_multiplier: view.calibration.sensitivity_multiplier,
            range_min: view.calibration.range_min,
            range_max: view.calibration.range_max,
            invert_x: view.calibration.invert_x,
            invert_y: view.calibration.invert_y,
            exponent: view.calibration.exponent,
            analog_mode: view.analog_mode_selected,
            camera_output_mode: if view.analog_mode_selected == aethermap_common::AnalogMode::Camera {
                Some(view.camera_mode_selected)
            } else {
                None
            },
        };
        let socket_path = state.socket_path.clone();

        return Command::perform(
            async move {
                let client = crate::ipc::IpcClient::new(socket_path);
                client.set_analog_calibration(&device_id, layer_id, calibration).await
                    .map_err(|e| e.to_string())
            },
            Message::AnalogCalibrationApplied,
        );
    }
    Command::none()
}

pub fn applied_ok(state: &mut State) -> Command<Message> {
    state.add_notification("Calibration saved successfully", false);
    Command::none()
}

pub fn applied_error(state: &mut State, error: String) -> Command<Message> {
    state.add_notification(&format!("Failed to save calibration: {}", error), true);
    if let Some(view) = &mut state.analog_calibration_view {
        let mut view = view.clone();
        view.error = Some(error);
        state.analog_calibration_view = Some(view);
    }
    Command::none()
}

pub fn close(state: &mut State) -> Command<Message> {
    let device_id = state.analog_calibration_view.as_ref()
        .map(|v| v.device_id.clone())
        .unwrap_or_default();
    let socket_path = state.socket_path.clone();

    state.analog_calibration_view = None;

    let _ = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let client = crate::ipc::IpcClient::new(socket_path);
            if let Err(e) = client.unsubscribe_analog_input(&device_id).await {
                eprintln!("Failed to unsubscribe: {}", e);
            }
        });
    });

    Command::none()
}

pub fn input_updated(state: &mut State, x: f32, y: f32) -> Command<Message> {
    if let Some(view) = &mut state.analog_calibration_view {
        if view.last_visualizer_update.elapsed() >= Duration::from_millis(33) {
            view.stick_x = x;
            view.stick_y = y;
            view.last_visualizer_update = Instant::now();
        }
    }
    Command::none()
}
