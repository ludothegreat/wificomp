use chrono::{DateTime, Utc};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

use crate::data::Session;
use crate::ui::widgets::SignalGraph;
use crate::utils::truncate;

/// History screen state
#[derive(Debug)]
pub struct HistoryState {
    pub session: Option<Session>,
    pub selected_ap_idx: usize,
    pub time_window_mins: u64,
    pub show_average: bool,
    pub scroll_offset: usize,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            session: None,
            selected_ap_idx: 0,
            time_window_mins: 5,
            show_average: false,
            scroll_offset: 0,
        }
    }
}

impl HistoryState {
    pub fn select_next_ap(&mut self) {
        if let Some(session) = &self.session {
            let aps = session.unique_aps();
            if !aps.is_empty() {
                self.selected_ap_idx = (self.selected_ap_idx + 1).min(aps.len() - 1);
            }
        }
    }

    pub fn select_prev_ap(&mut self) {
        self.selected_ap_idx = self.selected_ap_idx.saturating_sub(1);
    }

    pub fn cycle_time_window(&mut self) {
        self.time_window_mins = match self.time_window_mins {
            5 => 10,
            10 => 30,
            30 => 0, // All
            _ => 5,
        };
    }

    pub fn toggle_average(&mut self) {
        self.show_average = !self.show_average;
    }

    pub fn get_selected_ap(&self) -> Option<(String, String)> {
        self.session.as_ref().and_then(|s| {
            let aps = s.unique_aps();
            aps.get(self.selected_ap_idx).cloned()
        })
    }

    pub fn get_ap_data(&self) -> Vec<(DateTime<Utc>, i32)> {
        let Some(session) = &self.session else {
            return Vec::new();
        };
        let Some((bssid, _)) = self.get_selected_ap() else {
            return Vec::new();
        };

        session
            .scans
            .iter()
            .flat_map(|scan| {
                scan.access_points
                    .iter()
                    .filter(|ap| ap.bssid == bssid)
                    .map(|ap| (scan.timestamp, ap.signal_dbm))
            })
            .collect()
    }
}

/// History screen widget
pub struct HistoryScreen<'a> {
    state: &'a HistoryState,
}

impl<'a> HistoryScreen<'a> {
    pub fn new(state: &'a HistoryState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for HistoryScreen<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::vertical([
            Constraint::Length(2), // Header
            Constraint::Length(2), // AP selector and controls
            Constraint::Min(8),    // Graph
            Constraint::Length(2), // Stats
            Constraint::Length(2), // Footer
        ])
        .split(area);

        self.render_header(chunks[0], buf);
        self.render_controls(chunks[1], buf);
        self.render_graph(chunks[2], buf);
        self.render_stats(chunks[3], buf);
        self.render_footer(chunks[4], buf);
    }
}

impl<'a> HistoryScreen<'a> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        let info = if let Some(session) = &self.state.session {
            format!(
                "{} | {} | {} scans",
                session.adapter.display_name(),
                session.started_at.format("%m-%d %H:%M"),
                session.scans.len()
            )
        } else {
            "No session loaded".to_string()
        };

        let info_display = truncate(&info, inner.width as usize - 8);
        buf.set_string(inner.x, inner.y, &info_display, Style::default());
        buf.set_string(
            inner.x + inner.width - 6,
            inner.y,
            "[l]oad",
            Style::default().fg(Color::DarkGray),
        );
    }

    fn render_controls(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        // AP selector
        let ap_info = if let Some((bssid, ssid)) = self.state.get_selected_ap() {
            let ssid_display = if ssid.is_empty() { "<hidden>" } else { &ssid };
            format!("AP: {} ({})", truncate(ssid_display, 20), bssid)
        } else {
            "No APs".to_string()
        };

        buf.set_string(inner.x, inner.y, &ap_info, Style::default());
        buf.set_string(
            inner.x + inner.width - 6,
            inner.y,
            "[↑][↓]",
            Style::default().fg(Color::DarkGray),
        );

        // Time window and data mode
        let time_str = if self.state.time_window_mins == 0 {
            "All".to_string()
        } else {
            format!("{}m", self.state.time_window_mins)
        };
        let data_str = if self.state.show_average { "Avg" } else { "Raw" };

        let controls = format!("Time: [{}]   Data: [{}]", time_str, data_str);
        buf.set_string(inner.x, inner.y + 1, &controls, Style::default());
    }

    fn render_graph(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .title(" Signal Strength ");
        let inner = block.inner(area);
        block.render(area, buf);

        let data = self.state.get_ap_data();
        let time_window = if self.state.time_window_mins == 0 {
            u64::MAX
        } else {
            self.state.time_window_mins
        };

        SignalGraph::new(&data)
            .time_window(time_window)
            .show_average(self.state.show_average)
            .render(inner, buf);
    }

    fn render_stats(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        if let Some((bssid, _)) = self.state.get_selected_ap() {
            if let Some(session) = &self.state.session {
                if let Some(stats) = session.ap_stats(&bssid) {
                    let stats_str = format!(
                        "Avg: {}  Min: {}  Max: {}  Readings: {}",
                        stats.avg, stats.min, stats.max, stats.count
                    );
                    buf.set_string(inner.x, inner.y, &stats_str, Style::default());
                }
            }
        }
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        let help = "[↑↓]AP [w]indow [d]ata [e]xport [q]uit";
        buf.set_string(inner.x, inner.y, help, Style::default().fg(Color::DarkGray));
    }
}
