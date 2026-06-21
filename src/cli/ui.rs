use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};
use crate::cli::state::AppState;

// Tokyo-night-ish accent palette.
const ACCENT: Color = Color::Rgb(122, 162, 247); // blue
const ACCENT_2: Color = Color::Rgb(187, 154, 247); // purple
const OK: Color = Color::Rgb(158, 206, 106); // green
const WARN: Color = Color::Rgb(224, 175, 104); // orange
const ERR: Color = Color::Rgb(247, 118, 142); // red
const MUTED: Color = Color::Rgb(86, 95, 137); // dim gray-blue
const FG: Color = Color::Rgb(192, 202, 245); // soft white

pub fn render(f: &mut Frame, app: &AppState) {
    if app.show_details {
        render_details(f, app);
        return;
    }
    if app.show_help {
        render_help(f);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(6),
                Constraint::Min(6),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    render_title(f, chunks[0]);
    render_input(f, chunks[1], app);
    render_messages(f, chunks[2], app);
    render_status_bar(f, chunks[3], app);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled("›› ", Style::default().fg(ACCENT_2).add_modifier(Modifier::BOLD)),
        Span::styled(
            "CodePilot",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" — JS/TS coding agent", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(MUTED)),
    );

    f.render_widget(title, area);
}

fn render_input(f: &mut Frame, area: Rect, app: &AppState) {
    let (border_color, label) = if app.is_input_mode {
        (ACCENT_2, " Describe a task · Enter to run · Shift+Enter for newline · Esc to cancel ")
    } else if app.is_processing {
        (WARN, " Working… ")
    } else {
        (MUTED, " Press 'i' to describe a code task ")
    };

    // Content rows available inside the border + horizontal padding.
    let visible_rows = area.height.saturating_sub(2);

    let before_cursor = &app.input_text[..app.cursor_position.min(app.input_text.len())];
    let cursor_row = before_cursor.matches('\n').count() as u16;
    let cursor_col = before_cursor.rsplit('\n').next().unwrap_or("").chars().count() as u16;

    // Auto-scroll so the cursor's line is always inside the box, instead of
    // letting it run off the bottom — past which the terminal cursor would
    // land outside this widget's area entirely (ratatui clips rendered
    // content to a widget's Rect, but not the separate global terminal
    // cursor position, so an unclamped row visually overlaps whatever is
    // drawn below, e.g. the Activity panel).
    let scroll_row = cursor_row.saturating_sub(visible_rows.saturating_sub(1));

    let input = Paragraph::new(app.input_text.clone())
        .style(Style::default().fg(FG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1))
                .title(Span::styled(label, Style::default().fg(border_color).add_modifier(Modifier::BOLD)))
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true })
        .scroll((scroll_row, 0));

    f.render_widget(input, area);

    if app.is_input_mode {
        let cursor_row_in_box = (cursor_row - scroll_row).min(visible_rows.saturating_sub(1));
        let cursor_col = cursor_col.min(area.width.saturating_sub(4));
        f.set_cursor_position((area.x + cursor_col + 2, area.y + 1 + cursor_row_in_box));
    }
}

