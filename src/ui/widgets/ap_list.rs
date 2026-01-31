use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, StatefulWidget, Widget},
};

use crate::data::{AccessPoint, FrequencyFilter, SortBy};
use crate::utils::{signal_bar_width, signal_color, truncate};

/// State for the AP list
#[derive(Debug, Default)]
pub struct ApListState {
    pub selected: usize,
    pub offset: usize,
}

impl ApListState {
    pub fn select_next(&mut self, len: usize) {
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn ensure_visible(&mut self, visible_height: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + visible_height {
            self.offset = self.selected - visible_height + 1;
        }
    }
}

/// Access point list widget
pub struct ApList<'a> {
    items: &'a [AccessPoint],
    show_channel: bool,
    show_band: bool,
    highlight_best: bool,
    filter: FrequencyFilter,
    sort_by: SortBy,
    block: Option<Block<'a>>,
    excluded_bssids: Option<&'a HashSet<String>>,
}

impl<'a> ApList<'a> {
    pub fn new(items: &'a [AccessPoint]) -> Self {
        Self {
            items,
            show_channel: true,
            show_band: true,
            highlight_best: true,
            filter: FrequencyFilter::All,
            sort_by: SortBy::Signal,
            block: None,
            excluded_bssids: None,
        }
    }

    pub fn show_channel(mut self, show: bool) -> Self {
        self.show_channel = show;
        self
    }

    pub fn show_band(mut self, show: bool) -> Self {
        self.show_band = show;
        self
    }

    pub fn highlight_best(mut self, highlight: bool) -> Self {
        self.highlight_best = highlight;
        self
    }

    pub fn excluded(mut self, excluded: &'a HashSet<String>) -> Self {
        self.excluded_bssids = Some(excluded);
        self
    }

    pub fn filter(mut self, filter: FrequencyFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn filtered_sorted(&self) -> Vec<&AccessPoint> {
        let mut items: Vec<_> = self
            .items
            .iter()
            .filter(|ap| self.filter.matches(ap.band()))
            .filter(|ap| {
                // Filter out excluded BSSIDs
                if let Some(excluded) = &self.excluded_bssids {
                    !excluded.contains(&ap.bssid)
                } else {
                    true
                }
            })
            .collect();

        match self.sort_by {
            SortBy::Signal => items.sort_by(|a, b| b.signal_dbm.cmp(&a.signal_dbm)),
            SortBy::Ssid => items.sort_by(|a, b| a.ssid.to_lowercase().cmp(&b.ssid.to_lowercase())),
            SortBy::Channel => items.sort_by(|a, b| a.channel.cmp(&b.channel)),
        }

        items
    }
}

impl<'a> StatefulWidget for ApList<'a> {
    type State = ApListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        // Guard against zero-size areas
        if inner.height == 0 || inner.width < 10 {
            return;
        }

        let items = self.filtered_sorted();
        if items.is_empty() {
            if inner.width > 0 {
                buf.set_string(
                    inner.x,
                    inner.y,
                    "No access points found",
                    Style::default().fg(Color::DarkGray),
                );
            }
            return;
        }

        // Find best signal for highlighting
        let best_signal = items.iter().map(|ap| ap.signal_dbm).max();

        // Ensure selection is in bounds
        if state.selected >= items.len() {
            state.selected = items.len().saturating_sub(1);
        }

        // Calculate visible range
        let visible_height = inner.height as usize;
        if visible_height == 0 {
            return;
        }
        state.ensure_visible(visible_height);

        // Layout: SSID (variable) | Signal + Bar | CH | Band
        // Example: "MyNetwork       -45 ████████████████████████████ 36 5G"
        let ch_width: u16 = if self.show_channel { 4 } else { 0 }; // " 36 "
        let band_width: u16 = if self.show_band { 3 } else { 0 }; // "5G "
        let signal_width: u16 = 4; // "-45 "
        let min_bar_width: u16 = 10;
        let min_ssid_width: u16 = 8;

        // Calculate widths safely
        let suffix_width = ch_width + band_width;
        let fixed_width = signal_width + suffix_width + min_bar_width;
        let ssid_width = if inner.width > fixed_width + min_ssid_width {
            inner.width.saturating_sub(fixed_width + min_bar_width)
        } else {
            min_ssid_width.min(inner.width.saturating_sub(signal_width))
        };
        let bar_width = inner.width.saturating_sub(ssid_width + signal_width + suffix_width);

        for (i, ap) in items
            .iter()
            .skip(state.offset)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;

            // Bounds check for y coordinate
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = state.offset + i == state.selected;
            let is_best = self.highlight_best && Some(ap.signal_dbm) == best_signal;

            let base_style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Clear line (with bounds check)
            let line_end = (inner.x + inner.width).min(buf.area.right());
            for x in inner.x..line_end {
                buf.set_string(x, y, " ", base_style);
            }

            // SSID
            let ssid_display = if ap.ssid.is_empty() {
                "<hidden>".to_string()
            } else {
                truncate(&ap.ssid, ssid_width as usize)
            };
            buf.set_string(inner.x, y, &ssid_display, base_style);

            // Signal value
            let signal_x = inner.x.saturating_add(ssid_width);
            if signal_x < line_end {
                let signal_str = format!("{:>3} ", ap.signal_dbm);
                let signal_style = if is_best {
                    base_style.fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    base_style
                };
                buf.set_string(signal_x, y, &signal_str, signal_style);
            }

            // Signal bar
            let bar_x = signal_x.saturating_add(signal_width);
            if bar_x < line_end && bar_width > 0 {
                let filled = signal_bar_width(ap.signal_dbm, bar_width);
                let bar_color = signal_color(ap.signal_dbm);
                let bar_end = bar_x.saturating_add(bar_width).min(line_end);
                for x in bar_x..bar_end {
                    let j = x - bar_x;
                    let ch = if j < filled { '█' } else { ' ' };
                    let style = base_style.fg(bar_color);
                    buf.set_string(x, y, ch.to_string(), style);
                }
            }

            // Channel
            let mut next_x = bar_x.saturating_add(bar_width);
            if self.show_channel && next_x < line_end {
                let ch_str = format!("{:>3} ", ap.channel);
                buf.set_string(next_x, y, &ch_str, base_style);
                next_x = next_x.saturating_add(ch_width);
            }

            // Band
            if self.show_band && next_x < line_end {
                let band_str = format!("{}", ap.band().short_name());
                buf.set_string(next_x, y, &band_str, base_style);
            }
        }
    }
}
