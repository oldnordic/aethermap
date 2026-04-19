# [Aethermap Project Transformation] Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename project to Aethermap and implement an adaptive COSMIC-inspired GUI theme.

**Architecture:** Systematic refactor of all metadata, paths, and source code, followed by Iced theme customization with DBus integration.

**Tech Stack:** Rust, Iced, Ashpd (DBus), Cargo.

---

### Task 1: Core Renaming (Common & Daemon)

**Files:**
- Modify: `aethermap/Cargo.toml`
- Modify: `aethermap/aethermap-common/Cargo.toml`
- Modify: `aethermap/aethermapd/Cargo.toml`
- Rename: `aethermap/aethermap-common/` -> `aethermap/aethermap-common/`
- Rename: `aethermap/aethermapd/` -> `aethermap/aethermapd/`

- [ ] **Step 1: Update Workspace Metadata**
Update `aethermap/Cargo.toml` workspace members to use new paths.

- [ ] **Step 2: Rename Common Crate**
Update `aethermap-common/Cargo.toml` package name.

- [ ] **Step 3: Rename Daemon Crate**
Update `aethermapd/Cargo.toml` package name and dependencies.

- [ ] **Step 4: Run `cargo check`**
Ensure workspace still compiles after metadata changes.

---

### Task 2: GUI Renaming & System Integration

**Files:**
- Modify: `aethermap/aethermap-gui/Cargo.toml`
- Modify: `aethermap/aethermapd/systemd/aethermapd.service`
- Rename: `aethermap/aethermap-gui/` -> `aethermap/aethermap-gui/`
- Modify: `PKGBUILD`, `build-package.sh`

- [ ] **Step 1: Rename GUI Crate**
Update `aethermap-gui/Cargo.toml` package name.

- [ ] **Step 2: Update Systemd & Socket Paths**
Change `/run/aethermap/` to `/run/aethermap/` in daemon and GUI source code and service files.

- [ ] **Step 3: Update Arch Linux Packaging**
Update `PKGBUILD` and build scripts with new binary names.

---

### Task 3: Global Source Code Replace

- [ ] **Step 1: Case-Sensitive Replace**
Run `sed` or equivalent over the entire workspace:
- `aethermap` -> `aethermap`
- `Aethermap` -> `Aethermap`
- `AETHERMAP` -> `AETHERMAP`

- [ ] **Step 2: Fix Imports**
Ensure all `use aethermap_common` statements are now `use aethermap_common`.

- [ ] **Step 3: Verify Build**
Run `cargo build` to ensure the "Big Sweep" didn't break logic.

---

### Task 4: Adaptive COSMIC Theme Implementation

**Files:**
- Modify: `aethermap/aethermap-gui/src/gui.rs`
- Create: `aethermap/aethermap-gui/src/theme.rs`

- [ ] **Step 1: Create AetherTheme**
Define the Dark/Light palettes and styling rules.

- [ ] **Step 2: Implement DBus Listener**
Use `ashpd` to subscribe to system theme changes and send a `ThemeChanged` message to the GUI.

- [ ] **Step 3: Refresh Visuals**
Apply 10px rounded corners and Cosmic Blue accents to all containers.

---

### Task 5: Final Verification & Cleanup

- [ ] **Step 1: Test Migration**
Verify that a symlink or move from `~/.config/aethermap` to `~/.config/aethermap` works on startup.

- [ ] **Step 2: Final Test Run**
Run all workspace tests.

- [ ] **Step 3: Final Commit**
```bash
git commit -m "feat: transform project to Aethermap with Adaptive COSMIC Theme"
```
