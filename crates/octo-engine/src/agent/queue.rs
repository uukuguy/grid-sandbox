//! Message queue for Steering/FollowUp messages.

use std::collections::VecDeque;

/// Message queue kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    /// Steering messages (high priority).
    Steering,
    /// Follow-up messages.
    FollowUp,
}

/// Queue processing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueMode {
    /// Process all messages at once.
    All,
    /// Process one message at a time.
    OneAtATime,
}

/// A message entry in the queue.
#[derive(Debug, Clone)]
pub struct QueueEntry {
    /// The message content.
    pub content: String,
    /// Queue kind.
    pub kind: QueueKind,
    /// Timestamp.
    pub timestamp: std::time::Instant,
}

impl QueueEntry {
    /// Create a new queue entry.
    pub fn new(content: String, kind: QueueKind) -> Self {
        Self {
            content,
            kind,
            timestamp: std::time::Instant::now(),
        }
    }
}

/// Message queue for Steering/FollowUp messages.
#[derive(Debug)]
pub struct MessageQueue {
    /// Steering messages queue.
    steering: VecDeque<QueueEntry>,
    /// Follow-up messages queue.
    follow_up: VecDeque<QueueEntry>,
    /// Steering queue processing mode.
    steering_mode: QueueMode,
    /// Follow-up queue processing mode.
    follow_up_mode: QueueMode,
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(QueueMode::OneAtATime, QueueMode::OneAtATime)
    }
}

impl MessageQueue {
    /// Create a new message queue with specified modes.
    pub fn new(steering_mode: QueueMode, follow_up_mode: QueueMode) -> Self {
        Self {
            steering: VecDeque::new(),
            follow_up: VecDeque::new(),
            steering_mode,
            follow_up_mode,
        }
    }

    /// Push a steering message.
    pub fn push_steering(&mut self, message: String) {
        self.steering.push_back(QueueEntry::new(message, QueueKind::Steering));
    }

    /// Push a follow-up message.
    pub fn push_followup(&mut self, message: String) {
        self.follow_up.push_back(QueueEntry::new(message, QueueKind::FollowUp));
    }

    /// Push a message to a specific queue.
    pub fn push(&mut self, kind: QueueKind, message: String) {
        match kind {
            QueueKind::Steering => self.push_steering(message),
            QueueKind::FollowUp => self.push_followup(message),
        }
    }

    /// Drain all steering messages.
    pub fn drain_steering(&mut self) -> Vec<String> {
        match self.steering_mode {
            QueueMode::All => {
                std::mem::take(&mut self.steering)
                    .into_iter()
                    .map(|e| e.content)
                    .collect()
            }
            QueueMode::OneAtATime => {
                self.steering.pop_front().map(|e| vec![e.content]).unwrap_or_default()
            }
        }
    }

    /// Drain all follow-up messages.
    pub fn drain_followup(&mut self) -> Vec<String> {
        match self.follow_up_mode {
            QueueMode::All => {
                std::mem::take(&mut self.follow_up)
                    .into_iter()
                    .map(|e| e.content)
                    .collect()
            }
            QueueMode::OneAtATime => {
                self.follow_up.pop_front().map(|e| vec![e.content]).unwrap_or_default()
            }
        }
    }

    /// Drain messages from a specific queue.
    pub fn drain(&mut self, kind: QueueKind) -> Vec<String> {
        match kind {
            QueueKind::Steering => self.drain_steering(),
            QueueKind::FollowUp => self.drain_followup(),
        }
    }

    /// Peek at steering messages without removing.
    pub fn peek_steering(&self) -> Vec<&str> {
        self.steering.iter().map(|e| e.content.as_str()).collect()
    }

    /// Peek at follow-up messages without removing.
    pub fn peek_followup(&self) -> Vec<&str> {
        self.follow_up.iter().map(|e| e.content.as_str()).collect()
    }

    /// Check if steering queue is empty.
    pub fn is_steering_empty(&self) -> bool {
        self.steering.is_empty()
    }

    /// Check if follow-up queue is empty.
    pub fn is_followup_empty(&self) -> bool {
        self.follow_up.is_empty()
    }

    /// Check if all queues are empty.
    pub fn is_empty(&self) -> bool {
        self.steering.is_empty() && self.follow_up.is_empty()
    }

    /// Get total number of messages.
    pub fn len(&self) -> usize {
        self.steering.len() + self.follow_up.len()
    }

    /// Clear all queues.
    pub fn clear(&mut self) {
        self.steering.clear();
        self.follow_up.clear();
    }

    /// Get steering queue processing mode.
    pub fn steering_mode(&self) -> QueueMode {
        self.steering_mode
    }

    /// Get follow-up queue processing mode.
    pub fn followup_mode(&self) -> QueueMode {
        self.follow_up_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_drain_steering() {
        let mut queue = MessageQueue::default();
        assert!(queue.is_steering_empty());

        queue.push_steering("message 1".into());
        queue.push_steering("message 2".into());

        // OneAtATime mode: drain returns one message at a time
        let msgs = queue.drain_steering();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "message 1");

        // Still has one message left
        assert!(!queue.is_steering_empty());

        // Drain the second message
        let msgs2 = queue.drain_steering();
        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs2[0], "message 2");

        // Now empty
        assert!(queue.is_steering_empty());
    }

    #[test]
    fn test_push_and_drain_all() {
        let mut queue = MessageQueue::new(QueueMode::All, QueueMode::All);

        queue.push_steering("s1".into());
        queue.push_steering("s2".into());
        queue.push_followup("f1".into());

        let steering = queue.drain_steering();
        assert_eq!(steering.len(), 2);

        let followup = queue.drain_followup();
        assert_eq!(followup.len(), 1);
    }

    #[test]
    fn test_queue_mode_all() {
        let mut queue = MessageQueue::new(QueueMode::All, QueueMode::All);

        queue.push_steering("1".into());
        queue.push_steering("2".into());
        queue.push_steering("3".into());

        let msgs = queue.drain_steering();
        assert_eq!(msgs.len(), 3);
        assert!(queue.is_steering_empty());
    }

    #[test]
    fn test_queue_mode_one_at_a_time() {
        let mut queue = MessageQueue::new(QueueMode::OneAtATime, QueueMode::OneAtATime);

        queue.push_steering("1".into());
        queue.push_steering("2".into());

        let msg1 = queue.drain_steering();
        assert_eq!(msg1.len(), 1);
        assert_eq!(msg1[0], "1");

        let msg2 = queue.drain_steering();
        assert_eq!(msg2.len(), 1);
        assert_eq!(msg2[0], "2");

        let msg3 = queue.drain_steering();
        assert!(msg3.is_empty());
    }

    #[test]
    fn test_len() {
        let mut queue = MessageQueue::default();
        assert_eq!(queue.len(), 0);

        queue.push_steering("s1".into());
        assert_eq!(queue.len(), 1);

        queue.push_followup("f1".into());
        assert_eq!(queue.len(), 2);

        queue.drain_steering();
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut queue = MessageQueue::default();
        queue.push_steering("s1".into());
        queue.push_followup("f1".into());

        queue.clear();
        assert!(queue.is_empty());
    }
}
