# Razermapper Rust Project - Agent Guidelines

## Build & Test Commands
- `cargo build` - Build all workspace members
- `cargo build --release` - Build optimized release
- `cargo test` - Run all tests across workspace
- `cargo test -p razermapperd` - Run daemon tests
- `cargo test -p razermapper-gui` - Run GUI tests
- `cargo test -p razermapper-common` - Run common library tests
- `cargo test --test e2e` - Run end-to-end integration tests
- `cargo clippy` - Lint with Clippy
- `cargo fmt` - Format code with rustfmt

## Code Style Guidelines
- Use workspace dependencies defined in root Cargo.toml
- Follow Rust naming conventions: snake_case for functions/vars, PascalCase for types
- Use `tracing` for logging with appropriate levels (info, warn, error, debug)
- Implement proper error handling with `Result<T, E>` and `thiserror`
- Use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared state in async contexts
- All async functions must be `Send + Sync` compatible
- Use `#[tokio::main]` for async main functions
- Document public APIs with `///` doc comments
- Prefer `format!()` over string concatenation
- Use `cfg!` macro for platform-specific code

## Project Structure
- Workspace with members: razermapper-common, razermapperd, razermapper-gui, tests
- Common types and IPC in razermapper-common
- Privileged daemon in razermapperd (requires root)
- GUI client in razermapper-gui using iced framework
- End-to-end tests in tests/ directory