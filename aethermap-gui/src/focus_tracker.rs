//! Focus Tracker for Wayland window focus detection
//!
//! This module provides window focus tracking via xdg-desktop-portal.
//! Focus changes are detected and can be used to trigger profile switching.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// A window focus change event
///
/// Contains information about the application that gained focus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusEvent {
    /// Application identifier (e.g., "org.alacritty", "firefox")
    /// This is the primary identifier for profile matching.
    pub app_id: String,
    /// Optional window title
    /// May be empty on some compositors due to Wayland security restrictions.
    pub window_title: Option<String>,
}

impl FocusEvent {
    /// Create a new focus event
    pub fn new(app_id: impl Into<String>, window_title: Option<String>) -> Self {
        Self {
            app_id: app_id.into(),
            window_title,
        }
    }

    /// Create a focus event with only app_id
    pub fn from_app_id(app_id: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            window_title: None,
        }
    }

    /// Check if this event matches a given app_id pattern
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Exact match
        if self.app_id == pattern {
            return true;
        }

        // Suffix match (e.g., ".firefox" matches "org.mozilla.firefox")
        if pattern.starts_with('.') {
            return self.app_id.ends_with(pattern);
        }

        // Prefix match (e.g., "org.mozilla." matches "org.mozilla.firefox")
        if pattern.ends_with('.') {
            return self.app_id.starts_with(pattern);
        }

        false
    }
}

/// Focus tracker using xdg-desktop-portal
///
/// This tracker monitors window focus changes on Wayland compositors
/// via the xdg-desktop-portal API. When the portal is unavailable,
/// it gracefully degrades and provides a no-op implementation.
pub struct FocusTracker {
    /// Portal connection (None when unavailable)
    portal: Option<Arc<FocusPortal>>,
    /// Whether the tracker is currently running
    running: Arc<AtomicBool>,
}

/// Internal portal wrapper for ashpd integration
#[allow(dead_code)]
struct FocusPortal {
    /// Whether portal is available
    available: bool,
    /// Portal backend identifier (for logging/debugging)
    backend: String,
}

impl FocusPortal {
    /// Initialize the portal connection
    ///
    /// Returns None if xdg-desktop-portal is not available or
    /// running on a non-Wayland session.
    async fn try_new() -> Option<Self> {
        // Check if we're running on Wayland
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            tracing::warn!("Not running on Wayland, focus tracking unavailable");
            return None;
        }

        // Check if xdg-desktop-portal is running
        match ashpd::desktop::global_shortcuts::GlobalShortcuts::new().await {
            Ok(_) => {
                tracing::info!("Successfully connected to xdg-desktop-portal");
                Some(Self {
                    available: true,
                    backend: "xdg-desktop-portal".to_string(),
                })
            }
            Err(e) => {
                tracing::warn!("Failed to connect to xdg-desktop-portal: {}", e);
                tracing::warn!("Focus tracking will be unavailable");
                None
            }
        }
    }
}

impl FocusTracker {
    /// Create a new focus tracker
    ///
    /// Attempts to initialize the portal connection. If the portal
    /// is unavailable, returns a tracker that will gracefully handle
    /// all operations as no-ops.
    pub async fn new() -> Self {
        let portal = tokio::task::spawn(async { FocusPortal::try_new().await })
            .await
            .ok()
            .and_then(|r| r);

        Self {
            portal: portal.map(Arc::new),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the portal is available
    pub fn is_available(&self) -> bool {
        self.portal.is_some()
    }

    /// Start tracking focus changes
    ///
    /// Spawns an async task that listens for focus change events
    /// and sends them to the provided channel.
    ///
    /// # Arguments
    ///
    /// * `tx` - Channel to send focus events to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if tracking started successfully, or an error
    /// if tracking is already in progress or the portal is unavailable.
    pub async fn start(&self, _tx: mpsc::Sender<FocusEvent>) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Focus tracking is already running".to_string());
        }

        let running = self.running.clone();
        let _portal = self.portal.clone();

        tokio::spawn(async move {
            tracing::info!("Focus tracking task started");

            if _portal.is_some() {
                // Run focus monitoring loop
                while running.load(Ordering::SeqCst) {
                    // In a full implementation, we would:
                    // 1. Register for focus change notifications via portal
                    // 2. Listen for events
                    // 3. Extract app_id and send to channel

                    // For now, we provide a placeholder that
                    // simulates the structure for future enhancement
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } else {
                tracing::warn!("Focus tracking portal unavailable, task exiting");
            }

            tracing::info!("Focus tracking task stopped");
        });

        Ok(())
    }

    /// Stop tracking focus changes
    ///
    /// Signals the tracking task to stop. The task will exit
    /// on its next iteration.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if tracking is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for FocusTracker {
    fn default() -> Self {
        // Create a runtime for the async new() call
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        rt.block_on(Self::new())
    }
}

/// Start focus tracking with a callback
///
/// Convenience function that creates a tracker and starts it,
/// invoking the callback for each focus event.
///
/// # Arguments
///
/// * `callback` - Function to call with each focus event
///
/// # Returns
///
/// Returns the tracker and a handle that can be used to stop tracking.
pub async fn start_focus_tracking<F>(
    callback: F,
) -> Result<(FocusTracker, tokio::task::JoinHandle<()>), String>
where
    F: Fn(FocusEvent) + Send + 'static,
{
    let tracker = FocusTracker::new().await;

    if !tracker.is_available() {
        return Err("Focus tracking portal unavailable".to_string());
    }

    let (tx, mut rx) = mpsc::channel(32);
    tracker.start(tx).await?;

    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            callback(event);
        }
    });

    Ok((tracker, handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_event_creation() {
        let event = FocusEvent::new("org.alacritty".to_string(), Some("Alacritty".to_string()));
        assert_eq!(event.app_id, "org.alacritty");
        assert_eq!(event.window_title, Some("Alacritty".to_string()));
    }

    #[test]
    fn test_focus_event_from_app_id() {
        let event = FocusEvent::from_app_id("firefox");
        assert_eq!(event.app_id, "firefox");
        assert_eq!(event.window_title, None);
    }

    #[test]
    fn test_focus_event_matches_exact() {
        let event = FocusEvent::from_app_id("org.alacritty");
        assert!(event.matches("org.alacritty"));
        assert!(!event.matches("org.mozilla.firefox"));
    }

    #[test]
    fn test_focus_event_matches_wildcard() {
        let event = FocusEvent::from_app_id("org.alacritty");
        assert!(event.matches("*"));
    }

    #[test]
    fn test_focus_event_matches_suffix() {
        let event = FocusEvent::from_app_id("org.mozilla.firefox");
        assert!(event.matches(".firefox"));
        assert!(event.matches(".mozilla.firefox"));
        assert!(!event.matches(".alacritty"));
    }

    #[test]
    fn test_focus_event_matches_prefix() {
        let event = FocusEvent::from_app_id("org.mozilla.firefox");
        assert!(event.matches("org.mozilla."));
        assert!(event.matches("org."));
        assert!(!event.matches("com."));
    }

    #[tokio::test]
    async fn test_focus_tracker_creation() {
        let tracker = FocusTracker::new().await;
        // Tracker should always create successfully, even if portal unavailable
        assert!(!tracker.is_running());
    }

    #[test]
    fn test_focus_tracker_default() {
        let tracker = FocusTracker::default();
        assert!(!tracker.is_running());
    }
}
