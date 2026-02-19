//! macOS-specific implementation using NSWorkspace and User Notifications.
//!
//! Event detection:
//! - Poll `NSWorkspace.shared.frontmostApplication` every 500 ms via AppleScript
//!   subprocess (avoids raw Obj-C bindings in an MVP; proper CoreFoundation
//!   bindings are planned for Phase 2).
//! - Detect known messaging apps by bundle ID or process name.
//!
//! Overlay uses macOS User Notification Center (UNUserNotificationCenter)
//! via `notify-rust`, with "Send via HSIP" and "Continue" action buttons.

pub mod event_monitor;
pub mod overlay;

pub use event_monitor::MacOSEventMonitor;
pub use overlay::MacOSOverlay;

/// Extract recipient hint from a macOS messaging event (best-effort).
///
/// Reads the window title stored in event metadata and parses common
/// patterns like "Chat with Alice" or "Alice - iMessage".
pub fn extract_recipient_from_window(event: &crate::event::MessagingEvent) -> crate::Result<String> {
    if let Some(title) = &event.window_title {
        // Patterns: "Chat with Alice", "Alice – Telegram", "Alice - Messages"
        for prefix in &["Chat with ", "DM with ", "Message to "] {
            if let Some(rest) = title.strip_prefix(prefix) {
                let name = rest.split([' ', '—', '–', '-']).next().unwrap_or(rest);
                if !name.is_empty() && name.len() < 64 {
                    return Ok(name.to_string());
                }
            }
        }
        // Separator-first: "Alice – Messages"
        if let Some(name) = title.split(['—', '–', '-']).next() {
            let name = name.trim();
            if !name.is_empty() && name.len() < 64 {
                return Ok(name.to_string());
            }
        }
    }
    Err(crate::InterceptError::EventMonitor("Cannot extract recipient from macOS window".to_string()))
}
