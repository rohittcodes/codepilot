/// A single completed edit, shown in the Ctrl+O detail view and persisted to
/// `{save_state_dir}/history.jsonl` so it survives a restart.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditDetail {
    pub task: String,
    pub path: std::path::PathBuf,
    pub content: String,
    pub bytes: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// `true` if the edit passed verification and was kept; `false` if it was
    /// rejected and reverted. `#[serde(default)]` so history written before the
    /// verification gate existed still loads.
    #[serde(default = "default_applied")]
    pub applied: bool,
    /// Human-readable gate result, e.g. "tsc: passed" or "tsc: failed - <error>".
    #[serde(default)]
    pub verification: Option<String>,
}

fn default_applied() -> bool {
    true
}

#[derive(Debug)]
pub struct AppState {
    pub input_text: String,
    pub cursor_position: usize,
    pub is_input_mode: bool,
    pub messages: Vec<String>,
    pub is_processing: bool,
    pub show_help: bool,
    pub message_scroll: usize, // Scroll position for messages
    pub messages_expanded: Vec<String>, // Expanded messages with line wrapping
    pub target_repo_path: String,
    pub show_details: bool,
    pub edit_history: Vec<EditDetail>,
    pub detail_cursor: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input_text: String::new(),
            cursor_position: 0,
            is_input_mode: false,
            messages: Vec::new(),
            is_processing: false,
            show_help: false,
            message_scroll: 0,
            messages_expanded: Vec::new(),
            target_repo_path: String::new(),
            show_details: false,
            edit_history: Vec::new(),
            detail_cursor: 0,
        }
    }

    /// Record a new edit and point the detail view at it.
    pub fn push_edit_detail(&mut self, detail: EditDetail) {
        self.edit_history.push(detail);
        self.detail_cursor = self.edit_history.len() - 1;
    }

    pub fn show_older_detail(&mut self) {
        self.detail_cursor = self.detail_cursor.saturating_sub(1);
    }

    pub fn show_newer_detail(&mut self) {
        if self.detail_cursor + 1 < self.edit_history.len() {
            self.detail_cursor += 1;
        }
    }

    pub fn add_message(&mut self, message: String) {
        // Add timestamp to messages
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let formatted_message = format!("[{}] {}", timestamp, message);
        self.messages.push(formatted_message);
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    pub fn scroll_messages_up(&mut self) {
        if self.message_scroll > 0 {
            self.message_scroll = self.message_scroll.saturating_sub(3); // Scroll faster
        }
    }

    pub fn scroll_messages_down(&mut self) {
        let max_scroll = self.messages_expanded.len().saturating_sub(10);
        if self.message_scroll < max_scroll {
            self.message_scroll = (self.message_scroll + 3).min(max_scroll); // Scroll faster
        }
    }

    pub fn update_messages_expanded(&mut self, width: usize) {
        self.messages_expanded.clear();
        for message in &self.messages {
            let wrapped = self.wrap_message(message, width.saturating_sub(4));
            self.messages_expanded.extend(wrapped);
        }
        // Auto-scroll to bottom when new messages are added
        let max_scroll = self.messages_expanded.len().saturating_sub(10);
        self.message_scroll = max_scroll;
    }

    fn wrap_message(&self, message: &str, width: usize) -> Vec<String> {
        if width < 20 {
            return vec![message.to_string()];
        }

        let mut wrapped = Vec::new();
        let lines: Vec<&str> = message.lines().collect();

        for line in lines {
            if line.len() <= width {
                wrapped.push(line.to_string());
            } else {
                // Handle long lines by breaking at word boundaries
                let words: Vec<&str> = line.split_whitespace().collect();
                let mut current_line = String::new();

                for word in words {
                    if current_line.is_empty() {
                        current_line = word.to_string();
                    } else if current_line.len() + word.len() + 1 <= width {
                        current_line.push(' ');
                        current_line.push_str(word);
                    } else {
                        wrapped.push(current_line.clone());
                        current_line = word.to_string();
                    }
                }

                if !current_line.is_empty() {
                    wrapped.push(current_line);
                }
            }
        }

        // If no lines were created, return the original message
        if wrapped.is_empty() {
            wrapped.push(message.to_string());
        }

        wrapped
    }
}
