use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::cli::state::{AppState, ConnectionStatus};

pub fn render(f: &mut Frame, app: &AppState) {
    if app.show_help {
        render_help(f);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Min(8),
            ]
            .as_ref(),
        )
        .split(f.size());

    render_title(f, chunks[0]);
    render_services(f, chunks[1], app);
    render_input(f, chunks[2], app);
    render_messages(f, chunks[3], app);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("🚀 CodePilot - Multi-Agent MCP System")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    
    let instructions = Paragraph::new("Enter to confirm - Esc to exit")
        .style(Style::default().fg(Color::Gray))
        .alignment(ratatui::layout::Alignment::Left);
    
    let title_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(area);
    
    f.render_widget(title, title_chunks[0]);
    f.render_widget(instructions, title_chunks[1]);
}

fn render_services(f: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)].as_ref())
        .split(area);

    for (i, service) in app.services.iter().enumerate() {
        let is_selected = i == app.selected_service;
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let status_icon = match service.status {
            ConnectionStatus::Connected => "🟢",
            ConnectionStatus::Failed(_) => "🔴",
            ConnectionStatus::Pending => "🟡",
            ConnectionStatus::NotTested => "⚪",
        };

        let title = format!("{} {} {}", status_icon, service.name, if service.is_expanded { "▼" } else { "▶" });
        
        let mut items = vec![ListItem::new(title).style(style)];
        
        if service.is_expanded {
            let scroll_pos = app.service_scroll.get(i).copied().unwrap_or(0);
            let visible_tools = service.tools.iter().skip(scroll_pos).take(8); // Show max 8 tools
            
            for (j, tool) in visible_tools.enumerate() {
                let actual_index = scroll_pos + j;
                let tool_is_selected = app.selected_tool == Some(actual_index) && i == app.selected_service;
                let tool_style = if tool_is_selected {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let tool_status_icon = match tool.status {
                    ConnectionStatus::Connected => "🟢",
                    ConnectionStatus::Failed(_) => "🔴",
                    ConnectionStatus::Pending => "🟡",
                    ConnectionStatus::NotTested => "⚪",
                };

                // Truncate long tool names for better display
                let tool_name = if tool.name.len() > 18 {
                    format!("{}...", &tool.name[..18])
                } else {
                    tool.name.clone()
                };

                let tool_text = format!("  {} {}", tool_status_icon, tool_name);
                items.push(ListItem::new(tool_text).style(tool_style));
            }
            
            // Add scroll indicator if there are more tools
            if service.tools.len() > 8 {
                let scroll_indicator = if scroll_pos > 0 && scroll_pos + 8 < service.tools.len() {
                    "  ↕️  More tools..."
                } else if scroll_pos > 0 {
                    "  ↑  End of list"
                } else {
                    "  ↓  Scroll for more"
                };
                items.push(ListItem::new(scroll_indicator).style(Style::default().fg(Color::Yellow)));
            }
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!(" {} ", service.name.clone())))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(list, chunks[i]);
    }
}

fn render_input(f: &mut Frame, area: Rect, app: &AppState) {
    let input_prompt = if app.is_input_mode {
        "Input Mode - Type your question (Esc to exit, Enter to submit)"
    } else {
        "Ask your question (Press 'i' to enter input mode)"
    };
    
    let input = Paragraph::new(app.input_text.clone())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_prompt)
                .style(if app.is_input_mode {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                }),
        )
        .wrap(Wrap { trim: true });
    
    // Add cursor position
    if app.is_input_mode {
        f.set_cursor(
            area.x + 1 + app.cursor_position as u16,
            area.y + 1,
        );
    }

    f.render_widget(input, area);

    // Render cursor
    if app.is_input_mode {
        f.set_cursor(
            area.x + app.cursor_position as u16 + 1,
            area.y + 1,
        );
    }
}

fn render_messages(f: &mut Frame, area: Rect, app: &AppState) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    
    // Get the messages to display based on scroll position
    let start_idx = app.message_scroll;
    let end_idx = (start_idx + visible_height).min(app.messages_expanded.len());
    
    let visible_messages: Vec<ListItem> = if app.messages_expanded.is_empty() {
        if app.messages.is_empty() {
            vec![ListItem::new("No messages yet. Start by asking a question!".to_string())
                .style(Style::default().fg(Color::Gray))]
        } else {
            app.messages
                .iter()
                .map(|msg| ListItem::new(msg.clone()))
                .collect()
        }
    } else {
        app.messages_expanded[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(_, msg)| {
                let style = if msg.contains("Error") || msg.contains("Failed") {
                    Style::default().fg(Color::Red)
                } else if msg.contains("Success") {
                    Style::default().fg(Color::Green)
                } else if msg.contains("Processing") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(msg.clone()).style(style)
            })
            .collect()
    };

    // Create scroll indicators
    let mut title = "Messages".to_string();
    let mut scroll_indicators = String::new();
    
    if app.messages_expanded.len() > visible_height {
        let total_lines = app.messages_expanded.len();
        let current_line = start_idx + 1;
        let end_line = end_idx;
        title = format!("Messages ({}-{}/{})", current_line, end_line, total_lines);
        
        // Add scroll indicators
        if start_idx > 0 {
            scroll_indicators.push_str("↑ ");
        }
        if end_idx < total_lines {
            scroll_indicators.push_str("↓ ");
        }
        
        if !scroll_indicators.is_empty() {
            title = format!("{} {}", title, scroll_indicators);
        }
    }

    let messages_list = List::new(visible_messages)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Cyan)))
        .style(Style::default().fg(Color::White));

    f.render_widget(messages_list, area);
}

fn render_help(f: &mut Frame) {
    let help_text = vec![
        "🚀 CodePilot - Multi-Agent MCP System",
        "",
        "Keyboard Navigation:",
        "  ↑/↓/Tab  - Navigate between services",
        "  ←/→      - Navigate between tools",
        "  Space    - Toggle service expansion",
        "  i        - Toggle input mode",
        "  h        - Toggle this help screen",
        "  Esc      - Exit current mode or quit",
        "  Ctrl+C   - Press twice quickly to exit",
        "",
        "Message Scrolling:",
        "  j/k       - Scroll messages down/up",
        "  PageUp/Down - Scroll messages faster",
        "  Home/End  - Jump to top/bottom of messages",
        "",
        "Services:",
        "  🟢 Linear   - Project management",
        "  🟢 GitHub   - Repository management", 
        "  🟢 Supabase - Database operations",
        "",
        "Press 'h' or 'Esc' to return to main view",
    ];

    let help_paragraph = Paragraph::new(help_text.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("📖 Help"))
        .alignment(ratatui::layout::Alignment::Left);

    f.render_widget(help_paragraph, f.size());
} 