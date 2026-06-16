//! Top-level render dispatcher.
//!
//! Splits the terminal area into regions, then hands each region to a
//! dedicated renderer. The help overlay is drawn last so it sits on top.
//!
//! `sidebar` is `pub` (not just `mod`) so `app.rs` can look up the static
//! action metadata. In M2 this is replaced by the real `actions` registry
//! and the visibility here gets tightened.

mod footer;
mod help;
mod layout;
mod log_pane;
mod main_pane;
pub mod sidebar;
mod status_bar;

use ratatui::Frame;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::root(frame.area());
    status_bar::render(app, frame, chunks.status);
    sidebar::render(app, frame, chunks.sidebar);
    main_pane::render(app, frame, chunks.main);
    log_pane::render(app, frame, chunks.log);
    footer::render(app, frame, chunks.footer);
    if app.show_help {
        help::render(app, frame, frame.area());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Cli;
    use ratatui::Terminal;

    fn make_app() -> App {
        // Skip CLI parsing in tests by constructing directly. Cli::parse()
        // would also work, but parsing args from `std::env::args` is flaky
        // in `cargo test` and a no-arg construction is clearer.
        App::new(Cli {
            reset_state: false,
            dry_run: false,
            log_level: "info".into(),
        })
    }

    fn render_to(width: u16, height: u16, app: &App) {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test backend");
        terminal.draw(|frame| render(app, frame)).expect("render");
    }

    #[test]
    fn render_does_not_panic_at_120x40() {
        render_to(120, 40, &make_app());
    }

    #[test]
    fn render_does_not_panic_at_80x24() {
        render_to(80, 24, &make_app());
    }

    #[test]
    fn render_with_help_open_does_not_panic() {
        let mut app = make_app();
        app.show_help = true;
        render_to(120, 40, &app);
    }

    #[test]
    fn render_does_not_panic_after_navigating_to_each_action() {
        let mut app = make_app();
        for i in 0..crate::ui::sidebar::ACTION_COUNT {
            app.sidebar_index = i;
            app.focus = crate::app::Focus::Main;
            render_to(120, 40, &app);
        }
    }

    #[test]
    fn render_handles_small_terminal() {
        // 40x10 is below our typical minimum but should still not panic.
        render_to(40, 10, &make_app());
    }
}
