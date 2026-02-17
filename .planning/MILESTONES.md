# Milestones: Razermapper

Release history of Razermapper, shipped with verified capabilities.

## v1.0 MVP

**Status:** ✅ Shipped 2026-02-17
**Phases:** 4 (Core Remapping, Profiles/IPC, Hotplug/Hot-Reload, Testing)
**Plans:** 19 total (6 + 6 + 4 + 3)

### Key Accomplishments

1. **Core Remapping Engine** — KeyParser with 30+ keys, RemapEngine with HashMap lookup tables, YAML configuration loading
2. **Per-Device Profile Switching** — RemapTable type alias for O(1) atomic profile swaps, extended YAML config, GUI components
3. **Hotplug and Hot-Reload** — udev-based device monitoring, SIGHUP config reload with validate-then-swap pattern
4. **Comprehensive Testing** — 50+ unit/integration tests, virtual device end-to-end tests with evdev::uinput
5. **Memory Safety** — Fixed macro engine memory leak, removed unwrap() calls from hot paths, Drop trait for device cleanup

### What Shipped

Users can now remap any input key to any other key through YAML configuration (`/etc/razermapperd/remaps.yaml`). Remaps persist across reboots, key repeat works correctly, and the daemon has no memory leaks. Per-device profiles can be switched at runtime via GUI, with hotplug support and configuration hot-reload.

### Files

- **Archive:** `.planning/milestones/v1.0-ROADMAP.md`
- **Requirements:** `.planning/milestones/v1.0-REQUIREMENTS.md`
- **Tag:** `v1.0`

---

*Last updated: 2026-02-17*
