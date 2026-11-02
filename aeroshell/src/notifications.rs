//! AeroShell notification daemon.
//!
//! Implements the freedesktop.org Desktop Notifications Specification
//! (org.freedesktop.Notifications D-Bus interface).
//!
//! Phase 3: In-memory notification queue, scheduling, expiry.
//! Phase 4: Rendered notification toasts on a Wayland surface.
//! Phase 5: D-Bus integration (listen for real app notifications).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use slog::{debug, info};

/// A single notification entry.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub urgency: Urgency,
    pub created_at: Instant,
    /// None = use default duration from config.
    pub timeout_ms: Option<u64>,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Urgency {
    pub fn from_hint(value: u8) -> Self {
        match value {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }
}

/// Notification manager — holds the queue and handles expiry.
pub struct NotificationManager {
    queue: VecDeque<Notification>,
    next_id: u32,
    default_timeout: Duration,
    max_visible: usize,
}

impl NotificationManager {
    pub fn new(default_timeout_ms: u64, max_visible: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            next_id: 1,
            default_timeout: Duration::from_millis(default_timeout_ms),
            max_visible,
        }
    }

    /// Add a notification. Returns the assigned ID.
    pub fn notify(
        &mut self,
        app_name: &str,
        summary: &str,
        body: &str,
        urgency: Urgency,
        timeout_ms: Option<u64>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(Notification {
            id,
            app_name: app_name.to_string(),
            summary: summary.to_string(),
            body: body.to_string(),
            urgency,
            created_at: Instant::now(),
            timeout_ms,
            dismissed: false,
        });
        id
    }

    /// Dismiss a notification by ID.
    pub fn dismiss(&mut self, id: u32) {
        if let Some(n) = self.queue.iter_mut().find(|n| n.id == id) {
            n.dismissed = true;
        }
    }

    /// Remove expired and dismissed notifications. Returns IDs that were closed.
    pub fn tick(&mut self) -> Vec<u32> {
        let now = Instant::now();
        let default = self.default_timeout;

        let mut closed = Vec::new();
        self.queue.retain(|n| {
            let timeout = n.timeout_ms.map(Duration::from_millis).unwrap_or(default);
            let expired = now.duration_since(n.created_at) >= timeout;
            if expired || n.dismissed {
                closed.push(n.id);
                false
            } else {
                true
            }
        });
        closed
    }

    /// Returns the currently visible notifications (most recent first, capped).
    pub fn visible(&self) -> impl Iterator<Item = &Notification> {
        self.queue.iter().rev().take(self.max_visible)
    }

    pub fn count(&self) -> usize {
        self.queue.len()
    }
}

/// Run the notification daemon loop.
/// Phase 5 will listen on D-Bus. For now this is a background task that
/// ticks the manager every 100ms to expire old notifications.
pub async fn run(log: slog::Logger) {
    info!(log, "Notification daemon started (Phase 3 — no D-Bus yet)");
    let mut manager = NotificationManager::new(5000, 5);

    // Self-test: post a startup notification.
    let id = manager.notify(
        "AeroShell",
        "Welcome to FrutigerOS",
        "AeroShell has started. Tomorrow, Reimagined.",
        Urgency::Normal,
        Some(8000),
    );
    debug!(log, "Posted startup notification id={}", id);

    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let closed = manager.tick();
        for id in closed {
            debug!(log, "Notification {} expired/dismissed", id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_increments_id() {
        let mut m = NotificationManager::new(5000, 5);
        let a = m.notify("app", "hello", "world", Urgency::Normal, None);
        let b = m.notify("app", "hello2", "world2", Urgency::Normal, None);
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[test]
    fn dismiss_removes_notification() {
        let mut m = NotificationManager::new(5000, 5);
        let id = m.notify("app", "s", "b", Urgency::Normal, None);
        m.dismiss(id);
        let closed = m.tick();
        assert!(closed.contains(&id));
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn urgency_from_hint() {
        assert_eq!(Urgency::from_hint(0), Urgency::Low);
        assert_eq!(Urgency::from_hint(1), Urgency::Normal);
        assert_eq!(Urgency::from_hint(2), Urgency::Critical);
        assert_eq!(Urgency::from_hint(99), Urgency::Normal);
    }

    #[test]
    fn max_visible_caps_output() {
        let mut m = NotificationManager::new(60000, 3); // 60s so nothing expires
        for i in 0..5 {
            m.notify("app", &format!("n{}", i), "", Urgency::Normal, None);
        }
        assert_eq!(m.visible().count(), 3);
    }
}
