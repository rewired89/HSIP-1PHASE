//! Linux event monitor: active window polling via X11 (xdotool) or /proc.
//!
//! Detection pipeline:
//! 1. Every 500 ms, sample the currently focused window.
//! 2. On X11: call `xdotool getactivewindow getwindowname` (subprocess).
//!    On Wayland or if xdotool absent: scan /proc for known messaging processes.
//! 3. Compare against previous sample; emit a MessagingEvent on change.
//! 4. Classify platform and compute confidence from window title + process name.

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

/// Linux event monitor.
pub struct LinuxEventMonitor {
    event_tx: mpsc::Sender<MessagingEvent>,
    config: InterceptConfig,
    running: Arc<AtomicBool>,
}

impl LinuxEventMonitor {
    /// Create a new Linux event monitor.
    pub fn new(
        event_tx: mpsc::Sender<MessagingEvent>,
        config: &InterceptConfig,
    ) -> Result<Box<dyn EventMonitor>> {
        Ok(Box::new(Self {
            event_tx,
            config: config.clone(),
            running: Arc::new(AtomicBool::new(false)),
        }))
    }

    /// Detect current display server type.
    fn is_wayland() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|t| t.eq_ignore_ascii_case("wayland"))
                .unwrap_or(false)
    }

    /// Get the active window title via xdotool (X11 only).
    ///
    /// Returns `None` if xdotool is unavailable or the query fails.
    fn x11_active_window_title() -> Option<String> {
        let output = Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()
            .ok()?;

        if output.status.success() {
            let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
        None
    }

    /// Get the process name of the window owner via xdotool + /proc (X11 only).
    fn x11_active_window_process() -> Option<String> {
        // xdotool getactivewindow getwindowpid
        let pid_output = Command::new("xdotool")
            .args(["getactivewindow", "getwindowpid"])
            .output()
            .ok()?;

        if !pid_output.status.success() {
            return None;
        }

        let pid_str = String::from_utf8_lossy(&pid_output.stdout)
            .trim()
            .to_string();
        let pid: u32 = pid_str.parse().ok()?;
        Self::read_proc_comm(pid)
    }

    /// Read /proc/<pid>/comm to get process name.
    fn read_proc_comm(pid: u32) -> Option<String> {
        std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Scan /proc for running messaging application processes.
    ///
    /// Returns list of (pid, process_name) pairs for known messaging apps.
    fn scan_proc_for_messaging_apps() -> Vec<(u32, String)> {
        let messaging_processes = [
            "telegram-deskto", // truncated to 15 chars (Linux COMM limit)
            "Telegram Desktop",
            "signal-desktop",
            "slack",
            "discord",
            "whatsapp-nativef",
            "thunderbird",
            "evolution",
            "geary",
            "fractal",
            "element-desktop",
            "nheko",
            "ferdi",
            "rambox",
            "ferdium",
        ];

        let mut found = Vec::new();

        let proc_dir = match std::fs::read_dir("/proc") {
            Ok(d) => d,
            Err(_) => return found,
        };

        for entry in proc_dir.flatten() {
            let fname = entry.file_name();
            let fname_str = fname.to_string_lossy();

            // Only PID directories
            let pid: u32 = match fname_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if let Some(comm) = Self::read_proc_comm(pid) {
                let comm_lower = comm.to_lowercase();
                for &mp in &messaging_processes {
                    if comm_lower.contains(&mp.to_lowercase()) {
                        found.push((pid, comm));
                        break;
                    }
                }
            }
        }

        found
    }

    /// Sample the current window state and emit events on change.
    fn poll_once(&self, last_title: &mut Option<String>, last_process: &mut Option<String>) {
        let (title, process) = if Self::is_wayland() {
            // On Wayland, we can't easily query the focused window without
            // a compositor-specific protocol. Fall back to process scanning.
            let apps = Self::scan_proc_for_messaging_apps();
            if let Some((_, proc_name)) = apps.first() {
                (None, Some(proc_name.clone()))
            } else {
                (None, None)
            }
        } else {
            // X11: try xdotool
            let title = Self::x11_active_window_title();
            let process = Self::x11_active_window_process();
            (title, process)
        };

        // Check if anything changed
        let changed = title != *last_title || process != *last_process;
        if !changed {
            return;
        }

        *last_title = title.clone();
        *last_process = process.clone();

        let process_name = process.unwrap_or_default();
        let platform = PlatformType::from_process_name(&process_name);

        if !self.config.is_platform_enabled(platform) {
            return;
        }

        // Compute confidence from title keywords
        let title_lower = title.as_deref().unwrap_or("").to_lowercase();
        let messaging_keywords = [
            "compose",
            "message",
            "chat",
            "direct",
            "dm",
            "messenger",
            "inbox",
            "conversation",
            "write",
            "new message",
            "send",
            "reply",
        ];

        let keyword_hit = messaging_keywords
            .iter()
            .any(|&kw| title_lower.contains(kw));
        let confidence = if keyword_hit { 0.85 } else { 0.55 };

        let mut event = MessagingEvent::new(platform, EventType::WindowChange, process_name)
            .with_confidence(confidence);

        if let Some(t) = title {
            event = event.with_window_title(t);
        }

        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tx.send(event).await {
                error!("Linux monitor: failed to send event: {}", e);
            }
        });
    }
}

#[async_trait::async_trait]
impl EventMonitor for LinuxEventMonitor {
    async fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.running.store(true, Ordering::Relaxed);

        let display_server = if Self::is_wayland() { "Wayland" } else { "X11" };
        info!("Starting Linux event monitor ({})", display_server);

        if !Self::is_wayland() {
            // Check if xdotool is available
            if Command::new("xdotool").arg("--version").output().is_err() {
                warn!(
                    "xdotool not found; falling back to /proc scanning. \
                       Install xdotool for more accurate window detection."
                );
            }
        }

        let event_tx = self.event_tx.clone();
        let config = self.config.clone();
        let running = Arc::clone(&self.running);

        tokio::task::spawn_blocking(move || {
            let monitor = LinuxEventMonitor {
                event_tx,
                config,
                running: Arc::clone(&running),
            };

            let mut last_title: Option<String> = None;
            let mut last_process: Option<String> = None;

            debug!("Linux event monitor polling loop started");

            while running.load(Ordering::Relaxed) {
                monitor.poll_once(&mut last_title, &mut last_process);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            debug!("Linux event monitor polling loop stopped");
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Stopping Linux event monitor");
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
    fn test_proc_comm_read() {
        // PID 1 (init/systemd) should always have a readable comm
        if let Some(comm) = LinuxEventMonitor::read_proc_comm(1) {
            assert!(!comm.is_empty());
        }
        // Not found PID
        assert!(LinuxEventMonitor::read_proc_comm(u32::MAX).is_none());
    }

    #[test]
    fn test_wayland_detection() {
        // Just verify it doesn't panic
        let _ = LinuxEventMonitor::is_wayland();
    }
}
