//! Input message history with up/down navigation and file persistence.

use std::path::PathBuf;

/// Manages a bounded history of sent messages with cursor-based navigation.
pub struct MessageHistory {
    history: Vec<String>,
    cursor: usize,
    capacity: usize,
    navigating: bool,
    file_path: Option<PathBuf>,
}

impl MessageHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: Vec::new(),
            cursor: 0,
            capacity,
            navigating: false,
            file_path: None,
        }
    }

    /// Create with file-backed persistence. Loads existing history from file.
    pub fn with_file(capacity: usize, path: PathBuf) -> Self {
        let mut hist = Self {
            history: Vec::new(),
            cursor: 0,
            capacity,
            navigating: false,
            file_path: Some(path.clone()),
        };
        hist.load_from_file();
        hist
    }

    pub fn push(&mut self, msg: String) {
        if msg.is_empty() {
            return;
        }
        if self.history.last().map(|s| s.as_str()) == Some(&msg) {
            self.reset_cursor();
            return;
        }
        self.history.push(msg);
        if self.history.len() > self.capacity {
            self.history.remove(0);
        }
        self.reset_cursor();
        self.save_to_file();
    }

    pub fn up(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }
        if !self.navigating {
            self.navigating = true;
            self.cursor = self.history.len() - 1;
        } else if self.cursor > 0 {
            self.cursor -= 1;
        }
        Some(&self.history[self.cursor])
    }

    pub fn down(&mut self) -> Option<&str> {
        if !self.navigating || self.history.is_empty() {
            return None;
        }
        if self.cursor < self.history.len() - 1 {
            self.cursor += 1;
            Some(&self.history[self.cursor])
        } else {
            self.navigating = false;
            None
        }
    }

    pub fn reset_cursor(&mut self) {
        self.cursor = 0;
        self.navigating = false;
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn is_navigating(&self) -> bool {
        self.navigating
    }

    /// Get all history entries as a cloned vector (for search).
    pub fn entries(&self) -> Vec<String> {
        self.history.clone()
    }

    /// Load history from file (one entry per line, newlines escaped as \n).
    fn load_from_file(&mut self) {
        let path = match &self.file_path {
            Some(p) => p,
            None => return,
        };
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return,
        };
        for line in content.lines() {
            if !line.is_empty() {
                let msg = line.replace("\\n", "\n");
                self.history.push(msg);
            }
        }
        // Trim to capacity
        while self.history.len() > self.capacity {
            self.history.remove(0);
        }
    }

    /// Save history to file.
    fn save_to_file(&self) {
        let path = match &self.file_path {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content: String = self
            .history
            .iter()
            .map(|s| s.replace('\n', "\\n"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(path, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let hist = MessageHistory::new(100);
        assert!(hist.is_empty());
        assert_eq!(hist.len(), 0);
    }

    #[test]
    fn test_push_and_navigate() {
        let mut hist = MessageHistory::new(100);
        hist.push("first".into());
        hist.push("second".into());
        hist.push("third".into());
        assert_eq!(hist.len(), 3);

        assert_eq!(hist.up(), Some("third"));
        assert_eq!(hist.up(), Some("second"));
        assert_eq!(hist.up(), Some("first"));
        assert_eq!(hist.up(), Some("first"));

        assert_eq!(hist.down(), Some("second"));
        assert_eq!(hist.down(), Some("third"));
        assert_eq!(hist.down(), None);
    }

    #[test]
    fn test_capacity_eviction() {
        let mut hist = MessageHistory::new(3);
        hist.push("a".into());
        hist.push("b".into());
        hist.push("c".into());
        hist.push("d".into());
        assert_eq!(hist.len(), 3);

        assert_eq!(hist.up(), Some("d"));
        assert_eq!(hist.up(), Some("c"));
        assert_eq!(hist.up(), Some("b"));
        assert_eq!(hist.up(), Some("b"));
    }

    #[test]
    fn test_empty_push_ignored() {
        let mut hist = MessageHistory::new(100);
        hist.push("".into());
        assert!(hist.is_empty());
    }

    #[test]
    fn test_consecutive_duplicate_ignored() {
        let mut hist = MessageHistory::new(100);
        hist.push("same".into());
        hist.push("same".into());
        assert_eq!(hist.len(), 1);
    }

    #[test]
    fn test_up_empty() {
        let mut hist = MessageHistory::new(100);
        assert_eq!(hist.up(), None);
    }

    #[test]
    fn test_down_without_navigating() {
        let mut hist = MessageHistory::new(100);
        hist.push("msg".into());
        assert_eq!(hist.down(), None);
    }

    #[test]
    fn test_reset_cursor() {
        let mut hist = MessageHistory::new(100);
        hist.push("a".into());
        hist.push("b".into());
        hist.up();
        hist.reset_cursor();
        assert_eq!(hist.up(), Some("b"));
    }

    #[test]
    fn test_file_persistence() {
        let dir = std::env::temp_dir().join("octo_hist_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_history.txt");
        let _ = std::fs::remove_file(&path); // clean slate

        // Write history
        {
            let mut hist = MessageHistory::with_file(100, path.clone());
            hist.push("alpha".into());
            hist.push("beta".into());
            hist.push("gamma".into());
        }

        // Read it back
        {
            let mut hist = MessageHistory::with_file(100, path.clone());
            assert_eq!(hist.len(), 3);
            assert_eq!(hist.up(), Some("gamma"));
            assert_eq!(hist.up(), Some("beta"));
            assert_eq!(hist.up(), Some("alpha"));
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_file_persistence_multiline() {
        let dir = std::env::temp_dir().join("octo_hist_test_ml");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_history_ml.txt");
        let _ = std::fs::remove_file(&path);

        {
            let mut hist = MessageHistory::with_file(100, path.clone());
            hist.push("line1\nline2\nline3".into());
        }

        {
            let mut hist = MessageHistory::with_file(100, path.clone());
            assert_eq!(hist.len(), 1);
            assert_eq!(hist.up(), Some("line1\nline2\nline3"));
        }

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_is_navigating() {
        let mut hist = MessageHistory::new(100);
        assert!(!hist.is_navigating());
        hist.push("a".into());
        hist.up();
        assert!(hist.is_navigating());
        hist.reset_cursor();
        assert!(!hist.is_navigating());
    }
}
