# [Human-Speed Ergonomic Macros] Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement human-like macro execution with randomized jitter, network latency compensation, and full keyboard+mouse recording support, with a GUI in Iced to control these settings.

**Architecture:** Update `razermapper-common` for settings data, `razermapperd` for execution and recording logic, and `razermapper-gui` for user controls.

**Tech Stack:** Rust, Iced (GUI), evdev/uinput (input injection), Bincode/Serde (serialization/IPC).

---

### Task 1: Update Common Data Models & IPC

**Files:**
- Modify: `razermapper/razermapper-common/src/lib.rs`
- Modify: `razermapper/razermapper-common/src/ipc_client.rs`

- [ ] **Step 1: Add MacroSettings and update MacroEntry**

Update `razermapper-common/src/lib.rs` to include `MacroSettings` and add `humanize` and `capture_mouse` fields to `MacroEntry`. Add `SetMacroSettings` and `GetMacroSettings` to `Request` and `Response`.

- [ ] **Step 2: Run `cargo check` to verify types**

Run: `cargo check` in `razermapper/razermapper-common`
Expected: PASS (with warnings for unused fields/variants)

- [ ] **Step 3: Update IPC Client**

Update `razermapper/razermapper-common/src/ipc_client.rs` to add async methods for sending and receiving these new macro settings.

- [ ] **Step 4: Commit**

```bash
git add razermapper/razermapper-common/src/lib.rs razermapper/razermapper-common/src/ipc_client.rs
git commit -m "feat(common): add macro settings and IPC messages"
```

---

### Task 2: Implement Humanization Logic in MacroEngine

**Files:**
- Modify: `razermapper/razermapperd/src/macro_engine.rs`

- [ ] **Step 1: Update MacroEngine struct and execution logic**

Update `razermapper/razermapperd/src/macro_engine.rs` to store `MacroSettings`. Modify the `execute_macro` loop to apply the jitter and latency formula to all `Action::Delay` steps.

- [ ] **Step 2: Implement randomized delay unit test**

Add a test case to `razermapper/razermapperd/src/macro_engine.rs` that verifies delays with jitter fall within the expected range.

- [ ] **Step 3: Run tests**

Run: `cargo test` in `razermapper/razermapperd`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add razermapper/razermapperd/src/macro_engine.rs
git commit -m "feat(daemon): implement humanized delay logic in macro engine"
```

---

### Task 3: Enable Keyboard+Mouse Recording

**Files:**
- Modify: `razermapper/razermapperd/src/macro_engine.rs`
- Modify: `razermapper/razermapperd/src/device.rs`

- [ ] **Step 1: Update `process_input_event` to record mouse buttons**

Update `razermapper/razermapperd/src/macro_engine.rs` to handle mouse button codes during recording if `capture_mouse` is enabled.

- [ ] **Step 2: Ensure `device.rs` passes mouse events to recorder**

Modify `razermapper/razermapperd/src/device.rs` to ensure relative axis events and mouse buttons are forwarded to the `MacroEngine` for recording.

- [ ] **Step 3: Verify with recording test**

Add a test in `macro_engine.rs` that simulates mouse clicks during a recording session and verifies they are stored as `Action::MousePress/Release`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add razermapper/razermapperd/src/macro_engine.rs razermapper/razermapperd/src/device.rs
git commit -m "feat(daemon): support recording mouse events in macros"
```

---

### Task 4: GUI Implementation (Iced)

**Files:**
- Modify: `razermapper/razermapper-gui/src/gui.rs`

- [ ] **Step 1: Add Macro Settings UI**

Update `razermapper/razermapper-gui/src/gui.rs` to add sliders for Latency and Jitter, and a checkbox for "Capture Mouse" in the recording section.

- [ ] **Step 2: Handle IPC state for settings**

Wire up the GUI messages to send the corresponding IPC requests to the daemon when sliders or checkboxes are changed.

- [ ] **Step 3: Verify GUI build**

Run: `cargo check` in `razermapper/razermapper-gui`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add razermapper/razermapper-gui/src/gui.rs
git commit -m "feat(gui): add macro settings sliders and recording options"
```

---

### Task 5: End-to-End Verification

- [ ] **Step 1: Verify mixed macro execution**

Create a test macro in the GUI, record a keyboard+mouse combo, and verify its execution via logs.

- [ ] **Step 2: Final Commit**

```bash
git commit -m "docs: finalize human-speed macro implementation"
```
