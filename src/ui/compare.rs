use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::data::{CompareMetric, MatchBy, Session};
use crate::ui::widgets::ComparisonBar;
use crate::utils::truncate;

/// Compare screen state
#[derive(Debug, Default)]
pub struct CompareState {
    pub sessions: Vec<Session>,
    pub selected_session_idx: usize,
    pub session_list_offset: usize,
    pub selected_ap_idx: usize,
    pub match_by: MatchBy,
    pub metric: CompareMetric,
}

impl CompareState {
    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
    }

    pub fn remove_selected_session(&mut self) {
        if !self.sessions.is_empty() {
            self.sessions.remove(self.selected_session_idx);
            if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
                self.selected_session_idx = self.sessions.len() - 1;
            }
            // Adjust offset if needed
            if self.session_list_offset > 0 && self.session_list_offset >= self.sessions.len() {
                self.session_list_offset = self.sessions.len().saturating_sub(1);
            }
        }
    }

    pub fn select_next_session(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_session_idx = (self.selected_session_idx + 1).min(self.sessions.len() - 1);
        }
    }

    pub fn select_prev_session(&mut self) {
        self.selected_session_idx = self.selected_session_idx.saturating_sub(1);
    }

    /// Ensure selected session is visible in the list
    pub fn ensure_session_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected_session_idx < self.session_list_offset {
            self.session_list_offset = self.selected_session_idx;
        } else if self.selected_session_idx >= self.session_list_offset + visible_height {
            self.session_list_offset = self.selected_session_idx - visible_height + 1;
        }
    }

    pub fn select_next_ap(&mut self) {
        let aps = self.all_aps();
        if !aps.is_empty() {
            self.selected_ap_idx = (self.selected_ap_idx + 1).min(aps.len() - 1);
        }
    }

    pub fn select_prev_ap(&mut self) {
        self.selected_ap_idx = self.selected_ap_idx.saturating_sub(1);
    }

    pub fn cycle_match(&mut self) {
        self.match_by = self.match_by.next();
    }

    pub fn cycle_metric(&mut self) {
        self.metric = self.metric.next();
    }

    /// Get all unique APs across all sessions
    pub fn all_aps(&self) -> Vec<(String, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut aps = Vec::new();

        for session in &self.sessions {
            for (bssid, ssid) in session.unique_aps() {
                let key = match self.match_by {
                    MatchBy::Bssid => bssid.clone(),
                    MatchBy::Ssid => ssid.clone(),
                    MatchBy::Both => format!("{}|{}", bssid, ssid),
                };
                if seen.insert(key) {
                    aps.push((bssid, ssid));
                }
            }
        }
        aps
    }

    pub fn get_selected_ap(&self) -> Option<(String, String)> {
        self.all_aps().get(self.selected_ap_idx).cloned()
    }

    /// Get comparison data for the selected AP
    pub fn get_comparison_data(&self) -> Vec<(String, Option<i32>)> {
        let Some((sel_bssid, sel_ssid)) = self.get_selected_ap() else {
            return Vec::new();
        };

        self.sessions
            .iter()
            .map(|session| {
                let name = session
                    .adapter
                    .label
                    .clone()
                    .unwrap_or_else(|| session.adapter.interface.clone());

                // Find matching AP in this session
                let stats = session.scans.iter().flat_map(|s| &s.access_points).find(|ap| {
                    match self.match_by {
                        MatchBy::Bssid => ap.bssid == sel_bssid,
                        MatchBy::Ssid => ap.ssid == sel_ssid,
                        MatchBy::Both => ap.bssid == sel_bssid && ap.ssid == sel_ssid,
                    }
                });

                if stats.is_some() {
                    // Calculate metric
                    let matching_aps: Vec<_> = session
                        .scans
                        .iter()
                        .flat_map(|s| &s.access_points)
                        .filter(|ap| match self.match_by {
                            MatchBy::Bssid => ap.bssid == sel_bssid,
                            MatchBy::Ssid => ap.ssid == sel_ssid,
                            MatchBy::Both => ap.bssid == sel_bssid && ap.ssid == sel_ssid,
                        })
                        .collect();

                    if matching_aps.is_empty() {
                        return (name, None);
                    }

                    let signal = match self.metric {
                        CompareMetric::Avg => {
                            let sum: i32 = matching_aps.iter().map(|ap| ap.signal_dbm).sum();
                            sum / matching_aps.len() as i32
                        }
                        CompareMetric::Min => matching_aps.iter().map(|ap| ap.signal_dbm).min().unwrap(),
                        CompareMetric::Max => matching_aps.iter().map(|ap| ap.signal_dbm).max().unwrap(),
                    };
                    (name, Some(signal))
                } else {
                    (name, None)
                }
            })
            .collect()
    }

    /// Calculate which adapter is "best" (most APs with strongest signal)
    pub fn best_adapter(&self) -> Option<String> {
        if self.sessions.is_empty() {
            return None;
        }

        let aps = self.all_aps();
        let mut wins: Vec<usize> = vec![0; self.sessions.len()];

        for (bssid, ssid) in &aps {
            let mut best_signal = i32::MIN;
            let mut best_idx = None;

            for (idx, session) in self.sessions.iter().enumerate() {
                let signal = session
                    .scans
                    .iter()
                    .flat_map(|s| &s.access_points)
                    .filter(|ap| match self.match_by {
                        MatchBy::Bssid => &ap.bssid == bssid,
                        MatchBy::Ssid => &ap.ssid == ssid,
                        MatchBy::Both => &ap.bssid == bssid && &ap.ssid == ssid,
                    })
                    .map(|ap| ap.signal_dbm)
                    .max();

                if let Some(s) = signal {
                    if s > best_signal {
                        best_signal = s;
                        best_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = best_idx {
                wins[idx] += 1;
            }
        }

        let (best_idx, best_wins) = wins
            .iter()
            .enumerate()
            .max_by_key(|(_, w)| *w)?;

        let name = self.sessions[best_idx]
            .adapter
            .label
            .clone()
            .unwrap_or_else(|| self.sessions[best_idx].adapter.interface.clone());

        Some(format!("{} ({}/{} APs)", name, best_wins, aps.len()))
    }
}

/// Compare screen widget
pub struct CompareScreen<'a> {
    state: &'a CompareState,
}

impl<'a> CompareScreen<'a> {
    pub fn new(state: &'a CompareState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for CompareScreen<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Dynamic session list height - at least 4, up to 6 depending on terminal size
        let session_height = if area.height > 20 { 6 } else { 4 };

        let chunks = Layout::vertical([
            Constraint::Length(2),              // Header
            Constraint::Length(session_height), // Session list (scrollable)
            Constraint::Length(2),              // AP selector and controls
            Constraint::Min(5),                 // Comparison bars
            Constraint::Length(2),              // Summary
            Constraint::Length(2),              // Footer
        ])
        .split(area);

        self.render_header(chunks[0], buf);
        self.render_sessions(chunks[1], buf);
        self.render_controls(chunks[2], buf);
        self.render_comparison(chunks[3], buf);
        self.render_summary(chunks[4], buf);
        self.render_footer(chunks[5], buf);
    }
}

impl<'a> CompareScreen<'a> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        let info = format!("Sessions: {} loaded", self.state.sessions.len());
        buf.set_string(inner.x, inner.y, &info, Style::default());
        buf.set_string(
            inner.x + inner.width - 12,
            inner.y,
            "[+]add [x]del",
            Style::default().fg(Color::DarkGray),
        );
    }

    fn render_sessions(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.sessions.is_empty() {
            buf.set_string(
                inner.x,
                inner.y,
                "No sessions loaded. Press [+] to add.",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let visible_height = inner.height as usize;
        let offset = self.state.session_list_offset;

        // Show scroll indicator if there are more sessions
        if self.state.sessions.len() > visible_height {
            let indicator = format!(
                "[{}-{}/{}]",
                offset + 1,
                (offset + visible_height).min(self.state.sessions.len()),
                self.state.sessions.len()
            );
            if inner.width > indicator.len() as u16 + 2 {
                buf.set_string(
                    inner.x + inner.width - indicator.len() as u16,
                    inner.y,
                    &indicator,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }

        for (i, session) in self.state.sessions.iter()
            .skip(offset)
            .take(visible_height)
            .enumerate()
        {
            let actual_idx = offset + i;
            let y = inner.y + i as u16;
            let is_selected = actual_idx == self.state.selected_session_idx;

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Clear line first
            for x in inner.x..inner.x + inner.width {
                buf.set_string(x, y, " ", style);
            }

            let prefix = format!("{}. ", actual_idx + 1);
            let name = session.adapter.display_name();
            let scan_info = if session.scans.is_empty() {
                "(no data)".to_string()
            } else {
                format!("{} scans", session.scans.len())
            };
            let info = format!(
                "{} - {}",
                session.started_at.format("%m-%d %H:%M"),
                scan_info
            );

            buf.set_string(inner.x, y, &prefix, style);
            buf.set_string(
                inner.x + prefix.len() as u16,
                y,
                &truncate(&name, 18),
                style,
            );

            let info_x = inner.x + 22;
            if info_x < inner.x + inner.width {
                let max_info_len = (inner.width - 22) as usize;
                buf.set_string(info_x, y, &truncate(&info, max_info_len), style.fg(Color::DarkGray));
            }
        }
    }

    fn render_controls(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        // AP selector
        let ap_info = if let Some((_, ssid)) = self.state.get_selected_ap() {
            let ssid_display = if ssid.is_empty() { "<hidden>" } else { &ssid };
            format!("AP: {}", truncate(ssid_display, 30))
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

        // Match and metric controls
        let controls = format!(
            "Match: [{}]   Metric: [{}]",
            self.state.match_by.name(),
            self.state.metric.name()
        );
        buf.set_string(inner.x, inner.y + 1, &controls, Style::default());
    }

    fn render_comparison(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        let data = self.state.get_comparison_data();
        if data.is_empty() {
            buf.set_string(
                inner.x,
                inner.y,
                "Select an AP to compare",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        ComparisonBar::new(data).render(inner, buf);
    }

    fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        if let Some(best) = self.state.best_adapter() {
            let summary = format!("Best: {}", best);
            buf.set_string(
                inner.x,
                inner.y,
                &summary,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);
        let inner = block.inner(area);
        block.render(area, buf);

        let help = "[+]add [x]del [←→]sess [↑↓]AP [m]atch [M]etric [e]xp [q]uit";
        let help_display = truncate(help, inner.width as usize);
        buf.set_string(inner.x, inner.y, &help_display, Style::default().fg(Color::DarkGray));
    }
}
