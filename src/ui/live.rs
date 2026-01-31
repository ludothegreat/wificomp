use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::data::{AccessPoint, Adapter, FrequencyFilter, SortBy};
use std::collections::HashSet;
use crate::ui::widgets::{ApList, ApListState};
use crate::utils::{format_timer, truncate};

/// Live scan screen state
#[derive(Debug)]
pub struct LiveState {
    pub adapter: Option<Adapter>,
    pub access_points: Vec<AccessPoint>,
    pub ap_list_state: ApListState,
    pub auto_scan: bool,
    pub auto_scan_interval: u64,
    pub timer_target_secs: Option<u64>,
    pub elapsed_secs: u64,
    pub show_channel: bool,
    pub show_band: bool,
    pub highlight_best: bool,
    pub frequency_filter: FrequencyFilter,
    pub sort_by: SortBy,
    pub last_scan_error: Option<String>,
    pub scanning: bool,
    /// Session-level excluded APs (by BSSID)
    pub session_excluded_bssids: HashSet<String>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            adapter: None,
            access_points: Vec::new(),
            ap_list_state: ApListState::default(),
            auto_scan: true,
            auto_scan_interval: 5,
            timer_target_secs: Some(300),
            elapsed_secs: 0,
            show_channel: true,
            show_band: true,
            highlight_best: true,
            frequency_filter: FrequencyFilter::All,
            sort_by: SortBy::Signal,
            last_scan_error: None,
            scanning: false,
            session_excluded_bssids: HashSet::new(),
        }
    }
}

impl LiveState {
    pub fn toggle_auto_scan(&mut self) {
        self.auto_scan = !self.auto_scan;
    }

    pub fn toggle_channel(&mut self) {
        self.show_channel = !self.show_channel;
    }

    pub fn toggle_band(&mut self) {
        self.show_band = !self.show_band;
    }

    pub fn toggle_highlight(&mut self) {
        self.highlight_best = !self.highlight_best;
    }

    pub fn cycle_filter(&mut self) {
        self.frequency_filter = self.frequency_filter.next();
        // Reset selection when filter changes to prevent index out of bounds
        self.ap_list_state.selected = 0;
        self.ap_list_state.offset = 0;
    }

    /// Exclude AP for this session only
    pub fn exclude_session(&mut self, bssid: &str) {
        self.session_excluded_bssids.insert(bssid.to_string());
        self.ap_list_state.selected = 0;
        self.ap_list_state.offset = 0;
    }

    /// Get the currently selected AP
    pub fn get_selected_ap(&self) -> Option<&AccessPoint> {
        let filtered: Vec<_> = self.access_points.iter()
            .filter(|ap| !self.session_excluded_bssids.contains(&ap.bssid))
            .filter(|ap| self.frequency_filter.matches(ap.band()))
            .collect();
        filtered.get(self.ap_list_state.selected).copied()
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.next();
    }

    pub fn timer_remaining(&self) -> Option<u64> {
        self.timer_target_secs.map(|t| t.saturating_sub(self.elapsed_secs))
    }

    pub fn timer_expired(&self) -> bool {
        self.timer_target_secs.map(|t| self.elapsed_secs >= t).unwrap_or(false)
    }
}

/// Live scan screen widget
pub struct LiveScreen<'a> {
    state: &'a LiveState,
}

impl<'a> LiveScreen<'a> {
    pub fn new(state: &'a LiveState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for LiveScreen<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout: Header (3 lines) | AP List | Footer (1 line)
        let chunks = Layout::vertical([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // AP List
            Constraint::Length(2), // Footer
        ])
        .split(area);

        // Render header
        self.render_header(chunks[0], buf);

        // Render AP list
        self.render_ap_list(chunks[1], buf);

        // Render footer
        self.render_footer(chunks[2], buf);
    }
}

