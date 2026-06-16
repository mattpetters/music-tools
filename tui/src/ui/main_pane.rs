//! Main pane: shows the currently focused action's input UI.
//!
//! For actions with `InputKind::None` (no input), shows a hint card.
//! For actions with `InputKind::Dir` or `InputKind::File`, renders
//! the per-action [`Browser`].
//! For `InputKind::Choice` (M4+), renders a placeholder until the
//! version picker is implemented.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::actions::InputKind;
use crate::app::{App, Focus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Main;
    let border = app.theme.style_border(focused);

    let Some(meta) = app.action_metas.get(app.sidebar_index) else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(Span::styled(
                " (no action) ",
                app.theme.style_border(focused).fg(app.theme.border_focused),
            ));
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  Pick an action from the sidebar.",
                app.theme.style_dim(),
            ))
            .block(block),
            area,
        );
        return;
    };

    // For actions with a browser, render the browser INSIDE the main pane
    // area. The browser draws its own block + title; we just give it the
    // area and let it own the frame.
    match meta.input_kind {
        InputKind::Dir | InputKind::File => {
            if let Some(browser) = app.browsers.get(&meta.id) {
                browser.render(&app.theme, focused, area, frame);
            } else {
                // No browser yet — render a placeholder that explains
                // what's about to happen.
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(border)
                    .title(Span::styled(
                        format!(" {} ", meta.label),
                        app.theme.style_border(focused).fg(app.theme.border_focused),
                    ));
                let body = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  {}", meta.hint),
                        app.theme.style_text(),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press Tab to focus this pane, then '/' to type a path.",
                        app.theme.style_dim(),
                    )),
                ];
                let p = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
                frame.render_widget(p, area);
            }
        }
        InputKind::Choice => {
            // M4: version picker for reset_ableton.
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border)
                .title(Span::styled(
                    format!(" {} ", meta.label),
                    app.theme.style_border(focused).fg(app.theme.border_focused),
                ));
            let body = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", meta.hint),
                    app.theme.style_text(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Version picker lands in M4. Stay tuned.",
                    app.theme.style_warn(),
                )),
            ];
            let p = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
            frame.render_widget(p, area);
        }
        InputKind::None => {
            // patch_ozone and TestAction: just a Run button + hint.
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border)
                .title(Span::styled(
                    format!(" {} ", meta.label),
                    app.theme.style_border(focused).fg(app.theme.border_focused),
                ));
            let body = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", meta.hint),
                    app.theme.style_text(),
                )),
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to run. Press Esc to cancel a running job.",
                    app.theme.style_dim(),
                )),
            ];
            let p = Paragraph::new(body).block(block).wrap(Wrap { trim: false });
            frame.render_widget(p, area);
        }
    }
}
