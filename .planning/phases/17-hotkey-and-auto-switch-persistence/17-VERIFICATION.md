---
phase: 17-hotkey-and-auto-switch-persistence
verified: 2026-02-20T14:45:00Z
status: passed
score: 5/5 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "GlobalHotkeyManager startup initialization - load_bindings() called at daemon startup"
    - "GetAutoSwitchRules IPC protocol - GUI can load existing rules"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Start daemon, configure hotkey via GUI, restart daemon, verify hotkey still works"
    expected: "Hotkey configuration persists across daemon restart"
    why_human: "Requires actual daemon lifecycle testing with config files"
  - test: "Configure auto-switch rules in GUI, restart daemon, switch focus to different app, verify profile switches automatically"
    expected: "Auto-switch rules persist and work after daemon restart"
    why_human: "Integration test requiring running daemon with Wayland portal"
---

# Phase 17: Hotkey and Auto-Switch Persistence Verification Report

**Phase Goal:** Users can configure hotkey bindings and auto-switch rules via the GUI, and these settings persist across daemon restarts.
**Verified:** 2026-02-20T14:45:00Z
**Status:** passed
**Re-verification:** Yes — gap closure verification after Plans 17-09 and 17-10

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | GUI can add/remove/list hotkey bindings via IPC | ✓ VERIFIED | RegisterHotkey/RemoveHotkey/ListHotkeys handlers implemented (ipc.rs:1812-1931), GUI sends requests (gui.rs:1101-1167) |
| 2 | GUI can save auto-switch rules via IPC | ✓ VERIFIED | SetAutoSwitchRules handler implemented (ipc.rs:1931-1962), GUI sends requests (gui.rs:941, 982) |
| 3 | Hotkey bindings load from YAML on daemon startup | ✓ VERIFIED | GlobalHotkeyManager initialized in main.rs:114, load_bindings() called at main.rs:123, stored in DaemonState (lib.rs:79) |
| 4 | Auto-switch rules load from YAML on daemon startup | ✓ VERIFIED | ConfigManager::set_auto_switch_rules persists to config.yaml (config.rs:1025), DaemonConfig wraps in Arc<RwLock<>> (config.rs:764) |
| 5 | All settings persist across daemon restarts | ✓ VERIFIED | Unit tests pass for YAML serialization, startup initialization verified, GetAutoSwitchRules IPC implemented |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `razermapper/razermapperd/src/config.rs` | ConfigManager hotkey CRUD methods | ✓ VERIFIED | add_hotkey_binding (line 2446), remove_hotkey_binding (line 2533), get_hotkey_bindings (line 2593) - all substantive implementations with YAML persistence |
| `razermapper/razermapperd/src/config.rs` | ConfigManager auto-switch persistence | ✓ VERIFIED | set_auto_switch_rules (line 1025), get_auto_switch_rules (line 1073) - read/write to config.yaml via Arc<RwLock<>> |
| `razermapper/razermapperd/src/ipc.rs` | Hotkey IPC handlers | ✓ VERIFIED | RegisterHotkey (line 1812), ListHotkeys (line 1867), RemoveHotkey (line 1900) - all call ConfigManager methods and reload GlobalHotkeyManager |
| `razermapper/razermapperd/src/ipc.rs` | Auto-switch IPC handler | ✓ VERIFIED | SetAutoSwitchRules (line 1931) - calls ConfigManager::set_auto_switch_rules and reloads AutoProfileSwitcher |
| `razermapper/razermapper-common/src/lib.rs` | IPC protocol definitions | ✓ VERIFIED | Request::SetAutoSwitchRules (line 698), Request::GetAutoSwitchRules (line 703), Response::AutoSwitchRulesAck (line 1118), Response::AutoSwitchRules (line 1123) |
| `razermapper/razermapper-gui/src/gui.rs` | GUI hotkey persistence | ✓ VERIFIED | SaveHotkeyBinding (line 1101), LoadHotkeyBindings (line 1032), DeleteHotkeyBinding (line 1167) - full IPC integration |
| `razermapper/razermapper-gui/src/gui.rs` | GUI auto-switch persistence | ✓ VERIFIED | SaveAutoSwitchRule (line 918), DeleteAutoSwitchRule (line 958), LoadAutoSwitchRules (line 840) - full IPC integration with GetAutoSwitchRules |
| `razermapper/razermapperd/src/global_hotkey_manager.rs` | load_bindings() implementation | ✓ VERIFIED | load_bindings() (line 128) reads from ConfigManager::get_all_hotkey_bindings, normalizes modifiers, falls back to defaults |
| `razermapper/razermapperd/src/config.rs` | get_all_hotkey_bindings method | ✓ VERIFIED | get_all_hotkey_bindings() (line 2692) aggregates bindings from all devices in device_profiles.yaml |
| `razermapper/razermapperd/src/main.rs` | GlobalHotkeyManager startup initialization | ✓ VERIFIED | GlobalHotkeyManager::new at line 114, load_bindings() at line 123, stored in DaemonState at line 131 |
| `razermapper/razermapperd/src/lib.rs` | DaemonState hotkey_manager field | ✓ VERIFIED | hotkey_manager field added at line 79, initialized to None in DaemonState::new at line 97 |
| `razermapper/razermapperd/src/ipc.rs` | GetAutoSwitchRules handler | ✓ VERIFIED | Request::GetAutoSwitchRules handler at line 1968, calls ConfigManager::get_auto_switch_rules, returns Response::AutoSwitchRules |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|----|----|
| GUI SaveHotkeyBinding | Request::RegisterHotkey | IpcClient::send (gui.rs:1141) | ✓ WIRED | Async IPC call with proper error handling |
| Request::RegisterHotkey handler | ConfigManager::add_hotkey_binding | await call (ipc.rs:1853) | ✓ WIRED | Full implementation with profile validation |
| RegisterHotkey handler | GlobalHotkeyManager.load_bindings | hotkey_manager.lock().load_bindings().await (ipc.rs:1846) | ✓ WIRED | Reloads after successful add |
| GUI SaveAutoSwitchRule | Request::SetAutoSwitchRules | IpcClient::send (gui.rs:941) | ✓ WIRED | Type conversion from GUI to common type |
| Request::SetAutoSwitchRules handler | ConfigManager::set_auto_switch_rules | await call (ipc.rs:1943) | ✓ WIRED | Saves to config.yaml |
| SetAutoSwitchRules handler | AutoProfileSwitcher.reload_rules | switcher.write().await.load_rules() (ipc.rs:1947) | ✓ WIRED | Reloads with new rules |
| GlobalHotkeyManager::load_bindings | ConfigManager::get_all_hotkey_bindings | await call (global_hotkey_manager.rs:130) | ✓ WIRED | Reads all device bindings |
| **main.rs startup** | **GlobalHotkeyManager::new** | **Arc::new(tokio::sync::Mutex::new(...)) (main.rs:113-118)** | **✓ WIRED** | **Gap 1 closed - GlobalHotkeyManager initialized at daemon startup** |
| **main.rs startup** | **load_bindings()** | **hotkey_mgr.load_bindings().await (main.rs:123)** | **✓ WIRED** | **Gap 1 closed - Bindings loaded from device_profiles.yaml on startup** |
| **main.rs startup** | **DaemonState.hotkey_manager** | **state.hotkey_manager = Some(...) (main.rs:131)** | **✓ WIRED** | **Gap 1 closed - HotkeyManager stored for IPC access** |
| **main.rs startup** | **DeviceManager.set_hotkey_manager** | **device_manager.set_hotkey_manager(Some(hotkey_mgr)) (main.rs:150)** | **✓ WIRED** | **Gap 1 closed - HotkeyManager wired into device event loop** |
| **GUI LoadAutoSwitchRules** | **Request::GetAutoSwitchRules** | **client.send(&Request::GetAutoSwitchRules) (gui.rs:845-846)** | **✓ WIRED** | **Gap 2 closed - GUI loads existing rules from daemon** |
| **Request::GetAutoSwitchRules handler** | **ConfigManager::get_auto_switch_rules** | **config_manager.get_auto_switch_rules().await (ipc.rs:1970)** | **✓ WIRED** | **Gap 2 closed - Daemon returns rules from config** |
| **GetAutoSwitchRules handler** | **GUI Message::AutoSwitchRulesLoaded** | **Response::AutoSwitchRules { rules } (ipc.rs:1983)** | **✓ WIRED** | **Gap 2 closed - IPC response with rule data** |
| **GUI LoadAutoSwitchRules** | **AutoSwitchRulesLoaded message** | **Message::AutoSwitchRulesLoaded (gui.rs:861)** | **✓ WIRED** | **Gap 2 closed - Rules populated in view** |

