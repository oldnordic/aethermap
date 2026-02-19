# Codebase Concerns

**Analysis Date:** 2026-02-16

## Security Considerations

**Privilege Escalation Root Access:**
- Risk: Daemon MUST run as root for evdev/uinput access
- Files: `razermapperd/src/main.rs:34-37`, `razermapperd/src/device.rs:86-101`
- Current mitigation: Privilege dropping after initialization in `security.rs`
- Recommendations:
  - The `drop_privileges()` function is incomplete - uses simplified prctl approach instead of proper capset/capget
  - Socket permissions set to 0660 with "input" group, but actual enforcement depends on system configuration
  - Token authentication exists (`token-auth` feature) but is disabled by default
  - No rate limiting on IPC requests - vulnerable to resource exhaustion
  - Command execution whitelist in `injector.rs:426-434` but command injection still possible via shell metacharacters

**Insecure Socket Permissions:**
- Risk: README states socket permissions are 0666 (world-readable/writable)
- Files: `razermapperd/src/security.rs:97-116`, `README.md:97`
- Current mitigation: Code attempts to set 0660, but README warns about 0666
- Recommendations: Socket must enforce strict permissions (0660 minimum) and verify ownership

**Unsafe Token Generation:**
- Risk: `SecurityManager::generate_auth_token()` uses predictable entropy sources
- Files: `razermapperd/src/security.rs:150-183`
- Current mitigation: Uses DefaultHasher with timestamp + PID + memory address
- Recommendations: Replace with proper cryptographic RNG (e.g., `getrandom` crate)

**Unsafe File Descriptor Handling:**
- Risk: Raw libc calls without proper error handling
- Files: `razermapperd/src/injector.rs:167-168`, `razermapperd/src/device.rs:93-95`
- Current mitigation: `mem::forget` used on file to keep fd valid
- Recommendations: Audit all ioctl calls and fd handling for proper cleanup on error paths

## Incomplete Features

**Actual Key Remapping:**
- Problem: Core feature not implemented - intercepting key A and outputting key B
- Files: None - feature completely missing
- Blocks: Primary use case for the application
- Priority: HIGH

**LED/RGB Control:**
- Problem: Only protocol stubs exist
- Files: `razermapper-common/src/lib.rs:94-98` (LedSet request), `razermapperd/src/ipc.rs:329-333` (stub handler)
- Blocks: Custom lighting configuration
- Priority: MEDIUM

**Per-Device Macro Restrictions:**
- Problem: `device_id` field exists in `MacroEntry` but not enforced in macro engine
- Files: `razermapperd/src/macro_engine.rs:233-238` (partial check only)
- Blocks: Device-specific macro isolation
- Priority: MEDIUM

**GUI Key Binding Configuration:**
- Problem: No UI for configuring key remaps or editing macro triggers
- Files: `razermapper-gui/src/gui.rs` (no relevant code)
- Blocks: User-friendly configuration
- Priority: HIGH

**Trigger Condition Editing:**
- Problem: Cannot edit macro triggers after recording
- Files: None
- Blocks: Macro customization
- Priority: MEDIUM

**Hot-Reload of Configuration:**
- Problem: `ReloadConfig` request exists but is a no-op
- Files: `razermapperd/src/ipc.rs:324-328`
- Blocks: Runtime configuration changes
- Priority: LOW

**Multi-Device Macro Coordination:**
- Problem: No support for macros spanning multiple devices
- Files: None
- Blocks: Complex macro scenarios
- Priority: LOW

## Tech Debt

**Excessive unwrap() Calls (Potential Panics):**
- Problem: Over 80 `.unwrap()` calls across codebase
- Files: Throughout all modules
  - `razermapperd/src/ipc.rs`: 15+ unwrap() calls on Mutex/RwLock
  - `razermapperd/src/injector.rs`: 10+ unwrap() calls on RwLock
  - `razermapperd/src/macro_engine.rs`: 5+ unwrap() calls
  - `razermapperd/src/main.rs:79`: `state.devices.lock().unwrap()`
  - `razermapper-gui/src/gui.rs:273`: `.iter().next().unwrap()` - assumes non-empty set
- Impact: Potential panics in production if locks are poisoned
- Fix approach: Replace with proper error handling or `expect()` with descriptive messages