impl<'a> LiveScreen<'a> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        // Guard against zero-size
        if inner.width < 10 || inner.height == 0 {
            return;
        }

        // Line 1: Adapter info
        let adapter_info = if let Some(adapter) = &self.state.adapter {
            adapter.display_name()
        } else {
            "No adapter detected".to_string()
        };
        let adapter_display = truncate(&adapter_info, inner.width.saturating_sub(10) as usize);
        buf.set_string(inner.x, inner.y, &adapter_display, Style::default());

        if inner.width >= 8 {
            buf.set_string(
                inner.x + inner.width.saturating_sub(8),
                inner.y,
                "[r]ename",
                Style::default().fg(Color::DarkGray),
            );
        }

        // Line 2: Timer, auto-scan, AP count (if there's room)
        if inner.height >= 2 {
            let timer = format_timer(
                std::time::Duration::from_secs(self.state.elapsed_secs),
                self.state.timer_target_secs.map(std::time::Duration::from_secs),
            );
            let auto_status = if self.state.auto_scan {
                format!("Auto: ON {}s", self.state.auto_scan_interval)
            } else {
                "Auto: OFF".to_string()
            };
            let ap_count = format!("APs: {}", self.state.access_points.len());
            let scanning = if self.state.scanning { " ⟳" } else { "" };

            let line2 = format!(
                "Timer: {}  {}  {}{}",
                timer, auto_status, ap_count, scanning
            );
            let line2_display = truncate(&line2, inner.width.saturating_sub(8) as usize);
            buf.set_string(inner.x, inner.y + 1, &line2_display, Style::default());

            if inner.width >= 8 {
                buf.set_string(
                    inner.x + inner.width.saturating_sub(8),
                    inner.y + 1,
                    "[t] [a]",
                    Style::default().fg(Color::DarkGray),
                );
            }
        }

        // Show error if any (if there's room for line 3)
        if inner.height >= 3 {
            if let Some(err) = &self.state.last_scan_error {
                let err_display = truncate(err, inner.width as usize);
                buf.set_string(
                    inner.x,
                    inner.y + 2,
                    &err_display,
                    Style::default().fg(Color::Red),
                );
            }
        }
    }

    fn render_ap_list(&self, area: Rect, buf: &mut Buffer) {
        // Guard against insufficient space
        if area.height < 2 || area.width < 10 {
            return;
        }

        // Column header
        let header_area = Rect::new(area.x, area.y, area.width, 1);
        let list_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));

        // Draw header with border
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let header_inner = block.inner(header_area);
        block.render(header_area, buf);

        if header_inner.width > 0 {
            let ch_col = if self.state.show_channel { "CH " } else { "" };
            let band_col = if self.state.show_band { "Band" } else { "" };
            let header = format!(
                "{:<15} Signal       {}{} Filter:{}",
                "SSID",
                ch_col,
                band_col,
                self.state.frequency_filter.name()
            );
            let header_display = truncate(&header, header_inner.width as usize);
            buf.set_string(
                header_inner.x,
                header_inner.y,
                &header_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        }

        // Draw list
        if list_area.height == 0 {
            return;
        }

        let list_block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let list_inner = list_block.inner(list_area);
        list_block.render(list_area, buf);

        // Clone state for rendering (we need to render with borrowed state)
        let mut ap_state = ApListState {
            selected: self.state.ap_list_state.selected,
            offset: self.state.ap_list_state.offset,
        };

        ApList::new(&self.state.access_points)
            .show_channel(self.state.show_channel)
            .show_band(self.state.show_band)
            .highlight_best(self.state.highlight_best)
            .filter(self.state.frequency_filter)
            .sort_by(self.state.sort_by)
            .excluded(&self.state.session_excluded_bssids)
            .render(list_inner, buf, &mut ap_state);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let sort_name = self.state.sort_by.name();
        let help = format!(
            "[spc]scan [c]h [b]and [f]req [s]ort:{} [x]clude [e]xp [q]uit",
            sort_name
        );
        let help_display = truncate(&help, inner.width as usize);
        buf.set_string(inner.x, inner.y, &help_display, Style::default().fg(Color::DarkGray));
    }
}
