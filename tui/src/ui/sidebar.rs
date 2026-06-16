//! Action sidebar. Renders a vertical list grouped by category, with the
//! current selection highlighted. Reads metadata from
//! [`crate::actions::metadata`] at startup; the App passes it in via
//! [`App::action_metas`].

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::actions::ActionInfo;
use crate::app::{App, Focus};
use crate::jobs::JobStatus;

/// One visible row in the rendered sidebar.
#[derive(Debug, Clone)]
pub enum Row {
    Category(String),
    Action {
        index: usize,
        label: String,
        status: Option<JobStatus>,
    },
    Separator,
    Quit,
}

/// Build the full list of visible rows (categories + actions + separators +
/// quit footer). Pure function so the result is testable in isolation.
pub fn build_rows(metas: &[ActionInfo], last_status: &[Option<JobStatus>]) -> Vec<Row> {
    let mut out = Vec::new();
    let mut last_cat: Option<&str> = None;
    for (i, meta) in metas.iter().enumerate() {
        if last_cat.map(|c| c != meta.category).unwrap_or(true) {
            if last_cat.is_some() {
                out.push(Row::Separator);
            }
            out.push(Row::Category(meta.category.to_string()));
            last_cat = Some(meta.category);
        }
        let status = last_status.get(i).copied().flatten();
        out.push(Row::Action {
            index: i,
            label: meta.label.to_string(),
            status,
        });
    }
    out.push(Row::Separator);
    out.push(Row::Quit);
    out
}

/// Map `app.sidebar_index` (an action index) to the visible row index.
pub fn visible_index_for(rows: &[Row], action_index: usize) -> Option<usize> {
    rows.iter().position(|r| match r {
        Row::Action { index, .. } => *index == action_index,
        _ => false,
    })
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.style_border(focused))
        .title(Span::styled(
            " Actions ",
            app.theme.style_border(focused).fg(app.theme.border_focused),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = build_rows(&app.action_metas, &app.last_status);
    let selected_visible = visible_index_for(&rows, app.sidebar_index);

    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| match r {
            Row::Category(cat) => ListItem::new(Line::from(Span::styled(
                format!(" {}", cat),
                app.theme.style_sidebar_category(),
            ))),
            Row::Separator => ListItem::new(Line::from("")),
            Row::Action { label, status, .. } => {
                let prefix = match status {
                    Some(JobStatus::Running) => "… ",
                    Some(JobStatus::Done) => "✓ ",
                    Some(JobStatus::Failed) => "✗ ",
                    Some(JobStatus::Cancelled) => "↻ ",
                    None => "  ",
                };
                let prefix_style = match status {
                    Some(JobStatus::Done) => app.theme.style_highlight().fg(app.theme.log_success),
                    Some(JobStatus::Failed) => {
                        app.theme.style_highlight().fg(app.theme.log_failure)
                    }
                    Some(JobStatus::Running) => app.theme.style_highlight().fg(app.theme.log_info),
                    Some(JobStatus::Cancelled) => app.theme.style_dim(),
                    None => app.theme.style_dim(),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} ", prefix), prefix_style),
                    Span::raw(label),
                ]))
            }
            Row::Quit => ListItem::new(Line::from(Span::styled(
                "   ─────────────",
                app.theme.style_dim(),
            ))),
        })
        .collect();

    let list = List::new(items)
        .highlight_style(app.theme.style_highlight())
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(selected_visible);

    frame.render_stateful_widget(list, inner, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metas() -> Vec<ActionInfo> {
        crate::actions::metadata()
    }

    #[test]
    fn rows_contain_all_registered_actions() {
        let rows = build_rows(&metas(), &[]);
        let action_count = rows
            .iter()
            .filter(|r| matches!(r, Row::Action { .. }))
            .count();
        assert_eq!(action_count, metas().len());
    }

    #[test]
    fn sidebar_starts_with_category_header() {
        let rows = build_rows(&metas(), &[]);
        // First category is "Dev" (from the TestAction).
        assert!(matches!(rows[0], Row::Category(ref c) if c == "Dev"));
    }

    #[test]
    fn visible_index_for_first_action() {
        let rows = build_rows(&metas(), &[]);
        let idx = visible_index_for(&rows, 0).expect("action 0 exists");
        assert!(matches!(rows[idx], Row::Action { index: 0, .. }));
    }

    #[test]
    fn status_badges_appear_when_set() {
        let metas = metas();
        let last_status = vec![Some(JobStatus::Done); metas.len()];
        let rows = build_rows(&metas, &last_status);
        // First action is the test action; should show ✓.
        for r in &rows {
            if let Row::Action {
                index: 0,
                status: Some(s),
                ..
            } = r
            {
                assert_eq!(*s, JobStatus::Done);
                return;
            }
        }
        panic!("expected an action row for index 0");
    }
}
