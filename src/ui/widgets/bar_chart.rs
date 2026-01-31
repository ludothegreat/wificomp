use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use crate::utils::{signal_bar_width, signal_color};

/// A horizontal bar for signal strength
pub struct SignalBar {
    signal_dbm: i32,
    show_value: bool,
    highlighted: bool,
}

impl SignalBar {
    pub fn new(signal_dbm: i32) -> Self {
        Self {
            signal_dbm,
            show_value: true,
            highlighted: false,
        }
    }

    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }
}

impl Widget for SignalBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Reserve space for value if showing
        let value_width = if self.show_value { 4 } else { 0 }; // "-99 " = 4 chars
        let bar_width = area.width.saturating_sub(value_width);

        // Draw value
        if self.show_value && area.width >= 4 {
            let value_str = format!("{:>3}", self.signal_dbm);
            let style = if self.highlighted {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            buf.set_string(area.x, area.y, &value_str, style);
        }

        // Draw bar
        if bar_width > 0 {
            let filled = signal_bar_width(self.signal_dbm, bar_width);
            let bar_x = area.x + value_width;
            let color = signal_color(self.signal_dbm);

            for i in 0..bar_width {
                let ch = if i < filled { '█' } else { ' ' };
                let style = Style::default().fg(color);
                buf.set_string(bar_x + i, area.y, ch.to_string(), style);
            }
        }
    }
}

/// A comparison bar chart for multiple adapters
pub struct ComparisonBar {
    values: Vec<(String, Option<i32>)>, // (name, signal)
    max_name_width: u16,
}

impl ComparisonBar {
    pub fn new(values: Vec<(String, Option<i32>)>) -> Self {
        let max_name_width = values.iter().map(|(n, _)| n.len()).max().unwrap_or(10) as u16;
        Self {
            values,
            max_name_width: max_name_width.min(20),
        }
    }
}

impl Widget for ComparisonBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || self.values.is_empty() {
            return;
        }

        // Find best signal for highlighting
        let best_signal = self
            .values
            .iter()
            .filter_map(|(_, s)| *s)
            .max();

        for (i, (name, signal)) in self.values.iter().enumerate() {
            if i as u16 >= area.height {
                break;
            }

            let y = area.y + i as u16;

            // Draw name
            let name_display = if name.len() > self.max_name_width as usize {
                format!("{}...", &name[..self.max_name_width as usize - 3])
            } else {
                format!("{:width$}", name, width = self.max_name_width as usize)
            };
            buf.set_string(area.x, y, &name_display, Style::default());

            // Draw signal bar or "N/A"
            let bar_x = area.x + self.max_name_width + 1;
            let bar_width = area.width.saturating_sub(self.max_name_width + 1);

            match signal {
                Some(s) => {
                    let is_best = best_signal == Some(*s);
                    let bar = SignalBar::new(*s).highlighted(is_best);
                    let bar_area = Rect::new(bar_x, y, bar_width, 1);
                    bar.render(bar_area, buf);

                    // Add star for best
                    if is_best && bar_width > 5 {
                        buf.set_string(
                            area.x + area.width - 2,
                            y,
                            "★",
                            Style::default().fg(Color::Yellow),
                        );
                    }
                }
                None => {
                    buf.set_string(bar_x, y, "N/A", Style::default().fg(Color::DarkGray));
                }
            }
        }
    }
}
