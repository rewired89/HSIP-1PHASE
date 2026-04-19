//! Windows-specific implementation using UI Automation API.

pub mod event_monitor;
pub mod messenger;
pub mod overlay;
pub mod utils;

pub use event_monitor::WindowsEventMonitor;
pub use messenger::{extract_recipient_from_window, open_messenger_window};
pub use overlay::WindowsOverlay;

// Re-export common Windows types
pub use windows::Win32::UI::Accessibility::{IUIAutomation, IUIAutomationElement, UIA_PATTERN_ID};
