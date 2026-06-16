//! Main pane: shows the currently focused action's description, an "coming
//! soon" placeholder (for the M2 placeholders), or a real-input widget
//! (added in M3).

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::actions::TestAction;
use crate::app::{App, Focus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Main;
    let border = app.theme.style_border(focused);

    let (title, body) = match app.action_metas.get(app.sidebar_index) {
        Some(meta) => {
            let title = format!(" {} ", meta.label);
            let is_real = meta.id == TestAction::ID;
            let mut lines: Vec<Line> = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", meta.hint),
                    app.theme.style_text(),
                )),
                Line::from(""),
            ];

            if is_real {
                lines.push(Line::from(Span::styled(
                    "  Press Enter to run. Press Esc to cancel a running job.",
                    app.theme.style_dim(),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  This is a dev-only test action. Real actions live in M3+.",
                    app.theme.style_dim(),
                )));
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Coming soon.",
                    app.theme.style_warn(),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  This action will be wired up in a later milestone — see",
                    app.theme.style_dim(),
                )));
                lines.push(Line::from(Span::styled(
                    "  spec.md §9 (Implementation milestones) for the plan.",
                    app.theme.style_dim(),
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Press Tab to move focus, ? for help, q to quit.",
                app.theme.style_dim(),
            )));
            (title, lines)
        }
        None => (
            " (no action) ".to_string(),
            vec![Line::from(Span::styled(
                "  Pick an action from the sidebar.",
                app.theme.style_dim(),
            ))],
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Span::styled(
            title,
            app.theme.style_border(focused).fg(app.theme.border_focused),
        ));
    let p = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}
