//! Analog stick calibration configuration
//!
//! This module defines the data structures for configuring analog stick processing:
//! - Deadzone filtering to remove drift
//! - Sensitivity curves for response feel
//! - Range scaling and inversion
//!
//! These types are used by the AnalogProcessor and the YAML config system.

use serde::{Deserialize, Serialize};

/// Deadzone shape for analog stick filtering
///
/// Determines how the deadzone is calculated from X/Y coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DeadzoneShape {
    /// Circular deadzone - smoother diagonal response
    ///
    /// The deadzone is calculated as sqrt(x^2 + y^2). This provides
    /// more natural movement for analog sticks where diagonal input
    /// is common.
    #[default]
    Circular,

    /// Square deadzone - precise axis-aligned movement
    ///
    /// The deadzone is calculated as max(|x|, |y|). This is useful
    /// when you want precise control on individual axes without
    /// diagonal "bleed-through".
    Square,
}

/// Sensitivity response curve type
///
/// Determines how input values are mapped to output values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SensitivityCurve {
    /// Linear response (1:1 mapping)
    ///
    /// Output is directly proportional to input. No transformation applied.
    #[default]
    Linear,

    /// Quadratic response (exponent = 2)
    ///
    /// Output follows x^2 curve. Provides gradual increase with more
    /// precision at low values and less sensitivity at high values.
    Quadratic,

    /// Exponential response (configurable exponent)
    ///
    /// Output follows x^exponent curve. Higher exponents provide more
    /// aggressive curves with increased low-precision range.
    Exponential { exponent: f32 },
}

/// Analog stick calibration configuration
///
/// Defines how analog stick events are processed including deadzone filtering,
/// sensitivity curves, range scaling, and axis inversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalogCalibration {
    /// Deadzone radius (0.0 to 1.0)
    ///
    /// Values below this threshold are filtered out. 0.15 (15%) is typical
    /// for analog sticks to remove drift without sacrificing usable range.
    #[serde(default = "default_deadzone")]
    pub deadzone: f32,

    /// Shape of the deadzone
    ///
    /// Circular is recommended for analog sticks (smoother diagonals).
    /// Square can be used for precise axis-aligned control.
    #[serde(default)]
    pub deadzone_shape: DeadzoneShape,

    /// Sensitivity response curve
    ///
    /// Determines the feel of the analog stick response.
    #[serde(default)]
    pub sensitivity: SensitivityCurve,

    /// Sensitivity multiplier (0.1 to 5.0)
    ///
    /// Scales the output after curve application. 1.0 = no scaling,
    /// > 1.0 = more sensitive, < 1.0 = less sensitive.
    #[serde(default = "default_sensitivity_multiplier")]
    pub sensitivity_multiplier: f32,

    /// Minimum output value
    ///
    /// Default -32768 (Linux input minimum). Can be adjusted for
    /// specific use cases (e.g., gamepad emulation with different ranges).
    #[serde(default = "default_range_min")]
    pub range_min: i32,

    /// Maximum output value
    ///
    /// Default 32767 (Linux input maximum). Can be adjusted for
    /// specific use cases.
    #[serde(default = "default_range_max")]
    pub range_max: i32,

    /// Invert X axis
    ///
    /// When true, X-axis input is reversed (left becomes right).
    #[serde(default)]
    pub invert_x: bool,

    /// Invert Y axis
    ///
    /// When true, Y-axis input is reversed (up becomes down).
    /// This is commonly needed for Y-axis in camera controls.
    #[serde(default)]
    pub invert_y: bool,
}

// Default value functions

fn default_deadzone() -> f32 {
    0.15 // 15% deadzone typical for analog sticks
}

fn default_sensitivity_multiplier() -> f32 {
    1.0 // No scaling by default
}

fn default_range_min() -> i32 {
    -32768 // Linux input minimum
}

fn default_range_max() -> i32 {
    32767 // Linux input maximum
}

impl Default for AnalogCalibration {
    fn default() -> Self {
        Self {
            deadzone: default_deadzone(),
            deadzone_shape: DeadzoneShape::default(),
            sensitivity: SensitivityCurve::default(),
            sensitivity_multiplier: default_sensitivity_multiplier(),
            range_min: default_range_min(),
            range_max: default_range_max(),
            invert_x: false,
            invert_y: false,
        }
    }
}

