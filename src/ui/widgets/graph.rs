use chrono::{DateTime, Utc};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// A time-series graph for signal strength
pub struct SignalGraph<'a> {
    data: &'a [(DateTime<Utc>, i32)],
    time_window_mins: u64,
    show_average: bool,
}

impl<'a> SignalGraph<'a> {
    pub fn new(data: &'a [(DateTime<Utc>, i32)]) -> Self {
        Self {
            data,
            time_window_mins: 5,
            show_average: false,
        }
    }

    pub fn time_window(mut self, mins: u64) -> Self {
        self.time_window_mins = mins;
        self
    }

    pub fn show_average(mut self, show: bool) -> Self {
        self.show_average = show;
        self
    }
}

impl<'a> Widget for SignalGraph<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 5 || self.data.is_empty() {
            if self.data.is_empty() {
                buf.set_string(
                    area.x,
                    area.y + area.height / 2,
                    "No data",
                    Style::default().fg(Color::DarkGray),
                );
            }
            return;
        }

        // Reserve space for Y-axis labels and X-axis
        let y_label_width = 4; // "-99│"
        let graph_x = area.x + y_label_width;
        let graph_width = area.width.saturating_sub(y_label_width);
        let graph_height = area.height.saturating_sub(2); // Leave 2 lines for X-axis

        if graph_width == 0 || graph_height == 0 {
            return;
        }

        // Filter data by time window
        let now = Utc::now();
        let window_start = now - chrono::Duration::minutes(self.time_window_mins as i64);
        let filtered: Vec<_> = self
            .data
            .iter()
            .filter(|(t, _)| *t >= window_start)
            .collect();

        if filtered.is_empty() {
            buf.set_string(
                graph_x,
                area.y + graph_height / 2,
                "No data in time window",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        // Calculate Y-axis range (-40 to -90 is typical)
        let min_signal = filtered.iter().map(|(_, s)| *s).min().unwrap_or(-90);
        let max_signal = filtered.iter().map(|(_, s)| *s).max().unwrap_or(-40);
        let y_min = (min_signal - 5).max(-100);
        let y_max = (max_signal + 5).min(-20);
        // Ensure y_range is never zero to avoid division by zero
        let y_range = ((y_max - y_min) as f32).max(1.0);

        // Draw Y-axis labels
        let labels = [y_max, (y_max + y_min) / 2, y_min];
        let gh_safe = graph_height.saturating_sub(1).max(1);
        for (i, &label) in labels.iter().enumerate() {
            let y = area.y + (i as u16 * gh_safe / 2);
            if y < area.y + area.height {
                buf.set_string(
                    area.x,
                    y,
                    format!("{:>3}│", label),
                    Style::default().fg(Color::DarkGray),
                );
            }
        }

        // Draw vertical axis line
        let axis_x = graph_x.saturating_sub(1);
        for y in area.y..area.y + graph_height {
            if y < area.y + area.height && axis_x >= area.x {
                buf.set_string(
                    axis_x,
                    y,
                    "│",
                    Style::default().fg(Color::DarkGray),
                );
            }
        }

        // Draw horizontal axis
        let axis_y = area.y + graph_height;
        if axis_y < area.y + area.height {
            buf.set_string(
                area.x,
                axis_y,
                "   └",
                Style::default().fg(Color::DarkGray),
            );
            let axis_end = (graph_x + graph_width).min(area.x + area.width);
            for x in graph_x..axis_end {
                buf.set_string(x, axis_y, "─", Style::default().fg(Color::DarkGray));
            }
        }

        // Draw data points
        let time_start = filtered.first().unwrap().0;
        let time_end = now;
        let time_range = (time_end - time_start).num_seconds() as f32;

        if time_range > 0.0 && graph_width > 0 {
            // Group points by X position and average if needed
            let mut columns: Vec<Vec<i32>> = vec![Vec::new(); graph_width as usize];

            let gw_safe = (graph_width as usize).saturating_sub(1).max(1);
            for (timestamp, signal) in &filtered {
                let elapsed = (*timestamp - time_start).num_seconds() as f32;
                let x_pos = ((elapsed / time_range) * gw_safe as f32) as usize;
                let x_pos = x_pos.min(gw_safe);
                if x_pos < columns.len() {
                    columns[x_pos].push(*signal);
                }
            }

            for (x_idx, signals) in columns.iter().enumerate() {
                if signals.is_empty() {
                    continue;
                }

                let signal = if self.show_average {
                    signals.iter().sum::<i32>() / signals.len().max(1) as i32
                } else {
                    *signals.last().unwrap()
                };

                let y_frac = ((signal - y_min) as f32 / y_range).clamp(0.0, 1.0);
                let y_pos = gh_safe as f32 * (1.0 - y_frac);
                let y = area.y + (y_pos.round() as u16).min(gh_safe);

                // Bounds check before rendering
                let render_x = graph_x + x_idx as u16;
                let render_y = y.min(area.y + graph_height.saturating_sub(1));
                if render_x < area.x + area.width && render_y < area.y + area.height {
                    let color = crate::utils::signal_color(signal);
                    buf.set_string(
                        render_x,
                        render_y,
                        "█",
                        Style::default().fg(color),
                    );
                }
            }
        }

        // Draw time labels on X-axis
        let label_y = axis_y.saturating_add(1);
        if label_y < area.y + area.height && graph_x < area.x + area.width {
            let start_label = time_start.format("%H:%M").to_string();
            let end_label = time_end.format("%H:%M").to_string();
            buf.set_string(
                graph_x,
                label_y,
                &start_label,
                Style::default().fg(Color::DarkGray),
            );
            if graph_width > 15 {
                let end_x = graph_x.saturating_add(graph_width).saturating_sub(5);
                if end_x < area.x + area.width {
                    buf.set_string(
                        end_x,
                        label_y,
                        &end_label,
                        Style::default().fg(Color::DarkGray),
                    );
                }
            }
        }
    }
}
