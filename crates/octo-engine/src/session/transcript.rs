//! TranscriptWriter — append-only JSONL session transcript.
//!
//! Each conversation message is logged as a `TranscriptEntry` line in a
//! `.transcript.jsonl` file. Large content is referenced via blob hashes
//! rather than inlined, keeping transcripts compact.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single JSONL transcript entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    /// First 500 characters of the content (for quick scanning).
    pub content_preview: String,
    /// If the full content was externalized to BlobStore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
}

/// Maximum preview length (characters) stored inline.
const MAX_PREVIEW_LEN: usize = 500;

/// Create a content preview by truncating at `MAX_PREVIEW_LEN`.
pub fn make_preview(content: &str) -> String {
    if content.len() <= MAX_PREVIEW_LEN {
        content.to_string()
    } else {
        let mut end = MAX_PREVIEW_LEN;
        while !content.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &content[..end])
    }
}

/// Append-only JSONL transcript writer.
pub struct TranscriptWriter {
    file_path: PathBuf,
}

impl TranscriptWriter {
    /// Create a new writer. The file is created lazily on first `append`.
    pub fn new(session_dir: PathBuf, session_id: &str) -> Self {
        let file_path = session_dir.join(format!("{}.transcript.jsonl", session_id));
        Self { file_path }
    }

    /// Append a single entry as a JSONL line.
    pub fn append(&self, entry: &TranscriptEntry) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Read all entries from the transcript file.
    pub fn read_all(&self) -> anyhow::Result<Vec<TranscriptEntry>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.file_path)?;
        content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).map_err(Into::into))
            .collect()
    }

    /// Get the transcript file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Compress the transcript file to gzip format (AR-D1).
    ///
    /// Creates a `.transcript.jsonl.gz` file and removes the original JSONL.
    /// Returns the path to the compressed file, or None if no transcript exists.
    pub fn compress(&self) -> anyhow::Result<Option<PathBuf>> {
        use std::io::Read;

        if !self.file_path.exists() {
            return Ok(None);
        }

        let gz_path = PathBuf::from(format!("{}.gz", self.file_path.display()));
        let content = fs::read(&self.file_path)?;

        let gz_file = fs::File::create(&gz_path)?;
        let mut encoder = flate2::write::GzEncoder::new(gz_file, flate2::Compression::default());
        encoder.write_all(&content)?;
        encoder.finish()?;

        fs::remove_file(&self.file_path)?;
        Ok(Some(gz_path))
    }

    /// Read entries from a compressed transcript (AR-D1).
    pub fn read_compressed(gz_path: &Path) -> anyhow::Result<Vec<TranscriptEntry>> {
        use std::io::Read;

        let file = fs::File::open(gz_path)?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut content = String::new();
        decoder.read_to_string(&mut content)?;

        content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).map_err(Into::into))
            .collect()
    }

    /// List all transcript files (both JSONL and compressed) in a directory.
    pub fn list_transcripts(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut files: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name.ends_with(".transcript.jsonl") || name.ends_with(".transcript.jsonl.gz")
            })
            .collect();
        files.sort();
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_writer() -> (TranscriptWriter, TempDir) {
        let dir = TempDir::new().unwrap();
        let writer = TranscriptWriter::new(dir.path().to_path_buf(), "test-session");
        (writer, dir)
    }

    fn sample_entry(role: &str, content: &str) -> TranscriptEntry {
        TranscriptEntry {
            timestamp: Utc::now(),
            role: role.to_string(),
            content_preview: make_preview(content),
            blob_ref: None,
            tool_name: None,
            input_tokens: None,
            output_tokens: None,
        }
    }

    #[test]
    fn test_append_and_read_roundtrip() {
        let (writer, _dir) = test_writer();
        let entry = sample_entry("user", "Hello, world!");
        writer.append(&entry).unwrap();

        let entries = writer.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].role, "user");
        assert_eq!(entries[0].content_preview, "Hello, world!");
    }

    #[test]
    fn test_blob_ref_written() {
        let (writer, _dir) = test_writer();
        let mut entry = sample_entry("assistant", "short preview");
        entry.blob_ref = Some("[blob:sha256:abc123]".to_string());
        writer.append(&entry).unwrap();

        let entries = writer.read_all().unwrap();
        assert_eq!(entries[0].blob_ref.as_deref(), Some("[blob:sha256:abc123]"));
    }

    #[test]
    fn test_empty_file_returns_empty_vec() {
        let (writer, _dir) = test_writer();
        let entries = writer.read_all().unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_multiple_appends_preserve_order() {
        let (writer, _dir) = test_writer();
        for i in 0..5 {
            let entry = sample_entry("user", &format!("message {}", i));
            writer.append(&entry).unwrap();
        }
        let entries = writer.read_all().unwrap();
        assert_eq!(entries.len(), 5);
        assert!(entries[0].content_preview.contains("message 0"));
        assert!(entries[4].content_preview.contains("message 4"));
    }

    #[test]
    fn test_make_preview_short() {
        let short = "Hello";
        assert_eq!(make_preview(short), "Hello");
    }

    #[test]
    fn test_make_preview_long() {
        let long = "a".repeat(600);
        let preview = make_preview(&long);
        assert!(preview.len() < 510); // 500 + "..."
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn test_compress_and_read_compressed() {
        let (writer, _dir) = test_writer();
        for i in 0..3 {
            writer.append(&sample_entry("user", &format!("msg {}", i))).unwrap();
        }

        // Compress
        let gz_path = writer.compress().unwrap().expect("should compress");
        assert!(gz_path.exists());
        assert!(!writer.file_path().exists()); // original removed

        // Read compressed
        let entries = TranscriptWriter::read_compressed(&gz_path).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].content_preview.contains("msg 0"));
    }

    #[test]
    fn test_compress_empty_returns_none() {
        let (writer, _dir) = test_writer();
        let result = writer.compress().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_transcripts() {
        let dir = TempDir::new().unwrap();
        let w1 = TranscriptWriter::new(dir.path().to_path_buf(), "sess-a");
        let w2 = TranscriptWriter::new(dir.path().to_path_buf(), "sess-b");
        w1.append(&sample_entry("user", "hello")).unwrap();
        w2.append(&sample_entry("user", "world")).unwrap();

        let files = TranscriptWriter::list_transcripts(dir.path()).unwrap();
        assert_eq!(files.len(), 2);

        // Compress one and re-list
        w1.compress().unwrap();
        let files = TranscriptWriter::list_transcripts(dir.path()).unwrap();
        assert_eq!(files.len(), 2); // 1 jsonl + 1 gz
    }
}
