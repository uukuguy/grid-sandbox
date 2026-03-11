use octo_types::ToolOutput;

/// Check if a path is a symlink and return an error ToolOutput if so.
/// Returns None if the path is NOT a symlink (safe to proceed).
/// Returns None if the path doesn't exist (file_write creates new files).
pub fn reject_symlink(path: &std::path::Path) -> Option<ToolOutput> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Some(ToolOutput::error(format!(
            "Refusing to follow symlink: {}",
            path.display()
        ))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_reject_symlink_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("target.txt");
        fs::write(&file_path, "hello").unwrap();

        let link_path = dir.path().join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file_path, &link_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file_path, &link_path).unwrap();

        let result = reject_symlink(&link_path);
        assert!(result.is_some(), "symlink should be rejected");
        let output = result.unwrap();
        assert!(output.is_error);
        assert!(output.content.contains("Refusing to follow symlink"));
    }

    #[test]
    fn test_regular_file_passes() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("regular.txt");
        fs::write(&file_path, "hello").unwrap();

        let result = reject_symlink(&file_path);
        assert!(result.is_none(), "regular file should pass");
    }

    #[test]
    fn test_nonexistent_path_passes() {
        let result = reject_symlink(std::path::Path::new("/tmp/does_not_exist_12345"));
        assert!(result.is_none(), "nonexistent path should pass");
    }
}
