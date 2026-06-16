//! Main pane: shows the currently focused action's description, an "coming
//! soon" placeholder, and a brief contextual hint. M3+ replaces this with
//! the file browser / picker widgets.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::ui::sidebar;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Main;
    let border = app.theme.style_border(focused);

    let (title, body) = match sidebar::ACTION_DEFS.get(app.sidebar_index) {
        Some((_cat, _id, label, hint)) => {
            let title = format!(" {} ", label);
            let body = vec![
                Line::from(""),
                Line::from(Span::styled(format!("  {}", hint), app.theme.style_text())),
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled("  Coming soon.", app.theme.style_warn())),
                Line::from(""),
                Line::from(Span::styled(
                    "  This action will be wired up in a later milestone — see",
                    app.theme.style_dim(),
                )),
                Line::from(Span::styled(
                    "  spec.md §9 (Implementation milestones) for the plan.",
                    app.theme.style_dim(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to log a \"would run\" message, Tab to move",
                    app.theme.style_dim(),
                )),
                Line::from(Span::styled(
                    "  focus, ? for help, q to quit.",
                    app.theme.style_dim(),
                )),
            ];
            (title, body)
        }
        None => {
            let title = " (no action) ".to_string();
            let body = vec![Line::from(Span::styled(
                "  Pick an action from the sidebar.",
                app.theme.style_dim(),
            ))];
            (title, body)
        }
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