fn render_messages(f: &mut Frame, area: Rect, app: &AppState) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Get the messages to display based on scroll position
    let start_idx = app.message_scroll;
    let end_idx = (start_idx + visible_height).min(app.messages_expanded.len());

    let visible_messages: Vec<ListItem> = if app.messages_expanded.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No activity yet — press 'i' and describe a code task.",
            Style::default().fg(MUTED),
        )))]
    } else {
        app.messages_expanded[start_idx..end_idx]
            .iter()
            .map(|msg| {
                let (icon, color) = if msg.contains("Error") || msg.contains("Failed") {
                    ("✗ ", ERR)
                } else if msg.contains("Success") || msg.contains("Wrote") {
                    ("✓ ", OK)
                } else if msg.contains("Processing") || msg.contains("Working") {
                    ("⚙ ", WARN)
                } else {
                    ("· ", FG)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(icon, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(msg.clone(), Style::default().fg(color)),
                ]))
            })
            .collect()
    };

    let mut title = " Activity ".to_string();
    if app.messages_expanded.len() > visible_height {
        let total_lines = app.messages_expanded.len();
        let current_line = start_idx + 1;
        let end_line = end_idx;
        let mut indicators = String::new();
        if start_idx > 0 {
            indicators.push('↑');
        }
        if end_idx < total_lines {
            indicators.push('↓');
        }
        title = format!(" Activity ({}-{}/{}) {} ", current_line, end_line, total_lines, indicators);
    }

    let messages_list = List::new(visible_messages).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(1))
            .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(MUTED)),
    );

    f.render_widget(messages_list, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &AppState) {
    let (mode_label, mode_color) = if app.is_input_mode {
        (" INSERT ", ACCENT_2)
    } else {
        (" NORMAL ", ACCENT)
    };

    let repo = if app.target_repo_path.is_empty() {
        ".".to_string()
    } else {
        app.target_repo_path.clone()
    };

    let line = Line::from(vec![
        Span::styled(mode_label, Style::default().fg(Color::Black).bg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  repo: {repo}  "), Style::default().fg(MUTED)),
        Span::styled("·  'h' help  Ctrl+O details  'q' quit", Style::default().fg(MUTED)),
    ]);

    f.render_widget(Paragraph::new(line), area);
}

fn section(s: &str) -> Line<'static> {
    Line::from(Span::styled(s.to_string(), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

fn key(k: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<12}", k), Style::default().fg(ACCENT_2).add_modifier(Modifier::BOLD)),
        Span::styled(desc.to_string(), Style::default().fg(FG)),
    ])
}

fn render_help(f: &mut Frame) {
    let lines = vec![
        Line::from(Span::styled(
            "›› CodePilot — JS/TS coding agent",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        section("Task input"),
        key("i", "describe a code task"),
        key("Enter", "submit the task (input mode)"),
        key("Shift+Enter", "insert a newline instead of submitting"),
        Line::from(""),
        section("Navigation"),
        key("h", "toggle this help screen"),
        key("Ctrl+O", "view edit detail (j/k or PageUp/Dn to browse history)"),
        key("Esc", "exit current mode or quit"),
        key("Ctrl+C", "press twice quickly to exit"),
        Line::from(""),
        section("Scrolling"),
        key("j / k", "scroll messages down / up"),
        key("PageUp/Dn", "scroll faster"),
        key("Home/End", "jump to top / bottom"),
        Line::from(""),
        Line::from(Span::styled("Press 'h' or 'Esc' to return", Style::default().fg(MUTED))),
    ];

    let help_paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::uniform(1))
                .title(Span::styled(" Help ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .border_style(Style::default().fg(ACCENT_2)),
        )
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, f.area());
}

fn detail_field(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(ACCENT_2).add_modifier(Modifier::BOLD)),
        Span::styled(value, Style::default().fg(FG)),
    ])
}

fn render_details(f: &mut Frame, app: &AppState) {
    let footer = if app.edit_history.len() > 1 {
        format!(
            " {}/{} · j/k or PageUp/Dn to browse · Ctrl+O or Esc to close ",
            app.detail_cursor + 1,
            app.edit_history.len()
        )
    } else {
        " Ctrl+O or Esc to close ".to_string()
    };

    let lines: Vec<Line> = match app.edit_history.get(app.detail_cursor) {
        Some(detail) => {
            let (status_text, status_color) = if detail.applied {
                ("Applied", OK)
            } else {
                ("Rejected", ERR)
            };

            let mut lines = vec![
                detail_field("Task", detail.task.clone()),
                detail_field("File", detail.path.display().to_string()),
                detail_field("Size", format!("{} bytes", detail.bytes)),
                detail_field("Time", detail.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(ACCENT_2).add_modifier(Modifier::BOLD)),
                    Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                ]),
            ];
            if let Some(verification) = &detail.verification {
                lines.push(detail_field("Verification", verification.clone()));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Content:", Style::default().fg(ACCENT_2).add_modifier(Modifier::BOLD))));
            lines.extend(detail.content.lines().map(|l| Line::from(l.to_string())));
            lines
        }
        None => vec![Line::from(Span::styled(
            "No edits yet — run a task and its full file path + content will show up here.",
            Style::default().fg(MUTED),
        ))],
    };

    let body = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(FG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::uniform(1))
                .title(Span::styled(" Last Edit ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
                .title_bottom(Span::styled(footer, Style::default().fg(MUTED)))
                .border_style(Style::default().fg(ACCENT_2)),
        );

    f.render_widget(body, f.area());
}