**Memory Leak in Macro Execution:**
- Problem: `MacroEngine::execute_macro()` spawns task but never cleans up `executing` map entry
- Files: `razermapperd/src/macro_engine.rs:294-355`, comment at line 352-353 explicitly acknowledges this
- Impact: Memory grows with each macro execution, entry never removed from `executing` HashMap
- Fix approach: Use channel to notify completion or use `JoinHandle` to clean up after task completion

**Lock Poisoning Risk:**
- Problem: Extensive use of `lock().unwrap()` pattern
- Files: `razermapperd/src/ipc.rs`, `razermapperd/src/injector.rs`
- Impact: If any thread panics while holding lock, entire daemon crashes
- Fix approach: Use `lock().map_err()` or `RwLock::try_lock()` with proper error propagation

**Incomplete Privilege Dropping:**
- Problem: `SecurityManager::drop_privileges()` uses simplified prctl approach
- Files: `razermapperd/src/security.rs:43-92`, comments at lines 77-81 acknowledge this
- Impact: Process may retain more capabilities than intended
- Fix approach: Use libcap crate or proper capget/capset system calls

**Hardcoded Paths:**
- Problem: Configuration paths hardcoded to system directories
- Files: `razermapperd/src/config.rs:140-143`
- Impact: Cannot run without root or from different locations
- Fix approach: Use XDG base directory specification or allow path overrides

## Known Bugs

**Macro Recording May Miss Events:**
- Symptoms: Key events during recording may not be captured
- Files: `razermapperd/src/macro_engine.rs:166-189`, `README.md:103`
- Trigger: Recording while system is under heavy load
- Workaround: None documented
- Fix approach: Use dedicated channel for recording events with buffering

