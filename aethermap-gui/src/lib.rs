//! Aethermap GUI Library
//!
//! This library exposes the main GUI components for testing and reuse.

#![allow(clippy::arc_with_non_send_sync)]

pub mod focus_tracker;
pub mod gui;
pub mod handlers;
pub mod ipc;
pub mod theme;
pub mod views;
pub mod widgets;

// Re-export main types for easier access
pub use focus_tracker::{FocusEvent, FocusTracker};
pub use gui::{Message, State};
