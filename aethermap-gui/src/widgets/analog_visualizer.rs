//! Analog stick position visualizer widget
//!
//! Provides real-time visualization of analog stick position with
//! deadzone overlay and range indicators.

use iced::mouse;
use iced::widget::canvas::{self, event, Cache, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Point, Rectangle};
use std::sync::Arc;

/// Deadzone shape (matches gui.rs enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadzoneShape {
    Circular,
    Square,
}

/// Canvas-based analog stick visualizer
///
/// Displays the current stick position as a dot, with deadzone
/// shown as a shaded region (circle or square). The outer circle
/// represents the full range of motion.
///
/// Uses canvas::Cache to optimize rendering: static elements
/// (outer circle, deadzone, axes) are cached and only redrawn
/// when deadzone or shape changes. Only the stick position
/// dot is redrawn every frame.
///
/// The cache is wrapped in Arc to allow sharing across widget instances
/// since Cache doesn't implement Clone.
pub struct AnalogVisualizer {
    /// Current stick position X (-1.0 to 1.0)
    pub stick_x: f32,
    /// Current stick position Y (-1.0 to 1.0)
    pub stick_y: f32,
    /// Deadzone radius (0.0 to 1.0)
    pub deadzone: f32,
    /// Deadzone shape
    pub deadzone_shape: DeadzoneShape,
    /// Range minimum value (typically -32768)
    pub range_min: i32,
    /// Range maximum value (typically 32767)
    pub range_max: i32,
    /// Cache for static elements (deadzone, axes, outer bounds)
    /// Wrapped in Arc for sharing across widget instances
    pub cache: Arc<Cache>,
}

impl Default for AnalogVisualizer {
    fn default() -> Self {
        Self {
            stick_x: 0.0,
            stick_y: 0.0,
            deadzone: 0.15,
            deadzone_shape: DeadzoneShape::Circular,
            range_min: -32768,
            range_max: 32767,
            cache: Arc::new(Cache::default()),
        }
    }
}

impl<Message> Program<Message> for AnalogVisualizer {
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let size = bounds.width.min(bounds.height);
        let outer_radius = size * 0.45;

        // Draw static background with cache (outer circle, deadzone, axes)
        let background = self.cache.draw(renderer, bounds.size(), |frame| {
            // Draw outer bounds (circle representing full range)
            let outer_circle = Path::circle(center, outer_radius);
            frame.fill(&outer_circle, Color::from_rgb(0.15, 0.15, 0.15));
            frame.stroke(
                &outer_circle,
                Stroke::default()
                    .with_color(Color::from_rgb(0.4, 0.4, 0.4))
                    .with_width(2.0),
            );

            // Draw deadzone (filled circle or square)
            let deadzone_radius = (outer_radius * self.deadzone.clamp(0.0, 1.0)).max(0.0);
            let deadzone_color = Color::from_rgba(0.2, 0.5, 0.2, 0.4);

            if self.deadzone_shape == DeadzoneShape::Circular && deadzone_radius > 0.5 {
                let deadzone_circle = Path::circle(center, deadzone_radius);
                frame.fill(&deadzone_circle, deadzone_color);
                frame.stroke(
                    &deadzone_circle,
                    Stroke::default()
                        .with_color(Color::from_rgb(0.3, 0.7, 0.3))
                        .with_width(1.0),
                );
            } else if deadzone_radius > 0.5 {
                // Square deadzone
                let dz_size = deadzone_radius * 2.0;
                let deadzone_rect = Path::rectangle(
                    Point::new(center.x - deadzone_radius, center.y - deadzone_radius),
                    iced::Size::new(dz_size, dz_size),
                );
                frame.fill(&deadzone_rect, deadzone_color);
                frame.stroke(
                    &deadzone_rect,
                    Stroke::default()
                        .with_color(Color::from_rgb(0.3, 0.7, 0.3))
                        .with_width(1.0),
                );
            }

            // Draw crosshairs (axes)
            let h_line = Path::line(
                Point::new(center.x - outer_radius, center.y),
                Point::new(center.x + outer_radius, center.y),
            );
            let v_line = Path::line(
                Point::new(center.x, center.y - outer_radius),
                Point::new(center.x, center.y + outer_radius),
            );
            frame.stroke(
                &h_line,
                Stroke::default()
                    .with_color(Color::from_rgba(0.5, 0.5, 0.5, 0.3))
                    .with_width(1.0),
            );
            frame.stroke(
                &v_line,
                Stroke::default()
                    .with_color(Color::from_rgba(0.5, 0.5, 0.5, 0.3))
                    .with_width(1.0),
            );

            // Draw center point
            let center_dot = Path::circle(center, 3.0);
            frame.fill(&center_dot, Color::from_rgb(0.6, 0.6, 0.6));
        });

