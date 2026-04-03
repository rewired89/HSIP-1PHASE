//! Linux-specific implementation using AT-SPI accessibility and X11/Wayland.
//!
//! Event detection strategy:
//! - X11 sessions: poll active window title via xdotool / x11rb
//! - Wayland sessions: poll frontmost process via /proc
//! - Both: scan running processes for known messaging apps
//!
//! Overlay uses desktop notifications (libnotify / dbus) with action buttons.

pub mod event_monitor;
pub mod overlay;

pub use event_monitor::LinuxEventMonitor;
pub use overlay::LinuxOverlay;

/// Extract recipient hint from a Linux messaging event (best-effort).
///
/// Reads window title metadata set by the event monitor and attempts to
/// parse a recipient name from patterns like "Chat with Alice — Telegram".
pub fn extract_recipient_from_window(
    event: &crate::event::MessagingEvent,
) -> crate::Result<String> {
    if let Some(title) = &event.window_title {
        // Common patterns: "Chat with <name>", "Direct: <name>", "<name> – Telegram"
        for prefix in &["Chat with ", "Direct: ", "DM: "] {
            if let Some(rest) = title.strip_prefix(prefix) {
                let name = rest.split([' ', '—', '–', '-']).next().unwrap_or(rest);
                if !name.is_empty() {
                    return Ok(name.to_string());
                }
            }
        }
        // Em-dash separator: "Alice – Telegram Desktop"
        if let Some(name) = title.split(['—', '–']).next() {
            let name = name.trim();
            if !name.is_empty() && name.len() < 64 {
                return Ok(name.to_string());
            }
        }
    }
    Err(crate::InterceptError::EventMonitor(
        "Cannot extract recipient".to_string(),
    ))
}
