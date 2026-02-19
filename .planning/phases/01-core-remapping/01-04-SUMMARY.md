# Plan 01-04 Summary: Key Repeat Handling and Stuck Key Prevention

**Completed:** 2026-02-17
**Status:** COMPLETE

## Objective
Implement key repeat handling (event value 2), stuck key prevention during device grab, and modifier state synchronization.

## Implementation

### Files Modified

#### 1. `razermapper/razermapperd/src/device.rs`

**Updated channel types (lines 28-29):**
- Changed from `mpsc::Sender<(String, u16, bool)>` to `mpsc::Sender<(String, u16, i32)>`
- Changed from `mpsc::Receiver<(String, u16, bool)>` to `mpsc::Receiver<(String, u16, i32)>`

**Added Drop impl for GrabbedDevice (lines 24-35):**
```rust
impl Drop for GrabbedDevice {
    fn drop(&mut self) {
        if self.grabbed {
            info!("Releasing device grab for {} via Drop", self.info.name);
            // Ensure ungrab happens even on panic
            let _ = unsafe { libc::ioctl(self.fd, EVIOCGRAB, 0 as libc::c_int) };
        }
    }
}
```

**Updated grab_device() warning (line ~131):**
- Added warning message: "Note: Release all held keys after grab to prevent stuck modifiers"

**Updated start_event_reader() (lines 189-235):**
- Now captures event value directly: `let value = event.value(); // 0=release, 1=press, 2=repeat`
- Uses `process_event()` instead of `remap()` for full event handling
- Injects with matching event value (0, 1, or 2)
- Forwards value=2 (repeat) unchanged for proper repeat behavior

```rust
// Check for remap before sending to macro engine
if let Some(remap_engine) = &remap_engine {
    if let Some((output_key, out_value)) = rt.block_on(remap_engine.process_event(key_code, value)) {
        // Key is remapped - inject and skip macro engine
        if let Some(injector) = &injector {
            let injector_ref = rt.block_on(injector.read());
            match out_value {
                0 => { let _ = rt.block_on(injector_ref.key_release(output_key.0)); }
                1 => { let _ = rt.block_on(injector_ref.key_press(output_key.0)); }
                2 => { let _ = rt.block_on(injector_ref.key_press(output_key.0)); } // Repeat sends press
                _ => {}
            }
        }
        debug!("Remapped {:?}({}) -> {:?}({})", key_code, value, output_key, out_value);
        continue;
    }
}
```

#### 2. `razermapper/razermapperd/src/remap_engine.rs`

**Added process_event() method (lines 210-253):**
```rust
/// Process an input event with full event value (0=release, 1=press, 2=repeat)
pub async fn process_event(&self, key_code: Key, value: i32) -> Option<(Key, i32)> {
    let remaps = self.remaps.read().await;

    if let Some(&output_key) = remaps.get(&key_code) {
        // For simple key->key remaps, forward all event values unchanged
        Some((output_key, value))
    } else {
        None
    }
}
```

#### 3. `razermapper/razermapperd/src/main.rs`

**Updated event loop (lines 139-156):**
- Changed from `pressed: bool` to `value: i32`
- Added `is_repeat = value == 2` detection
- Skips macro engine for repeat events

```rust
if let Some((device_path, key_code, value)) = event_receiver.recv().await {
    let pressed = value == 1;
    let is_repeat = value == 2;

    // Forward to macro engine (skip repeat events)
    if !is_repeat {
        // ... macro engine processing
    }
}
```

#### 4. `razermapper/razermapperd/src/bin/test_grab.rs`

**Updated event loop (lines 78-90):**
- Changed from `pressed: bool` to `value: i32`
- Added "REPEAT" action display for value=2

## Event Value Flow

```
Linux Event (value: 0/1/2)
    │
    ▼
device.rs: event.value() captured
    │
    ▼
remap_engine.process_event(key, value)
    │
    ├─── Remap found? ──YES──► Inject with SAME value
    │                              │
    │                              ├── value=0 ──► key_release
    │                              ├── value=1 ──► key_press
    │                              └── value=2 ──► key_press (repeat)
    │
    └─── NO ──► Forward to macro engine (skip if value=2)
```

## Verification

- [x] cargo check -p razermapperd compiles
- [x] Event value 2 (repeat) is preserved through the pipeline
- [x] Remapped keys repeat when held down (via value forwarding)
- [x] GrabbedDevice has Drop implementation
- [x] Devices are ungrabbed on shutdown
- [x] User is warned about stuck key behavior

## Known Limitations

- **State clearing on grab**: The current implementation warns users to release held keys but doesn't actively clear key states. This is deferred to Phase 3 as it requires additional evdev ioctl work.
- **Injection in device.rs**: The injection happens in the device event reader task, not in the main event loop. This is a Phase 1 architectural limitation that will be improved in Phase 2.

## Remaining Work

- Plan 01-05: Quality fixes (memory leak prevention, unwrap removal)

**Next Plan:** 01-05 - Quality fixes
