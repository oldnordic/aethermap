# Design Spec: Human-Speed Ergonomic Macros (BDO Focused)

## 1. Overview
The goal is to implement "Human-Speed" macros in Aethermap, specifically optimized for high-APM games like Black Desert Online (BDO). These macros will reduce repetitive strain (hand pain) by automating complex **Keyboard + Mouse combos** with "Ergonomic Automation" that mimics natural human speed and accounts for network latency.

## 2. Success Criteria
- [ ] Macros can trigger and execute mixed sequences of keyboard keys and mouse buttons (LMB, RMB, etc.).
- [ ] Macro execution includes a user-configurable "Latency Offset" (safety buffer).
- [ ] Macro execution includes "Humanization" (randomized jitter) to avoid robotic timing.
- [ ] The Macro Recorder captures both keyboard and mouse events with natural timing (Auto-Delay).
- [ ] GUI (Rust Iced) provides sliders for Latency and Jitter, and toggles for mouse capture.

## 3. Architecture & Data Model

### 3.1 Common Data Structures (`aethermap-common`)
- **MacroSettings (New)**:
    - `latency_offset_ms: u32`: Global buffer added to every delay.
    - `jitter_pct: f32`: Randomization range (0.0 to 1.0).
- **MacroEntry (Modified)**:
    - `humanize: bool`: Flag to enable/disable jitter for this macro.
    - `capture_mouse: bool`: Instruction for the recorder to listen for mouse events.

### 3.2 Daemon Logic (`aethermapd`)
- **MacroEngine**:
    - Update `execute_macro` to apply the formula: `ActualSleep = Base_ms + Latency_Offset + (Random_Jitter * Base_ms)`.
    - Update `process_input_event` and `process_analog_event` to handle mouse button clicks and relative moves when `is_recording` is true.
- **Injector**:
    - Ensure `UinputInjector` can interleave keyboard and mouse events without blocking.

### 3.3 GUI Logic (`aethermap-gui`)
- **Settings Tab**:
    - Implement `Slider` widgets for Latency and Jitter Intensity.
- **Macro Recorder**:
    - Add a toggle for "Record Mouse Buttons".
    - Display captured mouse actions in the macro step list using `iced` widgets.

## 4. Error Handling
- Macros interrupted by device unplugging will perform a "Full Release" to prevent stuck keys/mouse buttons.
- Invalid latency/jitter settings will fallback to safe defaults (20ms / 10%).

## 5. Testing Strategy
- **Unit Tests**: Verify the randomization formula produces values within the expected range.
- **Integration Tests**: Verify that a macro with `Action::KeyPress(Shift)` and `Action::MousePress(LMB)` produces the correct interleaved event stream in `uinput`.
- **Manual Verification**: Test skill combos in BDO (e.g., Infinite Potion grinding rotation) to ensure "feel" and reliability.
