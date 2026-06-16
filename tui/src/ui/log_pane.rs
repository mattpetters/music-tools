//! Bottom log pane: streaming stdout/stderr from running jobs. In M1 there
//! are no real jobs yet, so the pane shows welcome lines and any
//! app-emitted log entries. M2+ uses the same renderer for real job output.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::log::{LogLine, LogStream};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Log;
    let border = app.theme.style_border(focused);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Span::styled(
            " Log ",
            app.theme.style_border(focused).fg(app.theme.border_focused),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.log_lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("(no log lines yet)", app.theme.style_dim())),
            inner,
        );
        return;
    }

    // Show only the last N lines that fit.
    let max_lines = inner.height as usize;
    let start = app.log_lines.len().saturating_sub(max_lines);
    let items: Vec<ListItem> = app.log_lines[start..]
        .iter()
        .map(|l| ListItem::new(render_line(l, &app.theme)))
        .collect();

    frame.render_widget(List::new(items), inner);
}

fn render_line<'a>(l: &'a LogLine, theme: &crate::theme::Theme) -> Line<'a> {
    let ts = l.timestamp.format("%H:%M:%S").to_string();
    let color = match l.stream {
        LogStream::Info => theme.log_info,
        LogStream::Out => theme.log_out,
        LogStream::Err => theme.log_err,
        LogStream::Success => theme.log_success,
        LogStream::Failure => theme.log_failure,
    };
    Line::from(vec![
        Span::styled(format!(" {} ", ts), theme.style_dim()),
        Span::styled(
            format!("{} ", l.stream.badge()),
            ratatui::style::Style::default().fg(color),
        ),
        Span::styled(
            l.message.clone(),
            ratatui::style::Style::default().fg(color),
        ),
    ])
}
