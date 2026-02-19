//! macOS event monitor: frontmost app polling via AppleScript subprocess.
//!
//! Uses `osascript` to query the name and window title of the frontmost
//! application every 500 ms. This avoids requiring Obj-C/Swift bindings
//! in the MVP; a Phase 2 update will use CoreFoundation directly for lower
//! latency and no subprocess overhead.
//!
//! # Required macOS permissions
//! The process must have the "Accessibility" and/or "Screen Recording" privacy
//! permission granted in System Preferences > Security & Privacy for the
//! AppleScript subprocess calls to succeed.

use crate::{
    config::InterceptConfig,
    error::{InterceptError, Result},
    event::{EventMonitor, EventType, MessagingEvent, PlatformType},
};
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// AppleScript to get frontmost app name.
const SCRIPT_APP_NAME: &str = r#"
tell application "System Events"
    return name of first process whose frontmost is true
end tell
"#;

/// AppleScript to get the title of the frontmost window.
const SCRIPT_WINDOW_TITLE: &str = r#"
tell application "System Events"
    set frontApp to first process whose frontmost is true
    if exists (windows of frontApp) then
        return name of first window of frontApp
    else
        return ""
    end if
end tell
"#;

/// macOS event monitor.
pub struct MacOSEventMonitor {
    event_tx: mpsc::Sender<MessagingEvent>,
    config: InterceptConfig,
    running: Arc<AtomicBool>,
}

impl MacOSEventMonitor {
    /// Create a new macOS event monitor.
    pub fn new(event_tx: mpsc::Sender<MessagingEvent>, config: &InterceptConfig) -> Result<Box<dyn EventMonitor>> {
        Ok(Box::new(Self {
            event_tx,
            config: config.clone(),
            running: Arc::new(AtomicBool::new(false)),
        }))
    }

    /// Run an AppleScript and return the trimmed stdout, or None on error.
    fn run_applescript(script: &str) -> Option<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .ok()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !result.is_empty() {
                return Some(result);
            }
        } else {
            debug!(
                "osascript error: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        None
    }

    /// Sample the frontmost application name and window title.
    fn sample_frontmost() -> (Option<String>, Option<String>) {
        let app_name = Self::run_applescript(SCRIPT_APP_NAME);
        let window_title = Self::run_applescript(SCRIPT_WINDOW_TITLE);
        (app_name, window_title)
    }

    /// Known macOS messaging app names (as reported by System Events).
    fn is_messaging_app(app_name: &str) -> bool {
        let messaging_apps = [
            "Messages",
            "Telegram",
            "Signal",
            "Slack",
            "Discord",
            "WhatsApp",
            "Messenger",
            "Mimestream",
            "Spark",
            "Mail",
            "Airmail",
            "Microsoft Outlook",
            "Zoom",
            "Microsoft Teams",
            "Skype",
        ];
        let app_lower = app_name.to_lowercase();
        messaging_apps
            .iter()
            .any(|&ma| app_lower.contains(&ma.to_lowercase()))
    }

    /// Check messaging keywords in window title.
    fn has_messaging_title(title: &str) -> (bool, f64) {
        let title_lower = title.to_lowercase();
        let strong_keywords = ["compose", "new message", "direct message", "dm", "send message"];
        let weak_keywords = ["message", "chat", "conversation", "inbox"];

        if strong_keywords.iter().any(|&kw| title_lower.contains(kw)) {
            return (true, 0.90);
        }
        if weak_keywords.iter().any(|&kw| title_lower.contains(kw)) {
            return (true, 0.70);
        }
        (false, 0.50)
    }

    /// Core polling tick: sample, compare, and emit event if changed.
    fn poll_once(
        &self,
        last_app: &mut Option<String>,
        last_title: &mut Option<String>,
    ) {
        let (app, title) = Self::sample_frontmost();

        // Only proceed if something changed
        if app == *last_app && title == *last_title {
            return;
        }

        *last_app = app.clone();
        *last_title = title.clone();

        let app_name = match &app {
            Some(a) => a.clone(),
            None => return,
        };

        if !Self::is_messaging_app(&app_name) {
            return;
        }

        let platform = PlatformType::from_process_name(&app_name);

        if !self.config.is_platform_enabled(platform) {
            return;
        }

        let (keyword_hit, keyword_confidence) = title
            .as_deref()
            .map(Self::has_messaging_title)
            .unwrap_or((false, 0.50));

        // Confidence: app match alone is 0.6; window title keywords raise it
        let confidence = if keyword_hit { keyword_confidence } else { 0.60 };

        let mut event = MessagingEvent::new(platform, EventType::WindowChange, app_name)
            .with_confidence(confidence);

        if let Some(t) = title {
            event = event.with_window_title(t);
        }

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tx.send(event).await {
                error!("macOS monitor: failed to send event: {}", e);
            }
        });
    }
}

#[async_trait::async_trait]
impl EventMonitor for MacOSEventMonitor {
    async fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Verify that osascript is available (it always is on macOS, but be defensive)
        if Command::new("osascript").arg("-e").arg("1").output().is_err() {
            return Err(InterceptError::EventMonitor(
                "osascript not available on this system".to_string(),
            ));
        }

        info!("Starting macOS event monitor (osascript polling)");
        warn!(
            "HSIP requires Accessibility permission in System Preferences > \
             Security & Privacy > Privacy > Accessibility"
        );

        self.running.store(true, Ordering::Relaxed);

        let event_tx = self.event_tx.clone();
        let config = self.config.clone();
        let running = Arc::clone(&self.running);

        tokio::task::spawn_blocking(move || {
            let monitor = MacOSEventMonitor {
                event_tx,
                config,
                running: Arc::clone(&running),
            };

            let mut last_app: Option<String> = None;
            let mut last_title: Option<String> = None;

            debug!("macOS event monitor polling loop started");

            while running.load(Ordering::Relaxed) {
                monitor.poll_once(&mut last_app, &mut last_title);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            debug!("macOS event monitor polling loop stopped");
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Stopping macOS event monitor");
        self.running.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn event_sender(&self) -> &mpsc::Sender<MessagingEvent> {
        &self.event_tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messaging_app_detection() {
        assert!(MacOSEventMonitor::is_messaging_app("Telegram"));
        assert!(MacOSEventMonitor::is_messaging_app("Messages"));
        assert!(MacOSEventMonitor::is_messaging_app("Slack"));
        assert!(!MacOSEventMonitor::is_messaging_app("Safari"));
        assert!(!MacOSEventMonitor::is_messaging_app("Finder"));
    }

    #[test]
    fn test_messaging_title_detection() {
        let (hit, conf) = MacOSEventMonitor::has_messaging_title("Compose new message");
        assert!(hit);
        assert!(conf >= 0.85);

        let (hit2, _) = MacOSEventMonitor::has_messaging_title("Finder");
        assert!(!hit2);
    }
}
