# Phase 16: Calibration GUI - Research

**Researched:** 2026-02-20
**Domain:** Iced GUI Framework (Rust), Canvas-based Visualization, Real-time Data Streaming via IPC
**Confidence:** HIGH

## Summary

Phase 16 creates a comprehensive calibration GUI with live visualization of analog stick input. The foundation already exists: `AnalogCalibrationView` in `gui.rs` (lines 276-4650) provides the UI structure with mode selector, deadzone/sensitivity/range controls, and inversion checkboxes. The missing pieces are: (1) Canvas-based visualization showing stick position and calibration zones, (2) Real-time analog input streaming from daemon to GUI, (3) Sensitivity curve graph visualization, and (4) Live preview of calibration changes.

**Primary recommendation:** Add a Canvas-based `AnalogVisualizer` widget that draws deadzone (circle/square), stick position (dot), and range limits (bounding box). Use `canvas::Program<Message>` trait with `Path::circle()` for circular deadzone and `Path::rectangle()` for square deadzone. For real-time input, add IPC subscription pattern: daemon broadcasts analog events to subscribed GUI clients via a dedicated socket channel or WebSocket-like streaming protocol.

**Key technical insight:** Iced's Canvas widget uses `canvas::Frame` with `Path::circle()` and `Path::rectangle()` methods for drawing shapes (HIGH confidence from [Iced Canvas画布](https://blog.csdn.net/gitblog_01014/article/details/151127786)). The existing `AnalogCalibrationConfig` (lines 341-377 in aethermap-common) already contains all calibration fields. The `get_analog_calibration` and `set_analog_calibration` IPC functions already exist in `ipc_client.rs` (lines 418-492).

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Must Implement:**
- Real-time analog input visualization (stick position widget)
- Deadzone controls (slider, shape selector, visual preview)
- Sensitivity controls (curve selector, multiplier, graph)
- Range controls (min/max sliders, inversion checkboxes)
- Live preview with immediate application
- Calibration persistence to device profile YAML

### Claude's Discretion

- Visual layout of calibration screen
- Curve graph rendering approach
- Color scheme for visualization

### Deferred Ideas (OUT OF SCOPE)

- Advanced Bezier curves
- 3D visualization
- Preset management (beyond YAML config)

</user_constraints>

---

## Standard Stack

### Core Dependencies (Already in Project)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `iced` | 0.12+ | GUI framework, Canvas widget | Project standard; already used for aethermap-gui |
| `aethermap-common` | local | `AnalogCalibrationConfig`, `AnalogMode`, `DeadzoneShape` | Shared types for IPC between GUI and daemon |
| `bincode` | 2.0 | IPC serialization | Already used for request/response protocol |
| `tokio` | 1.0+ | Async runtime for IPC | Required for async IPC communication |

### Supporting Dependencies

| Component | Purpose | When to Use |
|-----------|---------|-------------|
| `iced::widget::canvas` | Custom 2D drawing (stick viz, curve graph) | Visualization components |
| `iced::widget::slider` | Deadzone, sensitivity, range sliders | Numeric input with visual feedback |
| `iced::widget::pick_list` | Mode, shape, curve selectors | Enum selection from predefined options |
| `canvas::Frame` | Drawing context for shapes | All Canvas rendering |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Canvas widget | Static SVG/images | Canvas provides real-time updates; static images don't reflect live input |
| IPC subscription | Polling via timer | Subscription is more efficient; polling wastes CPU and adds latency |
| Single socket for all | Separate stream socket | Single socket is simpler; multiplexing can be added later if needed |

---

## Architecture Patterns

### Recommended Project Structure

```
aethermap/aethermap-gui/src/
├── gui.rs                    # EXTEND: Add AnalogVisualizer Canvas widget
├── ipc.rs                    # EXTEND: Add analog input subscription
└── widgets/                  # NEW: Create widgets module for reusable components
    ├── mod.rs
    ├── analog_visualizer.rs  # Canvas-based stick position widget
    └── curve_graph.rs        # Canvas-based sensitivity curve plot

aethermap/aethermap-common/src/
├── lib.rs                    # EXTEND: Add AnalogInputData for streaming
└── ipc_client.rs             # EXTEND: Add subscribe_analog_input()

aethermap/aethermapd/src/
├── ipc.rs                    # EXTEND: Add subscription handling
└── analog_processor.rs       # USE: Existing analog state for streaming
```

### Pattern 1: Canvas-based Stick Visualization

