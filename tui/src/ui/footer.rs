//! Bottom keybinding hint line. Shows the bindings relevant to the focused pane.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Focus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let hint = match app.focus {
        Focus::Sidebar => "[j/k] move  [1-6] jump  [Enter] select  [?] help",
        Focus::Main => "[Tab] cycle focus  [Enter] run  [?] help",
        Focus::Log => "[Tab] cycle focus  [Ctrl-L] clear  [?] help",
    };
    let p = Paragraph::new(Line::from(Span::styled(hint, app.theme.style_dim())))
        .alignment(Alignment::Center);
    frame.render_widget(p, area);
}
