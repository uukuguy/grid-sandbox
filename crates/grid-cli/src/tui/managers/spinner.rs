//! Spinner service for managing loading indicator state.
//!
//! Tracks whether a spinner is active, its message, and elapsed time
//! since activation.

use std::time::{Duration, Instant};

/// Service for managing spinner animation state and timing.
pub struct SpinnerService {
    active: bool,
    message: String,
    start_time: Option<Instant>,
}

impl SpinnerService {
    pub fn new() -> Self {
        Self {
            active: false,
            message: String::new(),
            start_time: None,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn start(&mut self, message: String) {
        self.message = message;
        self.start_time = Some(Instant::now());
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }
}

impl Default for SpinnerService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_inactive() {
        let svc = SpinnerService::new();
        assert!(!svc.active());
        assert!(svc.message().is_empty());
        assert_eq!(svc.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_start_stop() {
        let mut svc = SpinnerService::new();
        svc.start("Loading models...".into());
        assert!(svc.active());
        assert_eq!(svc.message(), "Loading models...");
        assert!(svc.elapsed() >= Duration::ZERO);

        svc.stop();
        assert!(!svc.active());
    }

    #[test]
    fn test_elapsed_increases() {
        let mut svc = SpinnerService::new();
        svc.start("Working".into());
        let _e = svc.elapsed();
    }
}
