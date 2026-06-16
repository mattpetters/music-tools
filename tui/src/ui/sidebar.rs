//! Action sidebar. Renders a vertical list grouped by category, with the
//! current selection highlighted. `app.sidebar_index` indexes into
//! [`ACTION_DEFS`], not into the visible list (which is interspersed with
//! category headers and separators).

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;

/// All the actions the TUI knows about, in display order. The category is
/// used for grouping; consecutive items in the same category share a header.
///
/// The id is the stable handle used by the Action trait in M2+. The label
/// is the human-readable sidebar text.
pub const ACTION_DEFS: &[(
    /*category*/ &str,
    /*id*/ &str,
    /*label*/ &str,
    /*hint*/ &str,
)] = &[
    (
        "Installers",
        "install_pkgs",
        "Install .pkg files",
        "Install every .pkg in a chosen directory",
    ),
    (
        "Installers",
        "run_patchers",
        "Run patchers (.command)",
        "Run every .command in a chosen directory",
    ),
    (
        "Plugins",
        "move_kd_plugs",
        "Move k'd plugins",
        "Copy *.vst / *.vst3 / *.component into /Library/Audio/Plug-Ins",
    ),
    (
        "Plugins",
        "remove_protection",
        "Remove protection",
        "Strip xattrs/quarantine and re-codesign a file or folder",
    ),
    (
        "Maintenance",
        "reset_ableton",
        "Reset Ableton",
        "Back up & clear prefs + templates for an installed Ableton version",
    ),
    (
        "System patches",
        "patch_ozone",
        "Patch iZotope Ozone 12",
        "Binary-patch iZotope Ozone 12 core libraries (system-level)",
    ),
];

/// Number of selectable actions. The "Quit" line is a non-selectable footer.
pub const ACTION_COUNT: usize = ACTION_DEFS.len();

/// One visible row in the rendered sidebar.
#[derive(Debug, Clone)]
pub enum Row {
    Category(String),
    Action { index: usize, label: String },
    Separator,
    Quit,
}

/// Build the full list of visible rows (categories + actions + separators +
/// quit footer). Pure function so the result is testable in isolation.
pub fn build_rows() -> Vec<Row> {
    let mut out = Vec::new();
    let mut last_cat: Option<&str> = None;
    for (i, (cat, _id, label, _hint)) in ACTION_DEFS.iter().enumerate() {
        if last_cat.map(|c| c != *cat).unwrap_or(true) {
            if last_cat.is_some() {
                out.push(Row::Separator);
            }
            out.push(Row::Category(cat.to_string()));
            last_cat = Some(cat);
        }
        out.push(Row::Action {
            index: i,
            label: label.to_string(),
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

/// Look up the human-readable label for an action index, if any.
pub fn action_label(index: usize) -> Option<&'static str> {
    ACTION_DEFS.get(index).map(|t| t.2)
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == crate::app::Focus::Sidebar;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.style_border(focused))
        .title(Span::styled(
            " Actions ",
            app.theme.style_border(focused).fg(app.theme.border_focused),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = build_rows();
    let selected_visible = visible_index_for(&rows, app.sidebar_index);

    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| match r {
            Row::Category(cat) => ListItem::new(Line::from(Span::styled(
                format!(" {}", cat),
                app.theme.style_sidebar_category(),
            ))),
            Row::Separator => ListItem::new(Line::from("")),
            Row::Action { label, .. } => {
                ListItem::new(Line::from(Span::raw(format!("   {}", label))))
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

    #[test]
    fn rows_contain_all_six_actions() {
        let rows = build_rows();
        let action_count = rows
            .iter()
            .filter(|r| matches!(r, Row::Action { .. }))
            .count();
        assert_eq!(action_count, ACTION_COUNT);
        assert_eq!(action_count, 6);
    }

    #[test]
    fn visible_index_for_first_action() {
        let rows = build_rows();
        // First visible row should be the first category header, action 0 is
        // right after. The exact index depends on category layout, but for the
        // current data the first action is the second item in its category,
        // i.e. right after the category header.
        let idx = visible_index_for(&rows, 0).expect("action 0 exists");
        assert!(idx >= 1);
        assert!(matches!(rows[idx], Row::Action { index: 0, .. }));
    }

    #[test]
    fn sidebar_starts_with_category_header() {
        let rows = build_rows();
        assert!(matches!(rows[0], Row::Category(ref c) if c == "Installers"));
    }

    #[test]
    fn action_labels_resolve() {
        assert_eq!(action_label(0), Some("Install .pkg files"));
        assert_eq!(action_label(5), Some("Patch iZotope Ozone 12"));
        assert_eq!(action_label(99), None);
    }
}
