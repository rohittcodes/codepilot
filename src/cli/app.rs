use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use std::time::Instant;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::cli::{state::AppState, ui};
use crate::{agents::*, config::*, MultiAgentOrchestrator, ResponseFormatter};

pub struct App {
    pub state: AppState,
    pub config: Config,
    pub should_quit: bool,
    pub last_ctrl_c: Option<Instant>,
}

impl App {
    pub fn new() -> Result<Self> {
        let state = AppState::new();
        let config = Config::from_env()?;
        
        Ok(Self { 
            state,
            config,
            should_quit: false,
            last_ctrl_c: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Clear screen and enter alternate screen
        let mut stdout = io::stdout();
        execute!(stdout, Clear(ClearType::All), EnterAlternateScreen)?;
        
        // Enable raw mode
        enable_raw_mode()?;
        
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            Clear(ClearType::All)
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("Error: {:?}", err);
        }

        Ok(())
    }

    async fn run_app<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        // Initialize agents and test connections
        self.initialize_agents().await?;

        loop {
            terminal.draw(|f| ui::render(f, &self.state))?;

            if let Event::Key(key) = event::read()? {
                // Handle Ctrl+C for graceful exit
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    let now = Instant::now();
                    if let Some(last_ctrl_c) = self.last_ctrl_c {
                        if now.duration_since(last_ctrl_c).as_secs() < 2 {
                            self.state.add_message("Exiting...".to_string());
                            return Ok(());
                        }
                    }
                    self.last_ctrl_c = Some(now);
                    self.state.add_message("Press Ctrl+C again within 2 seconds to exit".to_string());
                } else {
                    if self.state.is_input_mode {
                        // Input mode - only handle typing and basic input controls
                        match key.code {
                            KeyCode::Esc => {
                                self.state.is_input_mode = false;
                            }
                            KeyCode::Enter => {
                                if !self.state.input_text.is_empty() {
                                    self.process_user_input().await;
                                    self.state.is_input_mode = false;
                                }
                            }
                            KeyCode::Char(c) => {
                                self.state.input_text.push(c);
                                self.state.cursor_position = self.state.input_text.len();
                            }
                            KeyCode::Backspace => {
                                if !self.state.input_text.is_empty() {
                                    self.state.input_text.pop();
                                    self.state.cursor_position = self.state.cursor_position.saturating_sub(1);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // Navigation mode - handle all navigation shortcuts
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(());
                            }
                            KeyCode::Char('i') => {
                                self.state.is_input_mode = true;
                            }
                            KeyCode::Char('h') => {
                                self.state.show_help = !self.state.show_help;
                            }
                            KeyCode::Esc => {
                                if self.state.show_help {
                                    self.state.show_help = false;
                                } else {
                                    return Ok(());
                                }
                            }
                            KeyCode::Up => {
                                if let Some(service) = self.state.services.get(self.state.selected_service) {
                                    if service.is_expanded && !service.tools.is_empty() {
                                        // Scroll within the service
                                        self.state.scroll_service_up();
                                    } else {
                                        // Navigate between services
                                        self.state.previous_service();
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let Some(service) = self.state.services.get(self.state.selected_service) {
                                    if service.is_expanded && !service.tools.is_empty() {
                                        // Scroll within the service
                                        self.state.scroll_service_down();
                                    } else {
                                        // Navigate between services
                                        self.state.next_service();
                                    }
                                }
                            }
                            KeyCode::Tab => {
                                self.state.next_service();
                            }
                            KeyCode::Left => {
                                self.state.previous_tool();
                            }
                            KeyCode::Right => {
                                self.state.next_tool();
                            }
                            KeyCode::Char(' ') => {
                                self.state.toggle_service_expansion();
                            }
                            KeyCode::PageUp => {
                                self.state.scroll_messages_up();
                            }
                            KeyCode::PageDown => {
                                self.state.scroll_messages_down();
                            }
                            KeyCode::Char('j') => {
                                self.state.scroll_messages_down();
                            }
                            KeyCode::Char('k') => {
                                self.state.scroll_messages_up();
                            }
                            KeyCode::Home => {
                                self.state.message_scroll = 0;
                            }
                            KeyCode::End => {
                                let max_scroll = self.state.messages_expanded.len().saturating_sub(10);
                                self.state.message_scroll = max_scroll;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    async fn initialize_agents(&mut self) -> Result<()> {
        self.state.add_message("Initializing agents and testing connections...".to_string());

        // Test Linear connection
        self.test_linear_connection().await;
        
        // Test GitHub connection  
        self.test_github_connection().await;
        
        // Test Supabase connection
        self.test_supabase_connection().await;

        self.state.add_message("Initialization complete!".to_string());
        Ok(())
    }

    async fn test_linear_connection(&mut self) {
        self.state.update_service_status(0, crate::cli::state::ConnectionStatus::Pending);
        
        match LinearAgent::new(
            get_openai_api_key().unwrap_or_default(),
            &self.config,
        ).await {
            Ok(agent) => {
                self.state.update_service_status(0, crate::cli::state::ConnectionStatus::Connected);
                self.state.add_message("Linear agent connected successfully".to_string());
                
                // Fetch and update tools
                let operations = agent.get_available_operations();
                let tools: Vec<crate::cli::state::Tool> = operations.iter().map(|op| {
                    crate::cli::state::Tool {
                        name: op.clone(),
                        description: format!("Linear operation: {}", op),
                        status: crate::cli::state::ConnectionStatus::Connected,
                    }
                }).collect();
                self.state.services[0].tools = tools;
                self.state.add_message(format!("Loaded {} Linear tools", operations.len()));
            }
            Err(e) => {
                self.state.update_service_status(0, crate::cli::state::ConnectionStatus::Failed(e.to_string()));
                self.state.add_message(format!("Linear agent failed: {}", e));
            }
        }
    }

    async fn test_github_connection(&mut self) {
        self.state.update_service_status(1, crate::cli::state::ConnectionStatus::Pending);
        
        match GitHubAgent::new(
            get_openai_api_key().unwrap_or_default(),
            &self.config,
        ).await {
            Ok(agent) => {
                self.state.update_service_status(1, crate::cli::state::ConnectionStatus::Connected);
                self.state.add_message("GitHub agent connected successfully".to_string());
                
                // Fetch and update tools
                let operations = agent.get_available_operations();
                let tools: Vec<crate::cli::state::Tool> = operations.iter().map(|op| {
                    crate::cli::state::Tool {
                        name: op.clone(),
                        description: format!("GitHub operation: {}", op),
                        status: crate::cli::state::ConnectionStatus::Connected,
                    }
                }).collect();
                self.state.services[1].tools = tools;
                self.state.add_message(format!("Loaded {} GitHub tools", operations.len()));
            }
            Err(e) => {
                self.state.update_service_status(1, crate::cli::state::ConnectionStatus::Failed(e.to_string()));
                self.state.add_message(format!("GitHub agent failed: {}", e));
            }
        }
    }

    async fn test_supabase_connection(&mut self) {
        self.state.update_service_status(2, crate::cli::state::ConnectionStatus::Pending);
        
        match SupabaseAgent::new(
            get_openai_api_key().unwrap_or_default(),
            &self.config,
        ).await {
            Ok(agent) => {
                self.state.update_service_status(2, crate::cli::state::ConnectionStatus::Connected);
                self.state.add_message("Supabase agent connected successfully".to_string());
                
                // Fetch and update tools
                let operations = agent.get_available_operations();
                let tools: Vec<crate::cli::state::Tool> = operations.iter().map(|op| {
                    crate::cli::state::Tool {
                        name: op.clone(),
                        description: format!("Supabase operation: {}", op),
                        status: crate::cli::state::ConnectionStatus::Connected,
                    }
                }).collect();
                self.state.services[2].tools = tools;
                self.state.add_message(format!("Loaded {} Supabase tools", operations.len()));
            }
            Err(e) => {
                self.state.update_service_status(2, crate::cli::state::ConnectionStatus::Failed(e.to_string()));
                self.state.add_message(format!("Supabase agent failed: {}", e));
            }
        }
    }

    async fn process_user_input(&mut self) {
        let query = self.state.input_text.clone();
        self.state.input_text.clear();
        self.state.cursor_position = 0;
        self.state.is_input_mode = false;
        self.state.is_processing = true;

        self.state.add_message(format!("Processing: {}", query));
        
        // Update messages with proper wrapping
        self.update_messages_display();

        // Use the orchestrator for all queries
        let result = self.process_orchestrator_query(&query).await;
        let formatter = ResponseFormatter::new();

        match result {
            Ok(response) => {
                let formatted_response = if response.contains("USE_LINEAR_AGENT") || 
                                           response.contains("USE_GITHUB_AGENT") || 
                                           response.contains("USE_SUPABASE_AGENT") {
                    // This is from an agent
                    let agent_name = if response.contains("Linear") { "linear" }
                                   else if response.contains("GitHub") { "github" }
                                   else if response.contains("Supabase") { "supabase" }
                                   else { "agent" };
                    formatter.format_agent_response(agent_name, &response)
                } else {
                    formatter.format_agent_response("orchestrator", &response)
                };
                
                self.state.add_message(formatted_response);
            }
            Err(e) => {
                let error_msg = e.to_string();
                let formatted_error = if error_msg.contains("timed out") {
                    "LLM request timed out. Try asking a simpler question or try again later.".to_string()
                } else if error_msg.contains("rate limit") {
                    "LLM rate limit reached. Please wait a moment and try again.".to_string()
                } else {
                    formatter.format_error(&error_msg)
                };
                self.state.add_message(formatted_error);
            }
        }

        // Update messages display after adding new messages
        self.update_messages_display();
        self.state.is_processing = false;
    }

    fn update_messages_display(&mut self) {
        // Get terminal width for proper text wrapping
        // For now use a reasonable default, could be improved to get actual terminal size
        let width = 100; // Increased width for better readability
        self.state.update_messages_expanded(width);
    }


    async fn process_orchestrator_query(&self, query: &str) -> Result<String> {
        let orchestrator = MultiAgentOrchestrator::new().await?;
        let routing_response = orchestrator.process_query(query).await?;
        
        // Check if the orchestrator is routing to a specific agent
        if routing_response.contains("USE_LINEAR_AGENT") {
            let mut agent = LinearAgent::new(get_openai_api_key()?, &self.config).await?;
            agent.process_query(query).await
        } else if routing_response.contains("USE_GITHUB_AGENT") {
            let mut agent = GitHubAgent::new(get_openai_api_key()?, &self.config).await?;
            agent.process_query(query).await
        } else if routing_response.contains("USE_SUPABASE_AGENT") {
            let mut agent = SupabaseAgent::new(get_openai_api_key()?, &self.config).await?;
            agent.process_query(query).await
        } else {
            // For general queries, just return the orchestrator's response
            Ok(routing_response)
        }
    }
} 