**What:** Create a `AnalogVisualizer` widget using Iced's Canvas API to draw deadzone, stick position, and range limits.

**When to use:** When displaying real-time analog stick state with visual calibration zones.

**Example:**
```rust
// Source: Based on Iced Canvas画布 documentation
// https://blog.csdn.net/gitblog_01014/article/details/151127786

use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Size};

pub struct AnalogVisualizer {
    /// Current stick position (-1.0 to 1.0)
    stick_x: f32,
    stick_y: f32,
    /// Deadzone radius (0.0 to 1.0)
    deadzone: f32,
    /// Deadzone shape
    deadzone_shape: DeadzoneShape,
    /// Range limits
    range_min: i32,
    range_max: i32,
}

impl<Message> canvas::Program<Message> for AnalogVisualizer {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let center = frame.center();
        let size = bounds.width.min(bounds.height);

        // Draw outer bounds (circle representing full range)
        let outer_radius = size * 0.45;
        let outer_circle = canvas::Path::circle(center, outer_radius);
        frame.fill(&outer_circle, Color::from_rgb(0.2, 0.2, 0.2));
        frame.stroke(&outer_circle, Color::from_rgb(0.5, 0.5, 0.5), 2.0);

        // Draw deadzone (filled circle or square)
        let deadzone_radius = outer_radius * self.deadzone;
        let deadzone_color = Color::from_rgba(0.3, 0.5, 0.3, 0.5);

        if self.deadzone_shape == DeadzoneShape::Circular {
            let deadzone_circle = canvas::Path::circle(center, deadzone_radius);
            frame.fill(&deadzone_circle, deadzone_color);
            frame.stroke(&deadzone_circle, Color::from_rgb(0.4, 0.7, 0.4), 1.0);
        } else {
            // Square deadzone
            let dz_size = deadzone_radius * 2.0;
            let deadzone_rect = canvas::Path::rectangle(
                Point::new(center.x - deadzone_radius, center.y - deadzone_radius),
                Size::new(dz_size, dz_size),
            );
            frame.fill(&deadzone_rect, deadzone_color);
            frame.stroke(&deadzone_rect, Color::from_rgb(0.4, 0.7, 0.4), 1.0);
        }

        // Draw crosshairs (axes)
        let h_line = canvas::Path::line(
            Point::new(center.x - outer_radius, center.y),
            Point::new(center.x + outer_radius, center.y),
        );
        let v_line = canvas::Path::line(
            Point::new(center.x, center.y - outer_radius),
            Point::new(center.x, center.y + outer_radius),
        );
        frame.stroke(&h_line, Color::from_rgba(0.5, 0.5, 0.5, 0.3), 1.0);
        frame.stroke(&v_line, Color::from_rgba(0.5, 0.5, 0.5, 0.3), 1.0);

        // Draw stick position (dot)
        let stick_offset_x = self.stick_x * outer_radius;
        let stick_offset_y = -self.stick_y * outer_radius; // Invert Y for screen coords
        let stick_pos = Point::new(center.x + stick_offset_x, center.y + stick_offset_y);

        let stick_dot = canvas::Path::circle(stick_pos, 6.0);
        frame.fill(&stick_dot, Color::from_rgb(0.9, 0.3, 0.3));
        frame.stroke(&stick_dot, Color::from_rgb(1.0, 1.0, 1.0), 1.0);

        vec![frame.into_geometry()]
    }
}
```

