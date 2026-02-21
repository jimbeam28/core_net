// src/protocols/icmp/global.rs
//
// ICMP global state management
// Used to track pending Echo requests (matching requests and replies)

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::protocols::Ipv4Addr;

// ========== ICMP Configuration ==========

/// ICMP configuration parameters
#[derive(Debug, Clone)]
pub struct IcmpConfig {
    /// Echo request default timeout
    pub echo_timeout: Duration,

    /// Maximum pending Echo requests
    pub max_pending_echo: usize,

    /// Echo reply rate limit (max replies per second)
    pub max_echo_replies_per_sec: usize,

    /// Rate limit time window (seconds)
    pub rate_limit_window_secs: usize,
}

impl Default for IcmpConfig {
    fn default() -> Self {
        Self {
            echo_timeout: Duration::from_secs(1),
            max_pending_echo: 100,
            max_echo_replies_per_sec: 100,
            rate_limit_window_secs: 1,
        }
    }
}

// ========== Pending Echo Entry ==========

/// Pending Echo request entry
#[derive(Debug, Clone)]
pub struct PendingEcho {
    /// Identifier
    pub identifier: u16,

    /// Sequence number
    pub sequence: u16,

    /// Send timestamp
    pub sent_at: Instant,

    /// Destination address
    pub destination: Ipv4Addr,
}

impl PendingEcho {
    /// Create a new pending Echo request
    pub fn new(identifier: u16, sequence: u16, destination: Ipv4Addr) -> Self {
        PendingEcho {
            identifier,
            sequence,
            sent_at: Instant::now(),
            destination,
        }
    }

    /// Check if timed out
    pub fn is_timeout(&self, timeout: Duration) -> bool {
        self.sent_at.elapsed() >= timeout
    }

    /// Calculate round trip time
    pub fn rtt(&self) -> Duration {
        self.sent_at.elapsed()
    }
}

// ========== Echo Manager ==========

/// Echo request manager
pub struct EchoManager {
    /// Pending Echo requests (key: identifier, sequence, destination)
    pending: HashMap<(u16, u16, Ipv4Addr), PendingEcho>,

    /// Configuration
    config: IcmpConfig,

    /// Current window count
    current_window_count: usize,

    /// Window start time
    window_start: Instant,
}

impl EchoManager {
    /// Create new Echo manager with config
    pub fn new(config: IcmpConfig) -> Self {
        let now = Instant::now();
        Self {
            pending: HashMap::new(),
            config,
            current_window_count: 0,
            window_start: now,
        }
    }

    /// Create Echo manager with default config
    pub fn with_defaults() -> Self {
        Self::new(IcmpConfig::default())
    }

    /// Add pending Echo request
    pub fn add_pending(&mut self, echo: PendingEcho) -> Result<(), String> {
        self.cleanup_timeouts();

        if self.pending.len() >= self.config.max_pending_echo {
            let msg = format!(
                "Echo manager full: {} >= {}",
                self.pending.len(),
                self.config.max_pending_echo
            );
            return Err(msg);
        }

        let key = (echo.identifier, echo.sequence, echo.destination);
        self.pending.insert(key, echo);
        Ok(())
    }

    /// Remove pending Echo request
    pub fn remove_pending(&mut self, identifier: u16, sequence: u16, destination: Ipv4Addr) -> Option<PendingEcho> {
        let key = (identifier, sequence, destination);
        self.pending.remove(&key)
    }

    /// Get pending Echo request without removing
    pub fn get_pending(&self, identifier: u16, sequence: u16, destination: Ipv4Addr) -> Option<&PendingEcho> {
        let key = (identifier, sequence, destination);
        self.pending.get(&key)
    }

    /// Cleanup timed out requests
    pub fn cleanup_timeouts(&mut self) {
        self.pending.retain(|_, echo| !echo.is_timeout(self.config.echo_timeout));
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending requests
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Check if Echo reply can be sent (rate limit)
    pub fn can_send_echo_reply(&mut self) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.config.rate_limit_window_secs as u64);

        if now.duration_since(self.window_start) >= window_duration {
            self.window_start = now;
            self.current_window_count = 0;
        }

        if self.current_window_count >= self.config.max_echo_replies_per_sec {
            return false;
        }

        self.current_window_count += 1;
        true
    }

    /// Get config reference
    pub fn config(&self) -> &IcmpConfig {
        &self.config
    }

    /// Update config
    pub fn set_config(&mut self, config: IcmpConfig) {
        self.config = config;
    }
}

impl Default for EchoManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_manager_add_remove() {
        let mut manager = EchoManager::default();

        let dest = Ipv4Addr::new(192, 168, 1, 1);
        let echo = PendingEcho::new(1234, 1, dest);
        manager.add_pending(echo.clone()).unwrap();

        assert_eq!(manager.pending_count(), 1);

        let removed = manager.remove_pending(1234, 1, dest);
        assert!(removed.is_some());
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_echo_manager_cleanup() {
        let mut config = IcmpConfig::default();
        config.echo_timeout = Duration::from_millis(100);
        let mut manager = EchoManager::new(config);

        let dest = Ipv4Addr::new(192, 168, 1, 1);
        let echo = PendingEcho::new(1234, 1, dest);
        manager.add_pending(echo).unwrap();

        std::thread::sleep(Duration::from_millis(150));
        manager.cleanup_timeouts();

        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_echo_manager_key_includes_destination() {
        let mut manager = EchoManager::default();

        let dest1 = Ipv4Addr::new(192, 168, 1, 1);
        let dest2 = Ipv4Addr::new(192, 168, 1, 2);

        // Add two requests with same identifier+sequence but different destinations
        let echo1 = PendingEcho::new(1234, 1, dest1);
        let echo2 = PendingEcho::new(1234, 1, dest2);

        manager.add_pending(echo1).unwrap();
        manager.add_pending(echo2).unwrap();

        // Should have two separate entries
        assert_eq!(manager.pending_count(), 2);

        // Remove one
        manager.remove_pending(1234, 1, dest1);
        assert_eq!(manager.pending_count(), 1);

        // Other should still exist
        assert!(manager.get_pending(1234, 1, dest2).is_some());
    }

    #[test]
    fn test_rate_limit() {
        let mut config = IcmpConfig::default();
        config.max_echo_replies_per_sec = 2;
        let mut manager = EchoManager::new(config);

        // First two should succeed
        assert!(manager.can_send_echo_reply());
        assert!(manager.can_send_echo_reply());

        // Third should fail (rate limit exceeded)
        assert!(!manager.can_send_echo_reply());
    }
}