        // Draw dynamic stick position fresh each frame
        let mut frame = Frame::new(renderer, bounds.size());

        // Clamp stick position to valid range
        let stick_x_clamped = self.stick_x.clamp(-1.0, 1.0);
        let stick_y_clamped = self.stick_y.clamp(-1.0, 1.0);

        let stick_offset_x = stick_x_clamped * outer_radius;
        // Invert Y for screen coordinates (analog Y+ = up, screen Y+ = down)
        let stick_offset_y = -stick_y_clamped * outer_radius;
        let stick_pos = Point::new(center.x + stick_offset_x, center.y + stick_offset_y);

        let stick_dot = Path::circle(stick_pos, 6.0);
        frame.fill(&stick_dot, Color::from_rgb(0.9, 0.3, 0.3));
        frame.stroke(
            &stick_dot,
            Stroke::default()
                .with_color(Color::from_rgb(1.0, 1.0, 1.0))
                .with_width(1.0),
        );

        vec![background, frame.into_geometry()]
    }
}

impl AnalogVisualizer {
    /// Clear the cached geometry.
    ///
    /// Call this when deadzone or deadzone shape changes to force
    /// a redraw of the static background elements.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analog_visualizer_default() {
        let viz = AnalogVisualizer::default();
        assert_eq!(viz.stick_x, 0.0);
        assert_eq!(viz.stick_y, 0.0);
        assert_eq!(viz.deadzone, 0.15);
        assert_eq!(viz.deadzone_shape, DeadzoneShape::Circular);
    }

    #[test]
    fn test_analog_visualizer_with_values() {
        let viz = AnalogVisualizer {
            stick_x: 0.5,
            stick_y: -0.3,
            deadzone: 0.2,
            deadzone_shape: DeadzoneShape::Square,
            range_min: -32768,
            range_max: 32767,
            cache: Arc::new(Cache::default()),
        };
        assert_eq!(viz.stick_x, 0.5);
        assert_eq!(viz.stick_y, -0.3);
        assert_eq!(viz.deadzone, 0.2);
        assert_eq!(viz.deadzone_shape, DeadzoneShape::Square);
    }

    #[test]
    fn test_deadzone_shapes() {
        let circular = AnalogVisualizer {
            deadzone_shape: DeadzoneShape::Circular,
            ..Default::default()
        };
        assert_eq!(circular.deadzone_shape, DeadzoneShape::Circular);

        let square = AnalogVisualizer {
            deadzone_shape: DeadzoneShape::Square,
            ..Default::default()
        };
        assert_eq!(square.deadzone_shape, DeadzoneShape::Square);
    }

    #[test]
    fn test_range_values() {
        let viz = AnalogVisualizer {
            range_min: -16384,
            range_max: 16383,
            ..Default::default()
        };
        assert_eq!(viz.range_min, -16384);
        assert_eq!(viz.range_max, 16383);
    }

    #[test]
    fn test_stick_position_clamping_bounds() {
        // Test that stick values can be set to valid bounds
        let viz = AnalogVisualizer {
            stick_x: 1.0,
            stick_y: 1.0,
            ..Default::default()
        };
        assert_eq!(viz.stick_x, 1.0);
        assert_eq!(viz.stick_y, 1.0);

        let viz_negative = AnalogVisualizer {
            stick_x: -1.0,
            stick_y: -1.0,
            ..Default::default()
        };
        assert_eq!(viz_negative.stick_x, -1.0);
        assert_eq!(viz_negative.stick_y, -1.0);
    }

    #[test]
    fn test_clear_cache_exists() {
        let viz = AnalogVisualizer::default();
        // Just verify the method exists and doesn't panic
        viz.clear_cache();
        // Cache is wrapped in Arc, so we can't directly inspect its state
        // But we've verified the method is callable
    }
}
