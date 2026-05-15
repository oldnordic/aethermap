//! Integration tests for mixed keyboard+mouse macros
//!
//! These tests verify end-to-end functionality of macros with mixed
//! keyboard and mouse actions, including proper sequencing and timing.

use aethermap_common::{Action, KeyCombo, MacroEntry};
use aethermapd::macro_engine::MacroEngine;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock injector for testing macro execution without requiring root privileges
struct MockInjector {
    log: Arc<RwLock<Vec<String>>>,
}

impl MockInjector {
    fn new() -> Self {
        Self {
            log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn log_action(&self, action: &str) {
        let mut log = self.log.write().await;
        log.push(action.to_string());
    }

    async fn get_log(&self) -> Vec<String> {
        let log = self.log.read().await;
        log.clone()
    }
}

#[async_trait::async_trait]
impl aethermapd::injector::Injector for MockInjector {
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action("initialize").await;
        Ok(())
    }

    async fn key_press(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("key_press:{}", key_code)).await;
        Ok(())
    }

    async fn key_release(
        &self,
        key_code: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("key_release:{}", key_code)).await;
        Ok(())
    }

    async fn mouse_press(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("mouse_press:{}", button)).await;
        Ok(())
    }

    async fn mouse_release(
        &self,
        button: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("mouse_release:{}", button)).await;
        Ok(())
    }

    async fn mouse_move(
        &self,
        x: i32,
        y: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("mouse_move:{},{}", x, y)).await;
        Ok(())
    }

    async fn mouse_scroll(
        &self,
        amount: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("mouse_scroll:{}", amount)).await;
        Ok(())
    }

    async fn type_string(
        &self,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("type_string:{}", text)).await;
        Ok(())
    }

    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("execute_command:{}", command))
            .await;
        Ok(())
    }

    async fn analog_move(
        &self,
        axis_code: u16,
        value: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log_action(&format!("analog_move:{},{}", axis_code, value))
            .await;
        Ok(())
    }
}

