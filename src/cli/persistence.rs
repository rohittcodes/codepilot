use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::state::EditDetail;

fn history_path(save_state_dir: &str) -> PathBuf {
    Path::new(save_state_dir).join("history.jsonl")
}

/// Append one edit to `{save_state_dir}/history.jsonl`, one JSON object per line.
pub fn append_entry(save_state_dir: &str, entry: &EditDetail) -> anyhow::Result<()> {
    std::fs::create_dir_all(save_state_dir)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path(save_state_dir))?;
    writeln!(file, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

/// Load prior edits from `{save_state_dir}/history.jsonl`, if it exists.
/// Missing file or unreadable lines are treated as "no history" rather than an error,
/// since this is best-effort session restore, not load-bearing state.
pub fn load_entries(save_state_dir: &str) -> Vec<EditDetail> {
    let Ok(content) = std::fs::read_to_string(history_path(save_state_dir)) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_entries_across_appends() {
        let dir = std::env::temp_dir().join(format!("codepilot-test-{}", std::process::id()));
        let dir_str = dir.to_str().unwrap();

        let first = EditDetail {
            task: "add a function".to_string(),
            path: PathBuf::from("src/lib.ts"),
            content: "export function add(a: number, b: number) {\n  return a + b;\n}\n".to_string(),
            bytes: 50,
            timestamp: chrono::Utc::now(),
            applied: true,
            verification: Some("tsc: passed".to_string()),
        };
        let second = EditDetail {
            task: "add a test".to_string(),
            path: PathBuf::from("src/lib.test.ts"),
            content: "test('add', () => {});\n".to_string(),
            bytes: 24,
            timestamp: chrono::Utc::now(),
            applied: false,
            verification: Some("tsc: failed - TS2304: Cannot find name 'test'".to_string()),
        };

        append_entry(dir_str, &first).unwrap();
        append_entry(dir_str, &second).unwrap();

        let loaded = load_entries(dir_str);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].task, "add a function");
        assert_eq!(loaded[1].path, PathBuf::from("src/lib.test.ts"));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_history_file_loads_as_empty() {
        let dir = std::env::temp_dir().join(format!("codepilot-test-missing-{}", std::process::id()));
        assert!(load_entries(dir.to_str().unwrap()).is_empty());
    }
}
