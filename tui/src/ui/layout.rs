//! Root layout: status bar on top, sidebar / main+log in the middle, footer
//! at the bottom. Main pane is split vertically into the action view (top
//! 2/3) and the log pane (bottom ~30%).

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// All rectangles the UI needs. Named fields are easier to read at the call
/// site than a tuple or array.
pub struct Chunks {
    pub status: Rect,
    pub sidebar: Rect,
    pub main: Rect,
    pub log: Rect,
    pub footer: Rect,
}

pub fn root(area: Rect) -> Chunks {
    // Outer: status (1) | body (fill) | footer (1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(5),    // body
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Body: sidebar (24) | right (fill)
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24), // sidebar
            Constraint::Min(20),    // main + log
        ])
        .split(outer[1]);

    // Right side: main (2/3-ish) | log (1/3-ish, minimum 3 rows)
    let log_rows = ((area.height as u32 * 30) / 100).max(3) as u16;
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(log_rows)])
        .split(body[1]);

    Chunks {
        status: outer[0],
        sidebar: body[0],
        main: right[0],
        log: right[1],
        footer: outer[2],
    }
}