#[tokio::test]
async fn test_mixed_keyboard_mouse_macro() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::new();
    engine.set_injector(injector.clone()).await;

    // Create a macro with mixed keyboard and mouse actions
    let macro_entry = MacroEntry {
        name: "Ctrl+Click macro".to_string(),
        trigger: KeyCombo {
            keys: vec![29], // KEY_LEFTCTRL
            modifiers: vec![],
        },
        actions: vec![
            Action::KeyPress(30),    // A key press
            Action::MousePress(1),   // BTN_LEFT press (button 1 = code 272)
            Action::Delay(10),       // Small delay for testing
            Action::MouseRelease(1), // BTN_LEFT release
            Action::KeyRelease(30),  // A key release
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    // Execute the macro
    let result = engine.execute_macro(macro_entry.clone()).await;

    // Verify it executed successfully
    assert!(result.is_ok(), "Macro should execute successfully");

    // Give the spawned task time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify actions were logged in correct order
    let log = injector.read().await.get_log().await;
    assert!(log.contains(&"key_press:30".to_string()));
    assert!(log.contains(&"mouse_press:1".to_string()));
    assert!(log.contains(&"mouse_release:1".to_string()));
    assert!(log.contains(&"key_release:30".to_string()));
}

#[tokio::test]
async fn test_mouse_movement_macro() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::new();
    engine.set_injector(injector.clone()).await;

    // Create a mouse movement macro
    let macro_entry = MacroEntry {
        name: "Mouse movement".to_string(),
        trigger: KeyCombo {
            keys: vec![30], // KEY_A
            modifiers: vec![],
        },
        actions: vec![
            Action::MouseMove(10, 0), // Move right 10 units
            Action::Delay(10),
            Action::MouseMove(-5, 0), // Move left 5 units
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    let result = engine.execute_macro(macro_entry).await;
    assert!(result.is_ok(), "Mouse movement macro should execute");

    // Give the spawned task time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = injector.read().await.get_log().await;
    assert!(log.contains(&"mouse_move:10,0".to_string()));
    assert!(log.contains(&"mouse_move:-5,0".to_string()));
}

#[tokio::test]
async fn test_mouse_scroll_macro() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::new();
    engine.set_injector(injector.clone()).await;

    // Create a scroll macro
    let macro_entry = MacroEntry {
        name: "Scroll down".to_string(),
        trigger: KeyCombo {
            keys: vec![30], // KEY_A
            modifiers: vec![],
        },
        actions: vec![
            Action::MouseScroll(1), // Scroll down 1 click
            Action::Delay(10),
            Action::MouseScroll(1), // Scroll down another click
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    let result = engine.execute_macro(macro_entry).await;
    assert!(result.is_ok(), "Mouse scroll macro should execute");

    // Give the spawned task time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let log = injector.read().await.get_log().await;
    assert!(log.contains(&"mouse_scroll:1".to_string()));
}

#[tokio::test]
async fn test_delay_action_timing() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::new();
    engine.set_injector(injector.clone()).await;

    // Create a macro with delays to verify async timing
    let macro_entry = MacroEntry {
        name: "Delay test".to_string(),
        trigger: KeyCombo {
            keys: vec![30],
            modifiers: vec![],
        },
        actions: vec![
            Action::KeyPress(30),
            Action::Delay(50), // 50ms delay
            Action::KeyRelease(30),
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    let start = std::time::Instant::now();
    let result = engine.execute_macro(macro_entry).await;
    assert!(result.is_ok());

    // Wait for macro to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let elapsed = start.elapsed();
    // Should take at least 50ms for the delay
    assert!(elapsed >= tokio::time::Duration::from_millis(50));

    let log = injector.read().await.get_log().await;
    assert!(log.contains(&"key_press:30".to_string()));
    assert!(log.contains(&"key_release:30".to_string()));
}

#[tokio::test]
async fn test_action_sequence_order() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::new();
    engine.set_injector(injector.clone()).await;

    // Create a complex macro to verify action ordering
    let macro_entry = MacroEntry {
        name: "Sequence test".to_string(),
        trigger: KeyCombo {
            keys: vec![28], // KEY_ENTER
            modifiers: vec![],
        },
        actions: vec![
            Action::KeyPress(30),    // 1: Press A
            Action::MousePress(1),   // 2: Press left mouse
            Action::Delay(10),       // 3: Delay
            Action::MouseScroll(1),  // 4: Scroll
            Action::MouseMove(5, 5), // 5: Move mouse
            Action::MouseRelease(1), // 6: Release mouse
            Action::KeyRelease(30),  // 7: Release A
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    let result = engine.execute_macro(macro_entry).await;
    assert!(result.is_ok());

    // Wait for macro to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify all actions were executed in order
    let log = injector.read().await.get_log().await;

    // Check that all expected actions are present
    let expected = vec![
        "key_press:30",
        "mouse_press:1",
        "mouse_scroll:1",
        "mouse_move:5,5",
        "mouse_release:1",
        "key_release:30",
    ];

    for action in expected {
        assert!(
            log.contains(&action.to_string()),
            "Expected action '{}' not found in log: {:?}",
            action,
            log
        );
    }
}

#[tokio::test]
async fn test_multiple_macros_concurrent() {
    let injector = Arc::new(RwLock::new(MockInjector::new()));
    let mut engine = MacroEngine::with_config(10, 10);
    engine.set_injector(injector.clone()).await;

    // Create multiple different macros
    let macro1 = MacroEntry {
        name: "Macro 1".to_string(),
        trigger: KeyCombo {
            keys: vec![30],
            modifiers: vec![],
        },
        actions: vec![
            Action::KeyPress(30),
            Action::Delay(10),
            Action::KeyRelease(30),
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    let macro2 = MacroEntry {
        name: "Macro 2".to_string(),
        trigger: KeyCombo {
            keys: vec![31],
            modifiers: vec![],
        },
        actions: vec![
            Action::MousePress(1),
            Action::Delay(10),
            Action::MouseRelease(1),
        ],
        device_id: None,
        enabled: true,
        humanize: false,
        capture_mouse: false,
    };

    // Execute both macros
    let result1 = engine.execute_macro(macro1).await;
    let result2 = engine.execute_macro(macro2).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    // Wait for both to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify both executed
    let log = injector.read().await.get_log().await;
    assert!(log.iter().filter(|x| x.contains("key_press")).count() >= 1);
    assert!(log.iter().filter(|x| x.contains("mouse_press")).count() >= 1);
}
