//! Animated welcome panel with amber-gold breathing border and gradient title.
//!
//! Clean layout: double-line border containing title + subtitle,
//! model info and keyboard shortcuts below. No rain — minimal and focused.
//!
//! Visual identity (vs opendev-tui):
//! - Amber-gold (hue 30-60) instead of cyan-blue (190-250)
//! - Double-line border (╔═╗║╚═╝) instead of rounded (╭─╮│╰─╯)
//! - "O C T O  A G E N T" title with "Autonomous AI Agent Workbench" subtitle
//! - Breathing gradient animation on border + title

mod color;
mod state;

pub use state::WelcomePanelState;

use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

use color::hsl_to_rgb;

/// Stateless widget that renders the welcome panel from `WelcomePanelState`.
pub struct WelcomePanel<'a> {
    state: &'a WelcomePanelState,
    model_name: &'a str,
}

impl<'a> WelcomePanel<'a> {
    pub fn new(state: &'a WelcomePanelState, model_name: &'a str) -> Self {
        Self { state, model_name }
    }

    #[inline]
    fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, ch: char, fg: Color) {
        if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(ch);
                cell.set_fg(fg);
            }
        }
    }

    /// Write a centered string with a single color.
    fn center_text(buf: &mut Buffer, area: Rect, y: u16, text: &str, fg: Color) {
        if y >= area.y + area.height {
            return;
        }
        let len = text.len() as u16;
        let x = area.x + area.width.saturating_sub(len) / 2;
        buf.set_string(x, y, text, ratatui::style::Style::default().fg(fg));
    }

    /// Write a centered string with per-character amber gradient sweep.
    fn write_gradient_line(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        text: &str,
        base_lightness: f64,
    ) {
        if y >= area.y + area.height {
            return;
        }
        let text_len = text.chars().count() as u16;
        let start_x = area.x + area.width.saturating_sub(text_len) / 2;
        let fade = self.state.fade_progress as f64;
        let breathe = 0.85 + 0.15 * self.state.breathe_phase.sin();

        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let sweep = (i as u16 * 5 + self.state.gradient_offset) % 360;
            let hue = 30.0 + (sweep as f64 / 360.0) * 30.0;
            let lit = base_lightness * breathe * fade;
            let color = hsl_to_rgb(hue, 0.80 * fade, lit);
            Self::put(buf, area, start_x + i as u16, y, ch, color);
        }
    }

    /// Draw double-line border with animated amber gradient.
    fn draw_border(&self, buf: &mut Buffer, area: Rect, bx: u16, by: u16, bw: u16, bh: u16) {
        let offset = self.state.gradient_offset;
        let fade = self.state.fade_progress as f64;
        let breathe = 0.85 + 0.15 * self.state.breathe_phase.sin();
        let perimeter = 2 * (bw + bh);

        let border_color = |idx: u16| -> Color {
            let t = ((idx as f64 / perimeter as f64) + offset as f64 / 360.0) % 1.0;
            let hue = 30.0 + t * 30.0;
            hsl_to_rgb(hue, 0.60 * fade, 0.28 * breathe * fade)
        };

        // Top: ╔═══╗
        Self::put(buf, area, bx, by, '\u{2554}', border_color(0));
        for i in 1..bw.saturating_sub(1) {
            Self::put(buf, area, bx + i, by, '\u{2550}', border_color(i));
        }
        Self::put(buf, area, bx + bw - 1, by, '\u{2557}', border_color(bw));

        // Bottom: ╚═══╝
        Self::put(buf, area, bx, by + bh - 1, '\u{255a}', border_color(bw + bh));
        for i in 1..bw.saturating_sub(1) {
            Self::put(buf, area, bx + i, by + bh - 1, '\u{2550}', border_color(bw + bh + i));
        }
        Self::put(buf, area, bx + bw - 1, by + bh - 1, '\u{255d}', border_color(2 * bw + bh));

        // Sides: ║
        for j in 1..bh.saturating_sub(1) {
            Self::put(buf, area, bx, by + j, '\u{2551}', border_color(bw + j));
            Self::put(buf, area, bx + bw - 1, by + j, '\u{2551}', border_color(2 * bw + bh + j));
        }
    }
}