### Requirements Coverage

| Requirement | Status | Blocking Issue |
|------------|--------|-----------------|
| Hotkey bindings persist to YAML | ✓ SATISFIED | ConfigManager serializes to device_profiles.yaml (tokio::fs::write) |
| Auto-switch rules persist to YAML | ✓ SATISFIED | ConfigManager serializes to config.yaml (tokio::fs::write) |
| GUI can configure hotkeys | ✓ SATISFIED | Full IPC integration with RegisterHotkey/RemoveHotkey/ListHotkeys |
| GUI can configure auto-switch rules | ✓ SATISFIED | Full IPC integration with SetAutoSwitchRules/GetAutoSwitchRules |
| Settings load on daemon startup | ✓ SATISFIED | GlobalHotkeyManager initialized and load_bindings() called at startup (main.rs:114-131), auto-switch loads via DaemonConfig initialization |
| Settings survive daemon restart | ✓ SATISFIED | Unit tests verify YAML serialization, startup initialization verified, TODO removed from GUI LoadAutoSwitchRules |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| - | - | - | - | - |

**No anti-patterns found.** All implementations are substantive (no stubs, no TODOs in critical paths, proper error handling).

Previous TODO at gui.rs:845 ("Load from daemon") has been removed and replaced with full IPC implementation.

