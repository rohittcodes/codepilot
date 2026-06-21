use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use std::time::Instant;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::cli::{persistence, state::AppState, ui};
use crate::config::Config;
use crate::orchestrator::CodeTaskOrchestrator;
use crate::formatter::ResponseFormatter;

pub struct App {
    pub state: AppState,
    pub config: Config,
    pub should_quit: bool,
    pub last_ctrl_c: Option<Instant>,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut state = AppState::new();
        let config = Config::from_env()?;
        state.target_repo_path = config.target_repo_path.clone();

        if let Some(save_state_dir) = &config.save_state_dir {
            state.edit_history = persistence::load_entries(save_state_dir);
            state.detail_cursor = state.edit_history.len().saturating_sub(1);
        }

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
        self.state.add_message(format!(
            "Ready. Target repo: {}",
            self.config.target_repo_path
        ));

        loop {
            terminal.draw(|f| ui::render(f, &self.state))?;

            if let Event::Key(key) = event::read()? {
                // crossterm on Windows reports both press and release for a single
                // key tap; only act on press, or every key would fire twice.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

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
                } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
                    // Ctrl+O toggles the detail view for the most recent edit, from either mode.
                    self.state.show_details = !self.state.show_details;
                } else if self.state.is_input_mode {
                    // Input mode - only handle typing and basic input controls
                    match key.code {
                        KeyCode::Esc => {
                            self.state.is_input_mode = false;
                        }
                        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            self.state.input_text.push('\n');
                            self.state.cursor_position = self.state.input_text.len();
                        }
                        KeyCode::Enter => {
                            if !self.state.input_text.is_empty() {
                                self.state.is_input_mode = false;
                                self.state.is_processing = true;
                                // Redraw now so "Working..." actually shows before the
                                // blocking LLM call below, instead of freezing on the
                                // last frame until it returns.
                                terminal.draw(|f| ui::render(f, &self.state))?;
                                self.process_user_input().await;
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
                } else if self.state.show_details {
                    // Detail overlay - closing and paging through edit history.
                    match key.code {
                        KeyCode::Esc => {
                            self.state.show_details = false;
                        }
                        KeyCode::PageUp | KeyCode::Char('k') => {
                            self.state.show_older_detail();
                        }
                        KeyCode::PageDown | KeyCode::Char('j') => {
                            self.state.show_newer_detail();
                        }
                        _ => {}
                    }
                } else {
                    // Navigation mode
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

    async fn process_user_input(&mut self) {
        let task = self.state.input_text.clone();
        self.state.input_text.clear();
        self.state.cursor_position = 0;

        self.state.add_message(format!("Processing task: {}", task));
        self.update_messages_display();

        let formatter = ResponseFormatter::new();
        match self.run_code_task(&task).await {
            Ok(result) => {
                let verification_text = match &result.verification {
                    crate::runs::RunStatus::Succeeded => "tsc: passed".to_string(),
                    crate::runs::RunStatus::Failed(err) => format!("tsc: failed - {err}"),
                };

                if result.applied {
                    let summary = format!(
                        "Wrote {} ({} bytes) — {}",
                        result.target_path.display(),
                        result.edit.content.len(),
                        verification_text
                    );
                    self.state.add_message(formatter.format_success(&summary));
                } else {
                    let summary = format!(
                        "Rejected edit to {} — {}",
                        result.target_path.display(),
                        verification_text
                    );
                    self.state.add_message(formatter.format_error(&summary));
                }

                let detail = crate::cli::state::EditDetail {
                    task: task.clone(),
                    path: result.target_path.clone(),
                    content: result.edit.content.clone(),
                    bytes: result.edit.content.len(),
                    timestamp: chrono::Utc::now(),
                    applied: result.applied,
                    verification: Some(verification_text),
                };
                if let Some(save_state_dir) = &self.config.save_state_dir {
                    if let Err(e) = persistence::append_entry(save_state_dir, &detail) {
                        self.state.add_message(formatter.format_error(&format!(
                            "Could not save history: {e}"
                        )));
                    }
                }
                self.state.push_edit_detail(detail);
            }
            Err(e) => {
                self.state.add_message(formatter.format_error(&e.to_string()));
            }
        }

        self.update_messages_display();
        self.state.is_processing = false;
    }

    fn update_messages_display(&mut self) {
        let width = 100;
        self.state.update_messages_expanded(width);
    }

    async fn run_code_task(&self, task: &str) -> Result<crate::orchestrator::TaskResult> {
        let mut orchestrator = CodeTaskOrchestrator::new(&self.config).await?;
        orchestrator.run_task(task).await
    }
}
