//! Colors and reusable style helpers.
//!
//! Theme is intentionally minimal in M1 — just colors, no font/box-drawing
//! customization. Centralized so we can tweak palette without hunting through
//! every renderer.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
#[allow(dead_code)] // text_danger / log_failure are M3+; fields are referenced in renderers as they're added.
pub struct Theme {
    pub text: Color,
    pub text_dim: Color,
    pub text_warn: Color,
    pub text_danger: Color,
    pub sidebar_category: Color,
    pub border: Color,
    pub border_focused: Color,
    pub highlight_bg: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub log_info: Color,
    pub log_out: Color,
    pub log_err: Color,
    pub log_success: Color,
    pub log_failure: Color,
    pub overlay_border: Color,
    pub overlay_title: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            text: Color::White,
            text_dim: Color::DarkGray,
            text_warn: Color::Yellow,
            text_danger: Color::Red,
            sidebar_category: Color::Yellow,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            highlight_bg: Color::DarkGray,
            status_bar_bg: Color::Cyan,
            status_bar_fg: Color::Black,
            log_info: Color::Cyan,
            log_out: Color::White,
            log_err: Color::Yellow,
            log_success: Color::Green,
            log_failure: Color::Red,
            overlay_border: Color::Cyan,
            overlay_title: Color::Cyan,
        }
    }
}

impl Theme {
    pub fn style_text(&self) -> Style {
        Style::default().fg(self.text)
    }
    pub fn style_dim(&self) -> Style {
        Style::default().fg(self.text_dim)
    }
    pub fn style_warn(&self) -> Style {
        Style::default().fg(self.text_warn)
    }
    pub fn style_border(&self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.border_focused
        } else {
            self.border
        })
    }
    pub fn style_status_bar(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_fg)
            .add_modifier(Modifier::BOLD)
    }
    #[allow(dead_code)] // M3+ uses this for danger-zone borders on move_kd_plugs / patch_ozone.
    pub fn style_danger(&self) -> Style {
        Style::default().fg(self.text_danger)
    }
    pub fn style_highlight(&self) -> Style {
        Style::default()
            .bg(self.highlight_bg)
            .fg(self.text)
            .add_modifier(Modifier::BOLD)
    }
    pub fn style_sidebar_category(&self) -> Style {
        Style::default()
            .fg(self.sidebar_category)
            .add_modifier(Modifier::BOLD)
    }
}
