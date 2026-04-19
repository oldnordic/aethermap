//! Canvas-based sensitivity curve graph widget
//!
//! CurveGraph visualizes how the sensitivity curve transforms analog input.
//! Shows input (0-1) on X-axis and output (0-1) on Y-axis with the selected
//! curve shape plotted.

use iced::widget::canvas::{self, event, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Point, Rectangle};
use iced::mouse;

use crate::gui::SensitivityCurve;

/// Canvas widget that plots the sensitivity curve
///
/// Displays a graph showing how input values are transformed by the selected
/// sensitivity curve. X-axis represents input (0-1), Y-axis represents output (0-1).
pub struct CurveGraph {
    /// The sensitivity curve to plot
    pub curve: SensitivityCurve,
    /// Sensitivity multiplier (for display reference only)
    pub multiplier: f32,
}

impl CurveGraph {
    /// Create a new curve graph with the specified curve and multiplier
    pub fn new(curve: SensitivityCurve, multiplier: f32) -> Self {
        Self { curve, multiplier }
    }

    /// Apply the sensitivity curve to an input value
    ///
    /// This matches the daemon's curve application logic for visualization.
    /// The graph shows the normalized curve (0-1 range), not scaled by multiplier.
    pub fn apply_curve(input: f32, curve: &SensitivityCurve) -> f32 {
        match curve {
            SensitivityCurve::Linear => input,
            SensitivityCurve::Quadratic => input * input,
            // GUI's Exponential has no exponent field, use default 2.0
            SensitivityCurve::Exponential => {
                if input >= 0.0 {
                    input.powf(2.0)
                } else {
                    -(-input).powf(2.0)
                }
            }
        }
    }
}

impl<Message> Program<Message> for CurveGraph {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        _event: canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> (event::Status, Option<Message>) {
        (event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Graph margins for axes labels
        let margin = 20.0;
        let graph_width = bounds.width - 2.0 * margin;
        let graph_height = bounds.height - 2.0 * margin;

        // Origin point (bottom-left of graph area)
        let origin = Point::new(margin, bounds.height - margin);

        // Draw X and Y axes as white lines
        let x_axis = Path::line(
            Point::new(margin, bounds.height - margin),
            Point::new(bounds.width - margin, bounds.height - margin),
        );
        let y_axis = Path::line(
            Point::new(margin, margin),
            Point::new(margin, bounds.height - margin),
        );
        frame.stroke(
            &x_axis,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(2.0),
        );
        frame.stroke(
            &y_axis,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(2.0),
        );

        // Generate curve points (51 points for smooth curve: 0.0, 0.02, 0.04, ..., 1.0)
        let num_points = 51;
        let points: Vec<Point> = (0..num_points)
            .map(|i| {
                let input = i as f32 / (num_points - 1) as f32; // 0.0 to 1.0
                let output = Self::apply_curve(input, &self.curve);

                // Convert to screen coordinates
                Point::new(
                    origin.x + input * graph_width,
                    origin.y - output * graph_height,
                )
            })
            .collect();

        // Draw curve as connected line segments in green
        for window in points.windows(2) {
            let segment = Path::line(window[0], window[1]);
            frame.stroke(
                &segment,
                Stroke::default()
                    .with_color(Color::from_rgb(0.3, 0.8, 0.3))
                    .with_width(2.0),
            );
        }

        // Add optional "clamped" indicator if multiplier > 1.0
        if self.multiplier > 1.0 {
            // Draw a subtle warning indicator at the top of the graph
            let clamped_y = origin.y - graph_height;
            let indicator = Path::line(
                Point::new(bounds.width - margin - 30.0, clamped_y + 5.0),
                Point::new(bounds.width - margin, clamped_y + 5.0),
            );
            frame.stroke(
                &indicator,
                Stroke::default()
                    .with_color(Color::from_rgb(0.8, 0.5, 0.0))
                    .with_width(3.0),
            );
        }

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_curve_linear() {
        // Linear: output = input
        assert!((CurveGraph::apply_curve(0.0, &SensitivityCurve::Linear) - 0.0).abs() < f32::EPSILON);
        assert!((CurveGraph::apply_curve(0.5, &SensitivityCurve::Linear) - 0.5).abs() < f32::EPSILON);
        assert!((CurveGraph::apply_curve(1.0, &SensitivityCurve::Linear) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_curve_quadratic() {
        // Quadratic: output = input^2
        let result = CurveGraph::apply_curve(0.5, &SensitivityCurve::Quadratic);
        assert!((result - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_apply_curve_exponential() {
        // Exponential: output = input^2 (GUI uses fixed exponent 2.0)
        let result = CurveGraph::apply_curve(0.5, &SensitivityCurve::Exponential);
        assert!((result - 0.25).abs() < 0.001); // 0.5^2 = 0.25
    }

    #[test]
    fn test_apply_curve_with_multiplier() {
        // Multiplier is separate from curve calculation
        // apply_curve doesn't use multiplier, but we verify it exists on the struct
        let graph = CurveGraph::new(SensitivityCurve::Linear, 2.0);
        assert_eq!(graph.multiplier, 2.0);
    }

    #[test]
    fn test_apply_curve_negative_input_quadratic() {
        // Quadratic doesn't preserve sign: input * input is always positive
        // This is the actual behavior - the GUI's quadratic curve is non-negative
        let result = CurveGraph::apply_curve(-0.5, &SensitivityCurve::Quadratic);
        assert!((result - 0.25).abs() < 0.001); // (-0.5) * (-0.5) = 0.25
    }

    #[test]
    fn test_apply_curve_negative_input_exponential() {
        // Negative inputs for exponential (with sign preservation)
        let result = CurveGraph::apply_curve(-0.5, &SensitivityCurve::Exponential);
        assert!((result - (-0.25)).abs() < 0.001); // -0.5^2 = -0.25
    }

    #[test]
    fn test_curve_graph_new() {
        let graph = CurveGraph::new(SensitivityCurve::Quadratic, 1.5);
        assert_eq!(graph.multiplier, 1.5);
        // Curve is Copy, so we can compare directly
    }

    #[test]
    fn test_apply_curve_zero() {
        // Zero input should always produce zero output
        assert!((CurveGraph::apply_curve(0.0, &SensitivityCurve::Linear) - 0.0).abs() < f32::EPSILON);
        assert!((CurveGraph::apply_curve(0.0, &SensitivityCurve::Quadratic) - 0.0).abs() < f32::EPSILON);
        assert!((CurveGraph::apply_curve(0.0, &SensitivityCurve::Exponential) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_apply_curve_full_deflection() {
        // Full deflection (1.0) tests
        assert_eq!(CurveGraph::apply_curve(1.0, &SensitivityCurve::Linear), 1.0);
        assert_eq!(CurveGraph::apply_curve(1.0, &SensitivityCurve::Quadratic), 1.0);
        assert_eq!(CurveGraph::apply_curve(1.0, &SensitivityCurve::Exponential), 1.0);
    }
}
