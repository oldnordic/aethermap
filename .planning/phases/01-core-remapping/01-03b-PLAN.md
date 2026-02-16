---
phase: 01-core-remapping
plan: 03b
type: execute
wave: 3
depends_on: [01-03]
files_modified:
  - razermapper/razermapperd/src/device.rs
autonomous: true

must_haves:
  truths:
    - "RemapEngine processes events from grabbed devices before macro engine"
    - "Remapped key events are injected via UinputInjector"
    - "Unremapped keys pass through to macro engine (not suppressed)"
    - "Event loop order: device -> remap -> (inject if remapped) -> macro engine"
    - "GrabbedDevice has optional remap_engine and injector fields"
  artifacts:
    - path: "razermapper/razermapperd/src/device.rs"
      provides: "Event channel with remap capability"
      exports: ["GrabbedDevice", "grab_device"]
      covered_by: "Task 1: Add remap_engine and injector fields to GrabbedDevice"
  key_links:
    - from: "razermapper/razermapperd/src/device.rs"
      to: "razermapper/razermapperd/src/remap_engine.rs"
      via: "Option<Arc<RemapEngine>> parameter in event processing"
      pattern: "remap_engine.remap"
    - from: "razermapper/razermapperd/src/device.rs"
      to: "razermapper/razermapperd/src/injector.rs"
      via: "UinputInjector for remapped keys"
      pattern: "injector.key_press|key_release"
---

<objective>
Integrate RemapEngine into device event loop for key code translation.

Purpose: The RemapEngine must be called on every input event from grabbed devices. Remapped keys are injected and skipped from macro engine processing. This connects the translation engine to the device input stream.

Output: Modified device.rs that wires RemapEngine into the event processing pipeline.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/01-core-remapping/01-CONTEXT.md
@.planning/phases/01-core-remapping/01-RESEARCH.md
@.planning/phases/01-core-remapping/01-01-PLAN.md
@.planning/phases/01-core-remapping/01-02-PLAN.md
@.planning/phases/01-core-remapping/01-03-PLAN.md

# Existing event loop
@razermapper/razermapperd/src/device.rs
</context>

<tasks>

<task type="auto">
  <name>Add remap_engine and injector fields to GrabbedDevice</name>
  <files>razermapper/razermapperd/src/device.rs</files>
  <action>
    Update razermapper/razermapperd/src/device.rs:

    1. Add remap_engine and injector parameters to GrabbedDevice struct:
       ```rust
       pub struct GrabbedDevice {
           pub info: DeviceInfo,
           pub evdev: EvdevDevice,
           pub fd: RawFd,
           pub grabbed: bool,
           pub remap_engine: Option<Arc<crate::remap_engine::RemapEngine>>,
           pub injector: Option<Arc<tokio::sync::RwLock<dyn crate::injector::Injector + Send + Sync>>>,
       }
       ```

    2. Update grab_device to accept optional remap_engine and injector:
       - Add parameters: remap_engine: Option<Arc<RemapEngine>>, injector: Option<Arc<RwLock<dyn Injector>>>
       - Store in GrabbedDevice

    DO NOT break existing functionality when these are None

    REFERENCE: Current GrabbedDevice definition (line ~67)
  </action>
  <verify>
    grep -n "struct GrabbedDevice\|remap_engine\|injector" razermapper/razermapperd/src/device.rs
    Verify fields are added to struct
  </verify>
  <done>
    GrabbedDevice has:
    - remap_engine: Option<Arc<RemapEngine>> field
    - injector: Option<Arc<RwLock<dyn Injector>>> field
  </done>
</task>

<task type="auto">
  <name>Integrate remap into device event processing loop</name>
  <files>razermapper/razermapperd/src/device.rs</files>
  <action>
    Update razermapper/razermapperd/src/device.rs start_event_reader method:

    1. After receiving event (line ~168), check for remap before sending to macro engine:
       ```rust
       // In the blocking task, after getting key_code and pressed
       if let Some(remap_engine) = &grabbed.remap_engine {
           if let Some(output_key) = remap_engine.remap(key_code).await {
               // Key is remapped - inject and skip macro engine
               if let Some(injector) = &grabbed.injector {
                   let injector_ref = injector.read().await;
                   if pressed {
                       let _ = injector_ref.key_press(output_key).await;
                   } else {
                       let _ = injector_ref.key_release(output_key).await;
                   }
               }
               debug!("Remapped {:?} -> {:?}", key_code, output_key);
               continue; // Skip sending to macro engine
           }
       }
       // Send to macro engine for non-remapped keys
       ```

    CRITICAL: Unremapped keys MUST continue to macro engine unchanged

    REFERENCE: Event reading pattern (lines 162-189)
  </action>
  <verify>
    grep -n "remap_engine\|injector" razermapper/razermapperd/src/device.rs
    Verify remap logic is integrated
  </verify>
  <done>
    device.rs:
    - GrabbedDevice has remap_engine and injector fields
    - Events are checked for remaps before macro engine
    - Remapped keys are injected, not forwarded to macro engine
    - Unremapped keys continue to macro engine
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:

1. cargo check -p razermapperd compiles
2. Device grab/release works with remap engine integration
3. Remapped keys are injected
4. Unremapped keys reach macro engine
</verification>

<success_criteria>
1. GrabbedDevice has remap_engine and injector optional fields
2. Device event loop checks for remaps before macro engine
3. Remapped keys are injected via UinputInjector
4. Unremapped keys pass through to macro engine unchanged
5. No unwrap() calls added to hot paths
</success_criteria>

<output>
After completion, create `.planning/phases/01-core-remapping/01-03b-SUMMARY.md` with:
- Event flow diagram
- Integration points verified
- Remaining work for full IPC-driven device grabbing
</output>
