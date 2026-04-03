//! BlobGc — garbage collector for BlobStore using TTL + capacity policy.
//!
//! Two-phase cleanup:
//! 1. **TTL**: Delete blobs older than `max_age`.
//! 2. **Capacity**: If total size still exceeds `max_total_bytes`, evict
//!    oldest blobs until under limit.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use tracing::{debug, info};

/// Metadata for a single blob file.
struct BlobEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

/// Blob garbage collector with TTL + capacity dual strategy.
pub struct BlobGc {
    base_dir: PathBuf,
    max_age: Duration,
    max_total_bytes: u64,
}

impl BlobGc {
    pub fn new(base_dir: PathBuf, max_age: Duration, max_total_bytes: u64) -> Self {
        Self {
            base_dir,
            max_age,
            max_total_bytes,
        }
    }

    /// Default: 7-day TTL + 1 GB capacity limit.
    pub fn with_defaults(base_dir: PathBuf) -> Self {
        Self::new(base_dir, Duration::from_secs(7 * 86400), 1_073_741_824)
    }

    /// Run garbage collection. Returns `(deleted_count, freed_bytes)`.
    pub fn collect(&self) -> Result<(usize, u64)> {
        let mut entries = Vec::new();
        self.walk_blobs(&mut entries)?;

        if entries.is_empty() {
            return Ok((0, 0));
        }

        let mut deleted = 0usize;
        let mut freed = 0u64;
        let now = SystemTime::now();

        // Phase 1: TTL cleanup
        for entry in &entries {
            if let Ok(age) = now.duration_since(entry.modified) {
                if age > self.max_age {
                    if std::fs::remove_file(&entry.path).is_ok() {
                        debug!(path = %entry.path.display(), "BlobGc: TTL evicted");
                        deleted += 1;
                        freed += entry.size;
                    }
                }
            }
        }

        // Phase 2: Capacity cleanup (oldest-first eviction)
        let remaining: Vec<&BlobEntry> = entries.iter().filter(|e| e.path.exists()).collect();
        let total: u64 = remaining.iter().map(|e| e.size).sum();

        if total > self.max_total_bytes {
            let mut sorted = remaining;
            sorted.sort_by_key(|e| e.modified);
            let mut current_total = total;
            for entry in sorted {
                if current_total <= self.max_total_bytes {
                    break;
                }
                if std::fs::remove_file(&entry.path).is_ok() {
                    debug!(path = %entry.path.display(), "BlobGc: capacity evicted");
                    deleted += 1;
                    freed += entry.size;
                    current_total -= entry.size;
                }
            }
        }

        info!(deleted, freed_bytes = freed, "BlobGc: collection complete");
        Ok((deleted, freed))
    }

    /// Walk the two-level blob directory structure, collecting file metadata.
    fn walk_blobs(&self, entries: &mut Vec<BlobEntry>) -> Result<()> {
        if !self.base_dir.exists() {
            return Ok(());
        }

        for prefix_entry in std::fs::read_dir(&self.base_dir)? {
            let prefix_entry = prefix_entry?;
            let prefix_path = prefix_entry.path();
            if !prefix_path.is_dir() {
                continue;
            }
            for blob_entry in std::fs::read_dir(&prefix_path)? {
                let blob_entry = blob_entry?;
                let blob_path = blob_entry.path();
                if !blob_path.is_file() {
                    continue;
                }
                let meta = std::fs::metadata(&blob_path)?;
                entries.push(BlobEntry {
                    path: blob_path,
                    size: meta.len(),
                    modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                });
            }
        }
        Ok(())
    }

    /// Get the base directory.
    pub fn base_dir(&self) -> &std::path::Path {
        &self.base_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::BlobStore;
    use std::thread;
    use tempfile::TempDir;

    fn setup() -> (BlobStore, BlobGc, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = BlobStore::new(dir.path().to_path_buf());
        let gc = BlobGc::new(dir.path().to_path_buf(), Duration::from_secs(1), 10_000);
        (store, gc, dir)
    }

    #[test]
    fn test_ttl_expiry() {
        let (store, gc, _dir) = setup();
        store.store(b"old blob content").unwrap();
        // Wait for TTL to expire
        thread::sleep(Duration::from_secs(2));
        let (deleted, freed) = gc.collect().unwrap();
        assert_eq!(deleted, 1);
        assert!(freed > 0);
    }

    #[test]
    fn test_capacity_eviction() {
        let dir = TempDir::new().unwrap();
        let store = BlobStore::new(dir.path().to_path_buf());
        // 100-byte capacity limit, very short TTL so all survive TTL check
        let gc = BlobGc::new(dir.path().to_path_buf(), Duration::from_secs(3600), 100);

        // Store several blobs exceeding capacity
        for i in 0..10 {
            let content = format!("blob content number {} with some padding", i);
            store.store(content.as_bytes()).unwrap();
        }
        let (deleted, freed) = gc.collect().unwrap();
        assert!(deleted > 0, "Should have evicted some blobs");
        assert!(freed > 0);
    }

    #[test]
    fn test_empty_dir_no_error() {
        let dir = TempDir::new().unwrap();
        let gc = BlobGc::with_defaults(dir.path().to_path_buf());
        let (deleted, freed) = gc.collect().unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(freed, 0);
    }

    #[test]
    fn test_nonexistent_dir_no_error() {
        let gc = BlobGc::with_defaults(PathBuf::from("/tmp/nonexistent-blob-gc-test-dir"));
        let (deleted, freed) = gc.collect().unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(freed, 0);
    }
}