**No Trigger Conflict Validation:**
- Symptoms: Multiple macros can have overlapping triggers
- Files: `razermapperd/src/macro_engine.rs:69-84` (add_macro doesn't check conflicts)
- Trigger: Adding macros with same key combo
- Workaround: Manually ensure unique triggers
- Fix approach: Validate against `active_combos` in `add_macro()`

**GUI Doesn't Show Key Codes:**
- Symptoms: Only action counts displayed, not actual key mappings
- Files: `razermapper-gui/src/gui.rs:819-825`
- Trigger: Viewing macro details in GUI
- Workaround: Use daemon logs or inspect profile files
- Fix approach: Add key code to string conversion display

**Profile Format Undocumented:**
- Symptoms: Users don't know how to manually edit profiles
- Files: `README.md:106`, `razermapperd/src/config.rs:353-365`
- Trigger: Attempting manual profile editing
- Workaround: Use GUI only
- Fix approach: Write profile schema documentation

**No Graceful Shutdown on Some Errors:**
- Symptoms: Devices may remain grabbed after crash
- Files: `razermapperd/src/device.rs:377-390`, `README.md:108`
- Trigger: Daemon crash or SIGKILL
- Workaround: Manually release devices via EVIOCGRAB or reboot
- Fix approach: Implement signal handler cleanup for more error conditions

## Performance Bottlenecks

**Synchronous Event Reading in Async Context:**
- Problem: `spawn_blocking` used for evdev reading but channel sends block on runtime handle
- Files: `razermapperd/src/device.rs:147-192`, specifically line 177
- Cause: `rt.block_on(sender_clone.send(...))` blocks worker thread
- Improvement path: Use dedicated async channel or mpsc with try_send

**No Event Rate Limiting:**
- Problem: Spammable macros could flood system with input events
- Files: `razermapperd/src/macro_engine.rs:256-358`
- Cause: No rate limiting on macro execution or event injection
- Improvement path: Add per-macro rate limits and global event throttle

**Inefficient Device Discovery:**
- Problem: Scans all of `/dev/input` and sysfs on every discovery
- Files: `razermapperd/src/device.rs:198-230`
- Cause: No caching or incremental updates
- Improvement path: Use inotify for device hotplug detection

## Fragile Areas

**Macro Engine State Management:**
- Files: `razermapperd/src/macro_engine.rs:21-29`, `razermapperd/src/macro_engine.rs:283-287`
- Why fragile: Three separate RwLocks for different state aspects, complex locking order
- Safe modification: Always lock in consistent order: macros -> executing -> recording
- Test coverage: Unit tests exist but don't cover concurrent modification scenarios

**IPC Handler State Access:**
- Files: `razermapperd/src/ipc.rs:146-256`
- Why fragile: Complex state cloning pattern, potential for deadlocks
- Safe modification: Minimize lock hold time, clone data before processing
- Test coverage: Basic request handling tested, no concurrency stress tests

**Uinput Initialization:**
- Files: `razermapperd/src/injector.rs:146-254`
- Why fragile: Many ioctls must succeed, partial failure leaves inconsistent state
- Safe modification: Use RAII guard pattern, ensure cleanup on all error paths
- Test coverage: Only basic creation test, no failure injection tests

**Device Grab/Ungrab:**
- Files: `razermapperd/src/device.rs:74-137`
- Why fragile: EVIOCGRAB can fail silently, event reader task orphaned on error
- Safe modification: Track grab state separately, kill event reader on ungrab
- Test coverage: Basic grab/ungrab tested, no crash recovery tests

## Scaling Limits

**Concurrent Macro Execution:**
- Current capacity: `max_concurrent_macros` defaults to 10
- Limit: Each macro spawns a tokio task, no resource pooling
- Files: `razermapperd/src/macro_engine.rs:26-27`, `razermapperd/src/macro_engine.rs:219-224`
- Scaling path: Implement semaphore for concurrency limiting, add macro priority queue

**IPC Connection Handling:**
- Current capacity: No connection limit, unbounded task spawning
- Limit: Each connection spawns a new task (line 95 of `ipc.rs`)
- Files: `razermapperd/src/ipc.rs:82-120`
- Scaling path: Add connection limit, implement connection pooling

**Event Queue Size:**
- Current capacity: 1000 events in mpsc channel
- Limit: Hardcoded in `DeviceManager::new()`
- Files: `razermapperd/src/device.rs:32`, config defaults at 1000
- Scaling path: Make configurable, implement backpressure

## Dependencies at Risk

**evdev Crate:**
- Risk: Direct raw fd manipulation and libc ioctl usage
- Impact: Breaks on non-Linux platforms or kernel API changes
- Files: `razermapperd/src/device.rs`
- Migration plan: Abstract ioctl operations, version-check kernel compatibility

**iced GUI Framework:**
- Risk: 0.12 version used, API may change significantly
- Impact: GUI may break with minor version updates
- Files: `razermapper-gui/src/gui.rs:1-8`
- Migration plan: Pin to exact version, monitor upstream for breaking changes

**bincode Serialization:**
- Risk: No schema versioning, protocol changes break compatibility
- Impact: Old clients cannot talk to new daemon
- Files: `razermapper-common/src/lib.rs:234-243`, all IPC code
- Migration plan: Add protocol version field to Request enum, implement version negotiation

## Missing Critical Features

**Key Remapping Engine:**
- Problem: Cannot remap one key to another (primary use case)
- Blocks: Basic input remapping functionality
- Status: Not implemented, no code exists

**Configuration Validation:**
- Problem: No validation of loaded configs/macros
- Blocks: Safe error messages for malformed profiles
- Status: Partial, serde provides some validation

**Audit Logging:**
- Problem: No logging of security-sensitive operations
- Blocks: Forensic analysis of potential misuse
- Status: Not implemented

**User/Group Permission Management:**
- Problem: Assumes "input" group exists, no fallback
- Blocks: Running on systems without input group
- Status: Partial implementation in `security.rs`

## Test Coverage Gaps

**Untested: Concurrent macro execution**
- What's not tested: Multiple macros executing simultaneously
- Files: `razermapperd/src/macro_engine.rs:256-358`
- Risk: Race conditions in injector access
- Priority: HIGH

**Untested: Lock poisoning recovery**
- What's not tested: Behavior when locks are poisoned
- Files: All modules using `.unwrap()` on locks
- Risk: Unexpected panics in production
- Priority: MEDIUM

**Untested: Device hotplug handling**
- What's not tested: Adding/removing devices while daemon is running
- Files: `razermapperd/src/device.rs`
- Risk: Daemon state desync on device changes
- Priority: MEDIUM

**Untested: Malformed IPC messages**
- What's not tested: Invalid/oversized/corrupted IPC payloads
- Files: `razermapperd/src/ipc.rs:159-176`
- Risk: DoS via malicious client
- Priority: HIGH

**Untested: uinput failure modes**
- What's not tested: /dev/uinput unavailable, ioctl failures
- Files: `razermapperd/src/injector.rs:146-254`
- Risk: Silent failures, inconsistent state
- Priority: MEDIUM

**Untested: Profile downgrade**
- What's not tested: Loading profiles from newer version
- Files: `razermapperd/src/config.rs:369-391`
- Risk: Data loss or corruption
- Priority: LOW

---

*Concerns audit: 2026-02-16*
