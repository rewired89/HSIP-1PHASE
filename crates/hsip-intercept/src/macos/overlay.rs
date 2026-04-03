//! macOS overlay using macOS User Notification Center via notify-rust.
//!
//! Presents a macOS banner notification with two action buttons:
//!   - "Send via HSIP"  → UserChoice::SendPrivately
//!   - "Continue"       → UserChoice::Continue
//!
//! The user must grant notification permissions in System Preferences >
//! Notifications for the HSIP process.
//!
//! # Note on action buttons on macOS
//! macOS 10.14+ restricts custom action buttons in notifications to apps
//! distributed through the App Store unless using UNUserNotificationCenter
//! directly. For the MVP, we present a simple notification and rely on the
//! default dismiss/click semantics; a click maps to "Send via HSIP".

use crate::{
    config::InterceptConfig,
    error::{InterceptError, Result},
    event::MessagingEvent,
    overlay::{InterceptOverlay, OverlayContent, UserChoice},
    PlatformType,
};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// macOS notification-based overlay.
pub struct MacOSOverlay {
    config: InterceptConfig,
    current_platform: Arc<Mutex<PlatformType>>,
    visible: Arc<Mutex<bool>>,
}

impl MacOSOverlay {
    /// Create a new macOS overlay.
    pub fn new(config: &InterceptConfig) -> Result<Box<dyn InterceptOverlay>> {
        Ok(Box::new(Self {
            config: config.clone(),
            current_platform: Arc::new(Mutex::new(PlatformType::Unknown)),
            visible: Arc::new(Mutex::new(false)),
        }))
    }

    /// Display a macOS notification and wait for the user's response.
    ///
    /// Returns `UserChoice::SendPrivately` if the notification was clicked,
    /// `UserChoice::Continue` if dismissed or timed out.
    fn show_notification(summary: &str, body: &str, timeout_secs: u32) -> Result<UserChoice> {
        use notify_rust::{Notification, Timeout};

        let timeout = if timeout_secs == 0 {
            Timeout::Never
        } else {
            Timeout::Milliseconds(timeout_secs * 1_000)
        };

        // On macOS, notify-rust uses mac-notification-sys under the hood.
        // Action buttons are limited; a click on the notification body is
        // treated as "Send via HSIP".
        let handle = Notification::new()
            .summary(summary)
            .body(body)
            .action("default", "Send via HSIP")
            .action("close", "Continue")
            .timeout(timeout)
            .show()
            .map_err(|e| InterceptError::Overlay(format!("Notification error: {e}")))?;

        let mut choice = UserChoice::Continue;
        handle.wait_for_action(|action| {
            choice = match action {
                "default" => UserChoice::SendPrivately,
                "__closed" => UserChoice::Continue,
                _ => UserChoice::Continue,
            };
        });

        Ok(choice)
    }
}

#[async_trait::async_trait]
impl InterceptOverlay for MacOSOverlay {
    async fn show(
        &mut self,
        event: &MessagingEvent,
        recipient: Option<&str>,
    ) -> Result<UserChoice> {
        let content = OverlayContent::from_event(event, recipient);

        {
            let mut platform = self.current_platform.lock().unwrap();
            *platform = event.platform;
            let mut vis = self.visible.lock().unwrap();
            *vis = true;
        }

        let summary = content.title.clone();
        let body = content.message.clone();
        let timeout_secs = self.config.overlay.timeout_seconds;
        let visible = Arc::clone(&self.visible);
        let platform = event.platform;

        info!(
            "Showing macOS overlay notification for {:?}",
            event.platform
        );

        let choice = tokio::task::spawn_blocking(move || {
            let result = Self::show_notification(&summary, &body, timeout_secs);
            *visible.lock().unwrap() = false;
            result
        })
        .await
        .map_err(|e| InterceptError::Overlay(format!("Notification task panicked: {e}")))?;

        let choice = match choice? {
            UserChoice::DisableForApp(_) => UserChoice::DisableForApp(platform),
            other => other,
        };

        debug!("macOS overlay choice: {:?}", choice);
        Ok(choice)
    }

    async fn hide(&mut self) -> Result<()> {
        // macOS notifications are managed by the OS; no explicit hide needed.
        *self.visible.lock().unwrap() = false;
        Ok(())
    }

    fn is_visible(&self) -> bool {
        *self.visible.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_creation() {
        let config = InterceptConfig::default();
        let overlay = MacOSOverlay::new(&config);
        assert!(overlay.is_ok());
    }

    #[test]
    fn test_not_visible_initially() {
        let config = InterceptConfig::default();
        let overlay = MacOSOverlay {
            config,
            current_platform: Arc::new(Mutex::new(PlatformType::Unknown)),
            visible: Arc::new(Mutex::new(false)),
        };
        assert!(!overlay.is_visible());
    }
}