impl Widget for WelcomePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        let fade = self.state.fade_progress as f64;
        let dim = hsl_to_rgb(40.0, 0.25 * fade, 0.35 * fade);

        // Layout constants
        let title = "O C T O   A G E N T";
        let subtitle = "Autonomous AI Agent Workbench";
        let model_line = format!("model: {}", self.model_name);
        let help = "Enter: send  |  Ctrl+C: cancel  |  Ctrl+D: debug  |  Ctrl+E: eval";

        if area.height < 5 {
            // ── Tier 1: tiny terminal — just title ──
            let cy = area.y + area.height / 2;
            self.write_gradient_line(buf, area, cy, title, 0.55);
        } else if area.height < 10 {
            // ── Tier 2: small — border + title + subtitle ──
            let box_w = (area.width.saturating_sub(4)).min(50);
            let box_h = 5u16.min(area.height);
            let bx = area.x + (area.width.saturating_sub(box_w)) / 2;
            let by = area.y + (area.height.saturating_sub(box_h)) / 2;

            self.draw_border(buf, area, bx, by, box_w, box_h);
            self.write_gradient_line(buf, area, by + 1, title, 0.55);
            Self::center_text(buf, area, by + 3, subtitle, dim);
        } else {
            // ── Tier 3: full — border + title + subtitle, model, help ──
            let box_w = (area.width.saturating_sub(4)).min(50);
            let box_h = 6u16;

            // Total content: box(6) + blank(1) + model(1) + blank(1) + help(1) = 10
            let total_h = box_h + 4;
            let start_y = area.y + area.height.saturating_sub(total_h) / 2;
            let bx = area.x + (area.width.saturating_sub(box_w)) / 2;

            // Border box
            self.draw_border(buf, area, bx, start_y, box_w, box_h);

            // Title (row 2 inside box)
            self.write_gradient_line(buf, area, start_y + 2, title, 0.55);

            // Subtitle (row 4 inside box)
            Self::center_text(buf, area, start_y + 4, subtitle, dim);

            // Model info (below box + 1 blank)
            let model_y = start_y + box_h + 1;
            Self::center_text(buf, area, model_y, &model_line, dim);

            // Help (below model + 1 blank)
            let help_y = model_y + 2;
            Self::center_text(buf, area, help_y, help, dim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_panel_renders_without_panic() {
        let state = WelcomePanelState::new();
        let widget = WelcomePanel::new(&state, "test-model");
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        let modified: usize = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.cell((x, y)).unwrap().symbol() != " ")
            .count();
        assert!(modified > 20, "Expected visible output, got {modified} cells");
    }

    #[test]
    fn welcome_panel_small_terminal() {
        let state = WelcomePanelState::new();
        let widget = WelcomePanel::new(&state, "test-model");
        let area = Rect::new(0, 0, 80, 4);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        let modified: usize = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.cell((x, y)).unwrap().symbol() != " ")
            .count();
        assert!(modified > 5, "Tier 1 should render text, got {modified} cells");
    }

    #[test]
    fn welcome_panel_tier2() {
        let state = WelcomePanelState::new();
        let widget = WelcomePanel::new(&state, "gpt-4o");
        let area = Rect::new(0, 0, 60, 8);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        let modified: usize = (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.cell((x, y)).unwrap().symbol() != " ")
            .count();
        assert!(modified > 15, "Tier 2 should render border + text, got {modified} cells");
    }

    #[test]
    fn welcome_panel_full_layout() {
        let state = WelcomePanelState::new();
        let widget = WelcomePanel::new(&state, "gpt-4o");
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        // Collect all rendered text
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("model: gpt-4o"), "Should show model info");
    }
}
