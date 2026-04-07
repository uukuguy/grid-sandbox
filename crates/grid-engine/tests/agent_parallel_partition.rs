//! AV-T1: Tests for concurrent safety partitioning.

#[test]
fn test_partition_tools_by_concurrency_safety() {
    let tools = vec![
        ("grep".to_string(), true),
        ("bash".to_string(), false),
        ("glob".to_string(), true),
        ("file_write".to_string(), false),
        ("file_read".to_string(), true),
    ];
    let (safe, unsafe_): (Vec<_>, Vec<_>) = tools
        .into_iter()
        .partition(|(_, is_safe)| *is_safe);
    assert_eq!(safe.len(), 3);
    assert_eq!(unsafe_.len(), 2);
    assert_eq!(safe[0].0, "grep");
    assert_eq!(safe[1].0, "glob");
    assert_eq!(safe[2].0, "file_read");
    assert_eq!(unsafe_[0].0, "bash");
    assert_eq!(unsafe_[1].0, "file_write");
}

#[test]
fn test_partition_all_safe_tools() {
    let tools = vec![
        ("grep".to_string(), true),
        ("glob".to_string(), true),
        ("file_read".to_string(), true),
    ];
    let (safe, unsafe_): (Vec<_>, Vec<_>) = tools
        .into_iter()
        .partition(|(_, is_safe)| *is_safe);
    assert_eq!(safe.len(), 3);
    assert_eq!(unsafe_.len(), 0);
}

#[test]
fn test_partition_all_unsafe_tools() {
    let tools = vec![
        ("bash".to_string(), false),
        ("file_write".to_string(), false),
    ];
    let (safe, unsafe_): (Vec<_>, Vec<_>) = tools
        .into_iter()
        .partition(|(_, is_safe)| *is_safe);
    assert_eq!(safe.len(), 0);
    assert_eq!(unsafe_.len(), 2);
}

#[test]
fn test_merge_preserves_original_order() {
    // Simulate the indexed merge logic
    let safe_indices = vec![0, 2, 4];
    let unsafe_indices = vec![1, 3];
    let safe_results = vec!["grep_result", "glob_result", "file_read_result"];
    let unsafe_results = vec!["bash_result", "file_write_result"];

    let mut indexed: Vec<(usize, &str)> = Vec::new();
    for (i, &idx) in safe_indices.iter().enumerate() {
        indexed.push((idx, safe_results[i]));
    }
    for (i, &idx) in unsafe_indices.iter().enumerate() {
        indexed.push((idx, unsafe_results[i]));
    }
    indexed.sort_by_key(|(idx, _)| *idx);

    let merged: Vec<&str> = indexed.into_iter().map(|(_, r)| r).collect();
    assert_eq!(merged, vec![
        "grep_result",
        "bash_result",
        "glob_result",
        "file_write_result",
        "file_read_result",
    ]);
}
