//! Top status bar: logo, version on the left; help/quit hint on the right.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // logo + version
            Constraint::Min(1),     // context (unused in M1)
            Constraint::Length(26), // right-side hint
        ])
        .split(area);

    let logo = Line::from(vec![
        Span::styled(" mtui ", app.theme.style_status_bar()),
        Span::raw(" "),
        Span::styled(env!("CARGO_PKG_VERSION"), app.theme.style_dim()),
    ]);
    frame.render_widget(Paragraph::new(logo), chunks[0]);

    let hint = Line::from(Span::styled(
        "? for help  ·  q to quit ",
        app.theme.style_dim(),
    ));
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Right), chunks[2]);
}
