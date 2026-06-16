//! Main pane: shows the currently focused action's input UI.
//!
//! - `InputKind::Dir` / `InputKind::File`: per-action [`Browser`]
//! - `InputKind::Choice` (reset_ableton): version picker
//! - `InputKind::None` (patch_ozone): Run button card
//!
//! When `app.confirming` is set, the confirm prompt is overlaid on
//! top of whatever the main pane is showing.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::actions::reset_ableton::AbletonVersion;
use crate::actions::InputKind;
use crate::app::{App, Focus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Main;

    // Draw the underlying content first, then overlay the confirm prompt.
    render_underlying(app, frame, area, focused);

    if let Some(prompt) = &app.confirming {
        render_confirm(app, frame, area, prompt);
    }
}

fn render_underlying(app: &App, frame: &mut Frame, area: Rect, focused: bool) {
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

    match meta.input_kind {
        InputKind::Dir | InputKind::File => {
            if let Some(browser) = app.browsers.get(&meta.id) {
                browser.render(&app.theme, focused, area, frame);
            } else {
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
            render_choice(app, frame, area, meta, focused);
        }
        InputKind::None => {
            // patch_ozone and any other no-input action: simple card.
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

fn render_choice(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    meta: &crate::actions::ActionInfo,
    focused: bool,
) {
    let border = app.theme.style_border(focused);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Span::styled(
            format!(" {} ", meta.label),
            app.theme.style_border(focused).fg(app.theme.border_focused),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: title hint (1 row) | version list (rest) | footer hint (1 row)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("  {}", meta.hint),
            app.theme.style_text(),
        )),
        chunks[0],
    );

    let picker = app.choice_state.get(&meta.id);
    let versions: &[AbletonVersion] = match picker {
        Some(p) => &p.versions,
        None => &[],
    };
    if versions.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  No installed Ableton versions found.\n  (Looked in ~/Library/Preferences/Ableton/*/Preferences.cfg)",
                app.theme.style_warn(),
            ))
            .wrap(Wrap { trim: false }),
            chunks[1],
        );
    } else {
        let cursor = picker.map(|p| p.cursor).unwrap_or(0);
        let items: Vec<ListItem> = versions
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let is_cursor = i == cursor;
                let mtime = v
                    .cfg_mtime
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        let secs = d.as_secs();
                        let days = secs / 86400;
                        if days < 1 {
                            "today".to_string()
                        } else if days < 30 {
                            format!("{}d ago", days)
                        } else {
                            format!("{}mo ago", days / 30)
                        }
                    })
                    .unwrap_or_else(|| "—".into());
                let marker = if is_cursor { "▶" } else { " " };
                let name_style = if is_cursor {
                    app.theme.style_highlight()
                } else {
                    app.theme.style_text()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", marker), app.theme.style_dim()),
                    Span::styled(format!("{:<14}", v.version), name_style),
                    Span::styled(format!("  prefs: {:<10}", mtime), app.theme.style_dim()),
                    Span::styled(
                        format!("  undo: {:>6}", human_size(v.undo_size)),
                        app.theme.style_dim(),
                    ),
                ]))
            })
            .collect();
        let mut state = ListState::default();
        state.select(Some(cursor));
        let list = List::new(items)
            .highlight_style(app.theme.style_highlight())
            .highlight_symbol("");
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }

    let hint = if versions.is_empty() {
        "  (no versions to pick)"
    } else {
        "  [j/k] move  [Enter] select & confirm  [r] refresh"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(hint, app.theme.style_dim())),
        chunks[2],
    );
}

fn render_confirm(app: &App, frame: &mut Frame, area: Rect, prompt: &crate::app::ConfirmPrompt) {
    let popup = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup);

    let border_style = if prompt.danger {
        app.theme.style_danger()
    } else {
        app.theme.style_border(true)
    };
    let title_style = if prompt.danger {
        app.theme.style_danger()
    } else {
        app.theme.style_border(true).fg(app.theme.border_focused)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(format!(" {} ", prompt.title), title_style));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = vec![Line::from("")];
    if prompt.danger {
        lines.push(Line::from(Span::styled(
            "  ⚠ DANGER — this modifies files outside your home directory.",
            app.theme.style_danger(),
        )));
        lines.push(Line::from(""));
    }
    for line in prompt.detail.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", line),
            app.theme.style_text(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to run, Esc to cancel.",
        app.theme.style_warn(),
    )));

    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);
    frame.render_widget(p, inner);
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

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1}{}", size, UNITS[unit])
    }
}