**Source:** [Iced Canvas画布：自定义绘图与图形渲染](https://blog.csdn.net/gitblog_01014/article/details/151127786) - Shows `Path::circle()`, `Frame::fill()`, `Frame::stroke()` for basic shape drawing.

### Pattern 2: Sensitivity Curve Graph

**What:** Canvas-based line graph plotting input (X axis) vs output (Y axis) for the selected sensitivity curve.

**When to use:** Visual preview of how the sensitivity curve transforms analog input.

**Example:**
```rust
pub struct CurveGraph {
    curve: SensitivityCurve,
    multiplier: f32,
}

impl<Message> canvas::Program<Message> for CurveGraph {
    type State = ();

    fn draw(&self, _state: &Self::State, renderer: &Renderer,
            _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor)
            -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // Draw axes
        let margin = 20.0;
        let graph_width = bounds.width - 2.0 * margin;
        let graph_height = bounds.height - 2.0 * margin;
        let origin = Point::new(margin, bounds.height - margin);

        // X and Y axis lines
        let x_axis = canvas::Path::line(
            Point::new(margin, bounds.height - margin),
            Point::new(bounds.width - margin, bounds.height - margin),
        );
        let y_axis = canvas::Path::line(
            Point::new(margin, margin),
            Point::new(margin, bounds.height - margin),
        );
        frame.stroke(&x_axis, Color::WHITE, 2.0);
        frame.stroke(&y_axis, Color::WHITE, 2.0);

        // Plot curve points
        let points: Vec<Point> = (0..=50)
            .map(|i| {
                let input = i as f32 / 50.0; // 0.0 to 1.0
                let output = apply_sensitivity_curve(input, self.curve, self.multiplier);

                Point::new(
                    origin.x + input * graph_width,
                    origin.y - output * graph_height,
                )
            })
            .collect();

        // Draw curve as connected line segments
        for window in points.windows(2) {
            let segment = canvas::Path::line(window[0], window[1]);
            frame.stroke(&segment, Color::from_rgb(0.3, 0.8, 0.3), 2.0);
        }

        vec![frame.into_geometry()]
    }
}

fn apply_sensitivity_curve(input: f32, curve: SensitivityCurve, multiplier: f32) -> f32 {
    let scaled = match curve {
        SensitivityCurve::Linear => input,
        SensitivityCurve::Quadratic => input * input,
        SensitivityCurve::Exponential { exponent } => input.powf(exponent),
    };
    (scaled * multiplier).min(1.0)
}
```

### Pattern 3: IPC Subscription for Real-time Analog Data

**What:** Add subscription mechanism where GUI requests analog input stream and daemon pushes updates.

**When to use:** When real-time stick position updates are needed in the GUI.

**Example (IPC protocol extension):**
```rust
// aethermap-common/src/lib.rs - Add to Request enum
pub enum Request {
    // ... existing requests ...

    /// Subscribe to analog input updates for a device
    SubscribeAnalogInput {
        device_id: String,
    },

    /// Unsubscribe from analog input updates
    UnsubscribeAnalogInput {
        device_id: String,
    },
}

// Add to Response enum
pub enum Response {
    // ... existing responses ...

    /// Analog input update (streamed to subscribers)
    AnalogInputUpdate {
        device_id: String,
        axis_x: f32,  // -1.0 to 1.0
        axis_y: f32,  // -1.0 to 1.0
    },
}
```

**Daemon side (ipc.rs):**
```rust
// Track subscribers: HashMap<device_id, Vec<sender>>
let analog_subscribers: Arc<RwLock<HashMap<String, Vec<UnboundedSender<Response>>>>> = ...;

// In handle_request:
Request::SubscribeAnalogInput { device_id } => {
    // Add client to subscribers list
    // Returns initial calibration data immediately
}

// In analog event processing loop:
async fn broadcast_analog_input(device_id: &str, x: f32, y: f32) {
    let update = Response::AnalogInputUpdate {
        device_id: device_id.to_string(),
        axis_x: x,
        axis_y: y,
    };

    let subscribers = analog_subscribers.read().await;
    if let Some(clients) = subscribers.get(device_id) {
        for sender in clients {
            let _ = sender.send(update.clone());
        }
    }
}
```

**GUI side (gui.rs):**
```rust
// Add to State struct:
pub struct State {
    // ... existing fields ...
    /// Current analog input for visualization
    analog_input_x: f32,
    analog_input_y: f32,
}

// Add Message variants:
pub enum Message {
    // ... existing messages ...
    AnalogInputUpdated(f32, f32), // (x, y)
}

// Add subscription in OpenAnalogCalibration handler:
Message::OpenAnalogCalibration { device_id, layer_id } => {
    // Request subscription
    Command::perform(
        async move { (device_id.clone(), subscribe_analog_input(&device_id).await) },
        |(device_id, result)| match result {
            Ok(_) => Message::AnalogInputSubscribed(device_id),
            Err(e) => Message::ShowNotification(format!("Subscription failed: {}", e), true),
        },
    )
}
```

### Anti-Patterns to Avoid

- **Blocking draw in GUI:** Canvas `draw()` method must be fast; don't do IPC calls or heavy computation inside it
- **Redrawing entire Canvas on every input:** Use `canvas::Cache` to cache static elements (axes, labels) and only redraw dynamic content
- **Sending analog events at full input rate:** Throttle to ~30-60 FPS for GUI; input events can be 100+ Hz
- **Hardcoding Canvas size:** Use `bounds` parameter from `draw()` method to calculate sizes dynamically

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Canvas shape drawing | Custom drawing primitives | `iced::widget::canvas::Path` | Provides `circle()`, `rectangle()`, `line()` methods |
| Sensitivity curve math | Custom curve functions | Reuse `AnalogProcessor::apply_sensitivity()` | Already implemented and tested |
| IPC subscription | Custom WebSocket-like protocol | Extend existing `Request`/`Response` enum | Already using bincode serialization |
| Calibration persistence | Custom file format | Existing `ConfigManager::set_analog_calibration()` | Already persists to YAML |

**Key insight:** The only genuinely new code needed is the Canvas drawing code and the IPC subscription protocol. All calibration logic and persistence already exist.

---

## Common Pitfalls

### Pitfall 1: Canvas Coordinates vs Analog Coordinates

**What goes wrong:** Stick visualization shows incorrect position (e.g., up appears as down).

**Why it happens:** Analog coordinates typically have Y+ = up, but screen coordinates have Y+ = down.

**How to avoid:**
```rust
// Invert Y when converting from analog to screen coords
let screen_y = center.y - (analog_y * radius); // Note the minus sign
```

**Warning signs:** Moving stick up shows dot moving down in visualization.

### Pitfall 2: Deadzone Visualization Doesn't Match Actual Deadzone

**What goes wrong:** Visual deadzone circle is different size than actual deadzone applied to input.

**Why it happens:** Visualization uses different scaling or coordinate system than the actual processing logic.

**How to avoid:**
```rust
// Use the same deadzone calculation as AnalogProcessor
let deadzone_radius = outer_radius * self.calibration.deadzone;
// NOT: Some arbitrary multiplier
```

**Prevention strategy:** Unit test that deadzone radius calculation matches `AnalogCalibration::validate()`.

### Pitfall 3: IPC Subscription Leak

**What goes wrong:** GUI subscribes but never unsubscribes, causing memory leak in daemon.

**Why it happens:** No cleanup when calibration view is closed or GUI exits.

**How to avoid:**
```rust
// In CloseAnalogCalibration handler:
Message::CloseAnalogCalibration => {
    // Send unsubscribe request
    Command::perform(
        async move { unsubscribe_analog_input(&device_id).await },
        |result| { /* handle result */ }
    );
    self.analog_calibration_view = None;
}
```

**Warning signs:** Daemon memory grows continuously when opening/closing calibration view.

### Pitfall 4: Redrawing Too Often

**What goes wrong:** GUI becomes unresponsive when analog input is active.

**Why it happens:** Canvas redraws on every input event (can be 100+ Hz for analog sticks).

**How to avoid:**
```rust
// Throttle redraws using Iced's subscription system
// Only request redraw when data actually changes significantly
// Or use time-based throttling (max 60 FPS)

struct AnalogVisualizer {
    last_update: Instant,
    // ...
}

// Only redraw if >16ms since last update
if self.last_update.elapsed() < Duration::from_millis(16) {
    return vec![]; // Return cached geometry
}
```

### Pitfall 5: Curve Graph Shows Invalid Output

**What goes wrong:** Curve graph shows output > 1.0 (off the chart).

**Why it happens:** Sensitivity multiplier can push output beyond 1.0, but graph is normalized.

**How to avoid:**
```rust
// Clamp output for visualization
let output = apply_sensitivity_curve(input, self.curve, self.multiplier).min(1.0);

// Or show multiplier as separate indicator
```

---

## Code Examples

### Example 1: Integrating AnalogVisualizer into AnalogCalibrationView

```rust
// aethermap/aethermap-gui/src/gui.rs

// Add to AnalogCalibrationView
pub struct AnalogCalibrationView {
    // ... existing fields ...

    /// Visualizer state (current stick position)
    stick_x: f32,
    stick_y: f32,
}

// Update view() method to include Canvas
impl AnalogCalibrationView {
    pub fn view(&self) -> Element<Message> {
        use iced::widget::{horizontal_rule as rule, Row, Canvas};

        // ... existing UI code ...

        // Add visualizer section
        let visualizer_section = Column::new()
            .spacing(10)
            .push(text("Stick Position").size(18))
            .push(
                Container::new(
                    Canvas::new(AnalogVisualizer {
                        stick_x: self.stick_x,
                        stick_y: self.stick_y,
                        deadzone: self.calibration.deadzone,
                        deadzone_shape: self.deadzone_shape_selected,
                        range_min: self.calibration.range_min,
                        range_max: self.calibration.range_max,
                    })
                    .width(Length::Fixed(250.0))
                    .height(Length::Fixed(250.0))
                )
                .width(Length::Fixed(270.0))
                .height(Length::Fixed(270.0))
                .center_x()
                .center_y()
            );

        // Include in main layout
        let content = Column::new()
            .spacing(20)
            .push(title)
            .push(info)
            .push(rule(1))
            .push(visualizer_section)  // NEW
            .push(rule(1))
            .push(mode_section)
            // ... rest of sections ...
    }
}
```

### Example 2: Handling AnalogInputUpdated Message

```rust
// In update() method of gui.rs Application impl

Message::AnalogInputUpdated(x, y) => {
    // Update visualizer state if calibration view is open
    if let Some(view) = &mut self.analog_calibration_view {
        view.stick_x = x;
        view.stick_y = y;
    }
    Command::none() // Triggers redraw
}
```

### Example 3: Canvas with Cache for Performance

```rust
use iced::widget::canvas::{Cache, Geometry};

pub struct AnalogVisualizer {
    stick_x: f32,
    stick_y: f32,
    // ... calibration fields ...

    /// Cache for static elements (deadzone, axes)
    cache: Cache,
}

impl<Message> canvas::Program<Message> for AnalogVisualizer {
    type State = ();

    fn update(&self, _state: &mut Self::State, _event: canvas::Event) {}

    fn draw(&self, _state: &Self::State, renderer: &Renderer,
            _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor)
            -> Vec<Geometry> {
        // Draw static background with cache
        let background = self.cache.draw(renderer, bounds.size(), |frame| {
            // Draw axes, outer bounds, deadzone (these don't change often)
            // ...
        });

        // Draw dynamic stick position (always redraws)
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        // Draw stick dot based on self.stick_x, self.stick_y
        // ...

        vec![background, frame.into_geometry()]
    }
}
```

### Example 4: Complete Sensitivity Curve Function (Reused from Daemon)

```rust
// This should match aethermapd/src/analog_processor.rs implementation

fn apply_sensitivity_curve(input: f32, curve: SensitivityCurve, multiplier: f32) -> f32 {
    // Input should be 0.0 to 1.0 (after deadzone and centering)

    let curved = match curve {
        SensitivityCurve::Linear => input,
        SensitivityCurve::Quadratic => input * input,
        SensitivityCurve::Exponential { exponent } => {
            if input >= 0.0 {
                input.powf(exponent)
            } else {
                -(-input).powf(exponent)
            }
        }
    };

    // Apply multiplier and clamp to valid range
    (curved * multiplier).clamp(-1.0, 1.0)
}
```

---

## State of the Art

### Existing Implementation Status

| Component | Status | Location |
|-----------|--------|----------|
| `AnalogCalibrationView` UI | **Implemented** | gui.rs:276-4650 |
| Mode selector (6 modes) | **Implemented** | Phase 15-07 |
| Deadzone slider/shape | **Implemented** | gui.rs:4534-4554 |
| Sensitivity controls | **Implemented** | gui.rs:4556-4576 |
| Range sliders | **Implemented** | gui.rs:4578-4595 |
| Inversion checkboxes | **Implemented** | gui.rs:4597-4606 |
| IPC Get/Set calibration | **Implemented** | ipc_client.rs:418-492 |
| Canvas stick visualizer | **MISSING** | New for Phase 16 |
| Curve graph widget | **MISSING** | New for Phase 16 |
| Real-time analog streaming | **MISSING** | New for Phase 16 |

### What Changed from Phase 15

Phase 15-07 added the mode selector to `AnalogCalibrationView`. The view now has:
- Mode picker (Disabled, Dpad, Gamepad, WASD, Mouse, Camera)
- Camera sub-mode picker (Scroll, Keys)
- All calibration field controls

Phase 16 adds **visualization only** - no new calibration fields, but **live preview** of how calibration affects input.

---

## Open Questions

### 1. IPC Subscription Implementation Approach

**Question:** Should streaming use the same Unix socket with a multiplexed channel, or a separate dedicated socket?

**What we know:**
- Current IPC is request/response over single socket
- Need bidirectional streaming for analog updates
- Multiple GUI clients might subscribe simultaneously

**Recommendation:** Use the same socket with a multiplexed protocol. Add a "subscription mode" where the connection switches from request/response to streaming mode. This is simpler than managing multiple sockets and works with the existing connection pool.

**Confidence:** MEDIUM - multiplexing adds complexity but keeps the architecture clean. Could validate with a prototype.

### 2: Update Rate for Visualizations

**Question:** How often should the GUI request analog updates?

**What we know:**
- Analog sticks can generate 100+ events per second
- Human visual perception caps at ~60 FPS
- Redrawing Canvas has overhead

**Recommendation:** Throttle to 30 FPS (~33ms between updates). This provides smooth feedback without overwhelming the GUI or CPU.

**Confidence:** HIGH - 30 FPS is standard for UI feedback loops.

### 3: Should Curve Graph Show Actual Output or Just Shape?

**Question:** Should the curve graph plot the mathematical function (0-1 input to 0-1 output) or show the actual scaled output with multiplier?

**What we know:**
- Multiplier can push output beyond 1.0
- Users want to see the "feel" of the curve
- Graph has limited pixels

**Recommendation:** Plot the normalized curve (0-1 range) and show the multiplier as a numeric label separately. This keeps the graph simple and predictable. Add a "clamped" indicator if multiplier would cause output saturation.

**Confidence:** HIGH - simpler visualization is better for calibration UI.

---

## Sources

### Primary (HIGH confidence)

- [gui.rs:276-4650](/home/feanor/Projects/remapper_rs/aethermap/aethermap-gui/src/gui.rs) - Existing `AnalogCalibrationView` implementation with all controls
- [lib.rs:341-377](/home/feanor/Projects/remapper_rs/aethermap/aethermap-common/src/lib.rs) - `AnalogCalibrationConfig` structure with all calibration fields
- [ipc_client.rs:418-492](/home/feanor/Projects/remapper_rs/aethermap/aethermap-common/src/ipc_client.rs) - `get_analog_calibration()` and `set_analog_calibration()` functions
- [ipc.rs:1422-1447](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/ipc.rs) - Daemon handlers for analog calibration IPC
- [analog_calibration.rs:13-223](/home/feanor/Projects/remapper_rs/aethermap/aethermapd/src/analog_calibration.rs) - `AnalogCalibration`, `DeadzoneShape`, `SensitivityCurve` definitions
- [16-CONTEXT.md](/home/feanor/Projects/remapper_rs/.planning/phases/16-calibration-gui/16-CONTEXT.md) - Phase context with requirements

### Secondary (MEDIUM confidence)

- [Iced Canvas画布：自定义绘图与图形渲染](https://blog.csdn.net/gitblog_01014/article/details/151127786) - Iced Canvas drawing API documentation (Path::circle, Frame::fill, Frame::stroke)
- [使用 Rust 和 Iced 构建自定义图形界面](https://m.blog.csdn.net/m0_74337424/article/details/155469671) - Canvas Program trait implementation examples
- [15-07-PLAN.md](/home/feanor/Projects/remapper_rs/.planning/phases/15-wasd-mouse-camera-modes/15-07-PLAN.md) - Phase 15 plan showing mode selector implementation

### Tertiary (LOW confidence - marked for validation)

- Real-time IPC streaming patterns - should prototype to verify performance
- Canvas caching behavior in Iced - may need testing for optimal redraw strategy

---

## Metadata

### Confidence Breakdown

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | Iced, tokio, bincode already in project; verified via Cargo.toml |
| Existing UI code | HIGH | `AnalogCalibrationView` fully implemented; read source directly |
| Canvas drawing API | MEDIUM | Based on external docs; should verify with simple prototype |
| IPC subscription | MEDIUM | Pattern exists in similar projects; needs implementation validation |
| Update rate choice | HIGH | 30 FPS is standard for UI feedback loops |
| Curve graph approach | HIGH | Normalized 0-1 graph with separate multiplier display is clearest |

### Research Date

2026-02-20

### Valid Until

2026-03-22 (30 days - Iced API is stable; existing codebase won't change significantly)

---

## RESEARCH COMPLETE

### Key Findings

1. **UI foundation exists:** `AnalogCalibrationView` already has all controls (mode, deadzone, sensitivity, range, inversion) from Phase 15-07
2. **Canvas API is straightforward:** Iced's `canvas::Program<Message>` trait provides `draw()` method with `Frame` for drawing shapes
3. **Missing pieces are isolated:** Only need to add (a) Canvas widgets, (b) IPC subscription protocol, (c) Real-time update handling
4. **Reuse is extensive:** Sensitivity curve math, calibration persistence, and IPC structure all exist

### Ready for Planning

Research complete. Planner can now create PLAN.md files for:
1. Canvas-based `AnalogVisualizer` widget
2. Canvas-based `CurveGraph` widget
3. IPC subscription protocol for streaming analog input
4. Integration of visualization into `AnalogCalibrationView`
5. Throttling and caching for performance
