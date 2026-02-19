//! Linux overlay using desktop notifications (libnotify / D-Bus).
//!
//! On GNOME and most modern Linux desktops, `notify-rust` sends a
//! persistent notification with two action buttons:
//!   - "Send via HSIP"  → UserChoice::SendPrivately
//!   - "Continue"       → UserChoice::Continue
//!
//! If the notification daemon doesn't support actions (e.g., some minimal
//! WMs), we fall back to a 10-second auto-dismiss as "Continue".

use crate::{
    config::InterceptConfig,
    error::{InterceptError, Result},
    event::MessagingEvent,
    overlay::{InterceptOverlay, OverlayContent, UserChoice},
    PlatformType,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

const ACTION_HSIP: &str = "hsip";
const ACTION_CONTINUE: &str = "continue";
const ACTION_DISABLE: &str = "disable";

/// Linux overlay backed by a desktop notification.
pub struct LinuxOverlay {
    config: InterceptConfig,
    /// Current platform (used for DisableForApp).
    current_platform: Arc<Mutex<PlatformType>>,
}

impl LinuxOverlay {
    /// Create a new Linux overlay.
    pub fn new(config: &InterceptConfig) -> Result<Box<dyn InterceptOverlay>> {
        Ok(Box::new(Self {
            config: config.clone(),
            current_platform: Arc::new(Mutex::new(PlatformType::Unknown)),
        }))
    }

    /// Build the notification body text.
    fn build_body(content: &OverlayContent) -> String {
        content.message.clone()
    }

    /// Show a desktop notification and wait for the user's action choice.
    ///
    /// Uses `notify-rust`'s action API on Linux (D-Bus / libnotify).
    fn show_notification(
        summary: &str,
        body: &str,
        timeout_secs: u32,
    ) -> Result<UserChoice> {
        use notify_rust::{Notification, Timeout};

        let timeout_ms = if timeout_secs == 0 {
            Timeout::Never
        } else {
            Timeout::Milliseconds(timeout_secs * 1_000)
        };

        let handle = Notification::new()
            .summary(summary)
            .body(body)
            .icon("dialog-information")
            .action(ACTION_HSIP, "Send via HSIP")
            .action(ACTION_CONTINUE, "Continue normally")
            .action(ACTION_DISABLE, "Disable for this app")
            .timeout(timeout_ms)
            .show()
            .map_err(|e| InterceptError::Overlay(format!("Failed to show notification: {e}")))?;

        // Block until the user interacts or the notification is dismissed
        let mut chosen = UserChoice::Continue; // default on dismiss/timeout
        handle.wait_for_action(|action| {
            chosen = match action {
                ACTION_HSIP => UserChoice::SendPrivately,
                ACTION_DISABLE => UserChoice::DisableForApp(PlatformType::Unknown), // filled in by caller
                _ => UserChoice::Continue,
            };
        });

        Ok(chosen)
    }
}

#[async_trait::async_trait]
impl InterceptOverlay for LinuxOverlay {
    async fn show(&mut self, event: &MessagingEvent, recipient: Option<&str>) -> Result<UserChoice> {
        let content = OverlayContent::from_event(event, recipient);

        // Remember platform for DisableForApp
        {
            let mut platform = self.current_platform.lock().unwrap();
            *platform = event.platform;
        }

        let summary = content.title.clone();
        let body = Self::build_body(&content);
        let timeout_secs = self.config.overlay.timeout_seconds;
        let current_platform = Arc::clone(&self.current_platform);

        info!("Showing Linux overlay notification for {:?}", event.platform);

        // Run blocking D-Bus call on a dedicated thread
        let choice = tokio::task::spawn_blocking(move || {
            Self::show_notification(&summary, &body, timeout_secs)
        })
        .await
        .map_err(|e| InterceptError::Overlay(format!("Notification task panicked: {e}")))?;

        // Substitute actual platform into DisableForApp
        let choice = match choice? {
            UserChoice::DisableForApp(_) => {
                let platform = *current_platform.lock().unwrap();
                UserChoice::DisableForApp(platform)
            }
            other => other,
        };

        debug!("Linux overlay choice: {:?}", choice);
        Ok(choice)
    }

    async fn hide(&mut self) -> Result<()> {
        // Notifications are dismissed automatically or by user action.
        // No persistent handle to close here.
        Ok(())
    }

    fn is_visible(&self) -> bool {
        // We don't track visibility; the notification daemon handles this.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_creation() {
        let config = InterceptConfig::default();
        let overlay = LinuxOverlay::new(&config);
        assert!(overlay.is_ok());
    }

    #[test]
    fn test_body_build() {
        use crate::{EventType, PlatformType};
        let event = MessagingEvent::new(
            PlatformType::Telegram,
            EventType::WindowChange,
            "telegram-desktop".to_string(),
        );
        let content = OverlayContent::from_event(&event, Some("Alice"));
        let body = LinuxOverlay::build_body(&content);
        assert!(body.contains("Alice"));
    }
}
