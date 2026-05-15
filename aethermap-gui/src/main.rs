//! Aethermap GUI Application
//!
//! Main entry point for the Aethermap GUI application.

use aethermap_gui::State;
use iced::Application;

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    State::run(iced::Settings::default())
}
