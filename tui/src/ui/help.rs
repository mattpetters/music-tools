//! Help overlay. Modal: opens with `?`, closes with `?` / `Esc` / `q` /
//! `Ctrl-C`. Lists every keybinding from spec §5.6, marking which ones are
//! available in M1 vs. coming in later milestones.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(app.theme.overlay_border))
        .title(Span::styled(
            " Help — press Esc to close ",
            ratatui::style::Style::default()
                .fg(app.theme.overlay_title)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let text = vec![
        section("Navigation"),
        key("j / ↓", "Move down in sidebar"),
        key("k / ↑", "Move up in sidebar"),
        key("g / G", "Top / bottom of sidebar"),
        key("1-6", "Jump directly to action N"),
        key("Tab / Shift-Tab", "Cycle focus: sidebar → main → log"),
        blank(),
        section("Selection (M3+)"),
        key("Space", "Toggle selection in multi-select inputs"),
        key("a / n / i", "Select all / none / invert"),
        key("c", "Clear selection"),
        blank(),
        section("Browser (M3+)"),
        key("Enter", "Run the focused action"),
        key("/", "Focus path field"),
        key("~", "Jump to $HOME"),
        key("@", "Recent locations palette"),
        key("s", "Cycle sort (name / size / mtime)"),
        key(".", "Toggle hidden files"),
        key("r", "Refresh"),
        blank(),
        section("Jobs (M3+)"),
        key("Esc", "Cancel a running job (M7)"),
        key("h", "Job history (M7)"),
        blank(),
        section("View"),
        key("?", "Toggle this help"),
        key("Ctrl-L", "Clear the log pane"),
        key("y", "Yank visible log to clipboard (M3)"),
        key("Ctrl-Up / Down", "Resize log pane"),
        blank(),
        section("Global"),
        key("q / Ctrl-C", "Quit"),
    ];

    let p = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(p, inner);
}

fn section(name: &str) -> Line<'static> {
    Line::from(Span::styled(
        name.to_string(),
        ratatui::style::Style::default()
            .fg(ratatui::style::Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key<'a>(k: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<22}", k),
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        ),
        Span::raw(desc),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