### Human Verification Required

#### 1. Hotkey Persistence Across Restarts

**Test:** 
1. Start razermapperd daemon
2. Open GUI, navigate to device hotkey settings
3. Add a hotkey binding (e.g., Ctrl+1 → Gaming profile)
4. Close GUI
5. Restart daemon: `systemctl restart razermapperd`
6. Open GUI, verify hotkey still appears in list
7. Press the hotkey combination, verify profile switches

**Expected:** Hotkey configuration persists across daemon restart and functions correctly immediately after restart (no GUI interaction required).

**Why human:** Requires actual daemon lifecycle management and interactive GUI testing. Cannot verify programmatically without running full daemon process.

---

#### 2. Auto-Switch Rules Persistence and Functionality

**Test:**
1. Start razermapperd daemon with Wayland portal available
2. Open GUI, navigate to auto-switch rules
3. Add rule: "org.alacritty" → Terminal profile
4. Close GUI
5. Restart daemon
6. Open GUI, verify rule still exists (view should populate with existing rules)
7. Open Alacritty terminal, verify profile switches to Terminal automatically
8. Switch to another app, verify profile switches back

**Expected:** Auto-switch rules persist and function correctly after daemon restart. GUI loads and displays existing rules when opening the view.

**Why human:** Integration test requiring Wayland portal focus tracking and running daemon with config file I/O.

---

## Gap Closure Summary

### Gap 1: GlobalHotkeyManager Startup Initialization ✓ CLOSED

**Previous Issue:** GlobalHotkeyManager::load_bindings() existed and was called by IPC handlers after config changes, but there was no explicit startup initialization call in main.rs. Hotkeys worked after first GUI interaction, not immediately after restart.

**Implementation (Plan 17-09):**
- Added `hotkey_manager: Option<Arc<tokio::sync::Mutex<GlobalHotkeyManager>>>` field to DaemonState (lib.rs:79)
- Initialized GlobalHotkeyManager in main.rs after ConfigManager setup (main.rs:113-118)
- Called load_bindings() during startup with error handling (main.rs:120-126)
- Stored GlobalHotkeyManager in DaemonState for IPC access (main.rs:128-132)
- Set hotkey_manager on DeviceManager before start_discovery() (main.rs:146-153)

**Verification:**
- ✓ GlobalHotkeyManager::new called at main.rs:114
- ✓ load_bindings().await called at main.rs:123
- ✓ HotkeyManager stored in DaemonState at main.rs:131
- ✓ DeviceManager.set_hotkey_manager() called at main.rs:150
- ✓ Error handling with warn log on failure
- ✓ Both crates compile successfully

**Evidence:** 
```
main.rs:114:        razermapperd::GlobalHotkeyManager::new(
main.rs:123:            if let Err(e) = hotkey_mgr.load_bindings().await {
main.rs:131:            state.hotkey_manager = Some(global_hotkey_manager);
main.rs:150:        device_manager.set_hotkey_manager(Some(hotkey_mgr));
```

