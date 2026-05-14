//! Aethermap GUI Library
//!
//! This library exposes the main GUI components for testing and reuse.

pub mod ipc;
pub mod gui;
pub mod theme;
pub mod views;
pub mod focus_tracker;
pub mod widgets;

// Re-export main types for easier access
pub use gui::{State, Message};
pub use focus_tracker::{FocusTracker, FocusEvent};