impl AnalogCalibration {
    /// Create a new calibration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new calibration with a specific deadzone
    pub fn with_deadzone(deadzone: f32) -> Self {
        Self {
            deadzone,
            ..Default::default()
        }
    }

    /// Create a new calibration with a specific sensitivity curve
    pub fn with_sensitivity_curve(curve: SensitivityCurve) -> Self {
        Self {
            sensitivity: curve,
            ..Default::default()
        }
    }

    /// Check if calibration values are within valid ranges
    ///
    /// Returns Ok(()) if valid, Err with description if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.deadzone < 0.0 || self.deadzone > 1.0 {
            return Err(format!(
                "Deadzone must be between 0.0 and 1.0, got {}",
                self.deadzone
            ));
        }

        if self.sensitivity_multiplier < 0.1 || self.sensitivity_multiplier > 5.0 {
            return Err(format!(
                "Sensitivity multiplier must be between 0.1 and 5.0, got {}",
                self.sensitivity_multiplier
            ));
        }

        if self.range_min >= self.range_max {
            return Err(format!(
                "Range min ({}) must be less than range max ({})",
                self.range_min, self.range_max
            ));
        }

        // Validate exponent if present
        if let SensitivityCurve::Exponential { exponent } = self.sensitivity {
            if !(0.1..=10.0).contains(&exponent) {
                return Err(format!(
                    "Exponential curve exponent must be between 0.1 and 10.0, got {}",
                    exponent
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_default_calibration() {
        let calib = AnalogCalibration::default();

        assert_eq!(calib.deadzone, 0.15);
        assert_eq!(calib.deadzone_shape, DeadzoneShape::Circular);
        assert_eq!(calib.sensitivity, SensitivityCurve::Linear);
        assert_eq!(calib.sensitivity_multiplier, 1.0);
        assert_eq!(calib.range_min, -32768);
        assert_eq!(calib.range_max, 32767);
        assert!(!calib.invert_x);
        assert!(!calib.invert_y);
    }

    #[test]
    fn test_new_calibration() {
        let calib = AnalogCalibration::new();

        assert_eq!(calib.deadzone, 0.15);
        assert_eq!(calib.sensitivity, SensitivityCurve::Linear);
    }

    #[test]
    fn test_with_deadzone() {
        let calib = AnalogCalibration::with_deadzone(0.25);

        assert_eq!(calib.deadzone, 0.25);
        assert_eq!(calib.sensitivity, SensitivityCurve::Linear);
        assert_eq!(calib.deadzone_shape, DeadzoneShape::Circular);
    }

    #[test]
    fn test_with_sensitivity_curve() {
        let calib = AnalogCalibration::with_sensitivity_curve(SensitivityCurve::Quadratic);

        assert_eq!(calib.sensitivity, SensitivityCurve::Quadratic);
        assert_eq!(calib.deadzone, 0.15);
    }

    #[test]
    fn test_deadzone_shape_serialization() {
        // Circular should serialize to "circular"
        let circular = DeadzoneShape::Circular;
        let serialized = serde_yaml::to_string(&circular).unwrap();
        assert!(serialized.contains("circular"));

        // Square should serialize to "square"
        let square = DeadzoneShape::Square;
        let serialized = serde_yaml::to_string(&square).unwrap();
        assert!(serialized.contains("square"));
    }

    #[test]
    fn test_sensitivity_curve_serialization() {
        // Linear
        let linear = SensitivityCurve::Linear;
        let serialized = serde_yaml::to_string(&linear).unwrap();
        assert!(serialized.contains("linear"));

        // Quadratic
        let quad = SensitivityCurve::Quadratic;
        let serialized = serde_yaml::to_string(&quad).unwrap();
        assert!(serialized.contains("quadratic"));

        // Exponential
        let exp = SensitivityCurve::Exponential { exponent: 2.5 };
        let serialized = serde_yaml::to_string(&exp).unwrap();
        assert!(serialized.contains("exponential"));
    }

    #[test]
    fn test_calibration_yaml_roundtrip() {
        let original = AnalogCalibration {
            deadzone: 0.20,
            deadzone_shape: DeadzoneShape::Square,
            sensitivity: SensitivityCurve::Quadratic,
            sensitivity_multiplier: 1.5,
            range_min: -16384,
            range_max: 16383,
            invert_x: true,
            invert_y: false,
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: AnalogCalibration = serde_yaml::from_str(&yaml).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.deadzone, original.deadzone);
        assert_eq!(deserialized.deadzone_shape, original.deadzone_shape);
        assert_eq!(deserialized.sensitivity, original.sensitivity);
        assert_eq!(
            deserialized.sensitivity_multiplier,
            original.sensitivity_multiplier
        );
        assert_eq!(deserialized.range_min, original.range_min);
        assert_eq!(deserialized.range_max, original.range_max);
        assert_eq!(deserialized.invert_x, original.invert_x);
        assert_eq!(deserialized.invert_y, original.invert_y);
    }

    #[test]
    fn test_calibration_yaml_with_defaults() {
        let calib = AnalogCalibration::default();
        let yaml = serde_yaml::to_string(&calib).unwrap();

        // Default values should serialize compactly with serde(default)
        // Only non-default values would typically appear in a config file
        let deserialized: AnalogCalibration = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized, calib);
    }

    #[test]
    fn test_deadzone_bounds_valid() {
        let calib = AnalogCalibration::default();
        assert!(calib.validate().is_ok());
    }

    #[test]
    fn test_deadzone_bounds_invalid_low() {
        let mut calib = AnalogCalibration::default();
        calib.deadzone = -0.1;
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_deadzone_bounds_invalid_high() {
        let mut calib = AnalogCalibration::default();
        calib.deadzone = 1.5;
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_sensitivity_multiplier_bounds_valid() {
        let mut calib = AnalogCalibration::default();
        calib.sensitivity_multiplier = 2.5;
        assert!(calib.validate().is_ok());
    }

    #[test]
    fn test_sensitivity_multiplier_bounds_invalid_low() {
        let mut calib = AnalogCalibration::default();
        calib.sensitivity_multiplier = 0.05;
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_sensitivity_multiplier_bounds_invalid_high() {
        let mut calib = AnalogCalibration::default();
        calib.sensitivity_multiplier = 10.0;
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_inversion_defaults() {
        let calib = AnalogCalibration::default();
        assert!(!calib.invert_x);
        assert!(!calib.invert_y);
    }

    #[test]
    fn test_exponential_curve_validation() {
        let mut calib = AnalogCalibration::default();
        calib.sensitivity = SensitivityCurve::Exponential { exponent: 2.0 };
        assert!(calib.validate().is_ok());

        calib.sensitivity = SensitivityCurve::Exponential { exponent: 0.05 };
        assert!(calib.validate().is_err());

        calib.sensitivity = SensitivityCurve::Exponential { exponent: 15.0 };
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_range_validation() {
        let mut calib = AnalogCalibration::default();
        assert!(calib.validate().is_ok());

        // Invalid: min >= max
        calib.range_min = 32767;
        calib.range_max = 32767;
        assert!(calib.validate().is_err());

        calib.range_min = 40000;
        calib.range_max = 30000;
        assert!(calib.validate().is_err());
    }

    #[test]
    fn test_partial_yaml_deserialization() {
        // YAML with only some fields specified
        let yaml = r#"
deadzone: 0.25
sensitivity_multiplier: 2.0
"#;

        let calib: AnalogCalibration = serde_yaml::from_str(yaml).unwrap();

        // Specified values
        assert_eq!(calib.deadzone, 0.25);
        assert_eq!(calib.sensitivity_multiplier, 2.0);

        // Default values
        assert_eq!(calib.deadzone_shape, DeadzoneShape::Circular);
        assert_eq!(calib.sensitivity, SensitivityCurve::Linear);
        assert_eq!(calib.range_min, -32768);
        assert_eq!(calib.range_max, 32767);
        assert!(!calib.invert_x);
        assert!(!calib.invert_y);
    }

    #[test]
    fn test_exponential_with_default_exponent() {
        // Test that exponential curve with custom exponent serializes/deserializes correctly
        let original = AnalogCalibration {
            sensitivity: SensitivityCurve::Exponential { exponent: 3.0 },
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&original).unwrap();

        // Verify it contains the exponential tag and exponent
        assert!(yaml.contains("!exponential"));
        assert!(yaml.contains("exponent: 3"));

        // Deserialize it back
        let calib: AnalogCalibration = serde_yaml::from_str(&yaml).unwrap();

        match calib.sensitivity {
            SensitivityCurve::Exponential { exponent } => {
                assert_eq!(exponent, 3.0);
            }
            _ => panic!("Expected Exponential curve"),
        }
    }
}