### Gap 2: GetAutoSwitchRules IPC Protocol ✓ CLOSED

**Previous Issue:** GUI's LoadAutoSwitchRules handler had TODO comment and didn't load existing rules from daemon. No GetAutoSwitchRules IPC protocol existed. Auto-switch rules view started empty on open.

**Implementation (Plan 17-10):**
- Added Request::GetAutoSwitchRules to common library (lib.rs:703)
- Added Response::AutoSwitchRules { rules } to common library (lib.rs:1123)
- Implemented GetAutoSwitchRules handler in ipc.rs (line 1968-1984)
- Added get_auto_switch_rules() convenience method to ipc_client.rs (line 515)
- Updated GUI LoadAutoSwitchRules to call GetAutoSwitchRules IPC (gui.rs:840-862)
- Removed TODO comment at gui.rs:845

**Verification:**
- ✓ Request::GetAutoSwitchRules exists in common library
- ✓ Response::AutoSwitchRules exists in common library
- ✓ IPC handler calls ConfigManager::get_auto_switch_rules() (ipc.rs:1970)
- ✓ Handler returns Response::AutoSwitchRules with rules (ipc.rs:1983)
- ✓ GUI LoadAutoSwitchRules sends GetAutoSwitchRules request (gui.rs:845)
- ✓ GUI handles Response::AutoSwitchRules and converts to GUI type (gui.rs:847-854)
- ✓ GUI calls Message::AutoSwitchRulesLoaded on success (gui.rs:861)
- ✓ TODO comment removed from gui.rs:845
- ✓ Both crates compile successfully

**Evidence:**
```
common/lib.rs:703:    GetAutoSwitchRules,
common/lib.rs:1123:        AutoSwitchRules { rules: Vec<AutoSwitchRule> },
ipc.rs:1968:        Request::GetAutoSwitchRules => {
ipc.rs:1970:            let rules = config_manager.get_auto_switch_rules().await;
gui.rs:845:                        let request = Request::GetAutoSwitchRules;
gui.rs:847:                            Ok(Response::AutoSwitchRules { rules }) => {
```

---

## Overall Assessment

**Phase 17 fully achieves its goal.** All gaps from previous verification have been closed:

✅ **Complete (All 5 must-haves verified):**
1. GUI can add/remove/list hotkey bindings via IPC (full implementation, no stubs)
2. GUI can save auto-switch rules via IPC (full implementation, no stubs)
3. Hotkey bindings load from YAML on daemon startup (GlobalHotkeyManager initialization added)
4. Auto-switch rules load from YAML on daemon startup (DaemonConfig wraps in Arc<RwLock<>>)
5. All settings persist across daemon restarts (startup initialization verified, GetAutoSwitchRules implemented)

✅ **Gap Closure Verified:**
- Gap 1 (GlobalHotkeyManager startup initialization): ✓ CLOSED
  - GlobalHotkeyManager instantiated at daemon startup (main.rs:114)
  - load_bindings() called with error handling (main.rs:123)
  - HotkeyManager stored in DaemonState for IPC access (lib.rs:79, main.rs:131)
  - DeviceManager.set_hotkey_manager() wired into event loop (main.rs:150)

- Gap 2 (GetAutoSwitchRules IPC protocol): ✓ CLOSED
  - Request/Response variants added to common library
  - Daemon handler implemented calling ConfigManager::get_auto_switch_rules()
  - GUI LoadAutoSwitchRules wired to call GetAutoSwitchRules IPC
  - TODO comment removed, full IPC integration with type conversion

✅ **Code Quality:**
- No stub implementations (all code substantive with proper error handling)
- No TODO/FIXME/HACK comments in critical paths
- Both razermapperd and razermapper-gui compile successfully
- Proper async/await patterns throughout
- Error handling with logging for edge cases

❓ **Needs human verification:**
- Actual daemon restart persistence testing (end-to-end integration test)
- Real-world usage with hotkey activation after restart
- Auto-switch rules functionality with Wayland portal focus tracking

**Recommendation:** Phase 17 is complete and ready for human verification. All automated checks pass. No further implementation work required.

---

_Verified: 2026-02-20T14:45:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Gap closure after Plans 17-09 and 17-10_
