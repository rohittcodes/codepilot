

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connected,
    Failed(String),
    Pending,
    NotTested,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub status: ConnectionStatus,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub tools: Vec<Tool>,
    pub is_expanded: bool,
    pub status: ConnectionStatus,
}

#[derive(Debug)]
pub struct AppState {
    pub services: Vec<Service>,
    pub selected_service: usize,
    pub selected_tool: Option<usize>,
    pub input_text: String,
    pub cursor_position: usize,
    pub is_input_mode: bool,
    pub messages: Vec<String>,
    pub is_processing: bool,
    pub show_help: bool,
    pub service_scroll: Vec<usize>, // Scroll position for each service
    pub message_scroll: usize, // Scroll position for messages
    pub messages_expanded: Vec<String>, // Expanded messages with line wrapping
}

impl AppState {
    pub fn new() -> Self {
        let services = vec![
            Service {
                name: "Linear".to_string(),
                tools: vec![
                    Tool {
                        name: "LINEAR_CREATE_LINEAR_ISSUE".to_string(),
                        description: "Create a new issue in Linear".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "LINEAR_LIST_LINEAR_ISSUES".to_string(),
                        description: "List all issues in Linear".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "LINEAR_UPDATE_ISSUE".to_string(),
                        description: "Update an existing issue".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                ],
                is_expanded: true,
                status: ConnectionStatus::NotTested,
            },
            Service {
                name: "GitHub".to_string(),
                tools: vec![
                    Tool {
                        name: "GITHUB_CREATE_ISSUE".to_string(),
                        description: "Create a new issue on GitHub".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "GITHUB_LIST_ISSUES".to_string(),
                        description: "List all issues in a repository".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "GITHUB_CREATE_PULL_REQUEST".to_string(),
                        description: "Create a new pull request".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                ],
                is_expanded: true,
                status: ConnectionStatus::NotTested,
            },
            Service {
                name: "Supabase".to_string(),
                tools: vec![
                    Tool {
                        name: "SUPABASE_INSERT_RECORD".to_string(),
                        description: "Insert a new record into database".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "SUPABASE_SELECT_RECORDS".to_string(),
                        description: "Select records from database".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                    Tool {
                        name: "SUPABASE_CREATE_TABLE".to_string(),
                        description: "Create a new table in database".to_string(),
                        status: ConnectionStatus::NotTested,
                    },
                ],
                is_expanded: true,
                status: ConnectionStatus::NotTested,
            },
        ];

        Self {
            services,
            selected_service: 0,
            selected_tool: None,
            input_text: String::new(),
            cursor_position: 0,
            is_input_mode: false,
            messages: Vec::new(),
            is_processing: false,
            show_help: false,
            service_scroll: vec![0, 0, 0], // Initialize scroll positions for 3 services
            message_scroll: 0,
            messages_expanded: Vec::new(),
        }
    }

    pub fn toggle_service_expansion(&mut self) {
        if let Some(service) = self.services.get_mut(self.selected_service) {
            service.is_expanded = !service.is_expanded;
        }
    }

    pub fn next_service(&mut self) {
        self.selected_service = (self.selected_service + 1) % self.services.len();
        self.selected_tool = None;
    }

    pub fn previous_service(&mut self) {
        self.selected_service = if self.selected_service == 0 {
            self.services.len() - 1
        } else {
            self.selected_service - 1
        };
        self.selected_tool = None;
    }

    pub fn next_tool(&mut self) {
        if let Some(service) = self.services.get(self.selected_service) {
            if service.is_expanded && !service.tools.is_empty() {
                let tool_count = service.tools.len();
                self.selected_tool = Some(
                    self.selected_tool
                        .map(|t| (t + 1) % tool_count)
                        .unwrap_or(0),
                );
            }
        }
    }

    pub fn previous_tool(&mut self) {
        if let Some(service) = self.services.get(self.selected_service) {
            if service.is_expanded && !service.tools.is_empty() {
                let tool_count = service.tools.len();
                self.selected_tool = Some(
                    self.selected_tool
                        .map(|t| if t == 0 { tool_count - 1 } else { t - 1 })
                        .unwrap_or(tool_count - 1),
                );
            }
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

    pub fn update_service_status(&mut self, service_index: usize, status: ConnectionStatus) {
        if let Some(service) = self.services.get_mut(service_index) {
            service.status = status.clone();
            for tool in &mut service.tools {
                tool.status = status.clone();
            }
        }
    }

    pub fn scroll_service_up(&mut self) {
        if self.selected_service < self.service_scroll.len() {
            let scroll = &mut self.service_scroll[self.selected_service];
            if *scroll > 0 {
                *scroll -= 1;
            }
        }
    }

    pub fn scroll_service_down(&mut self) {
        if self.selected_service < self.service_scroll.len() {
            let scroll = &mut self.service_scroll[self.selected_service];
            let service = &self.services[self.selected_service];
            if service.is_expanded && *scroll < service.tools.len().saturating_sub(5) {
                *scroll += 1;
            }
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
