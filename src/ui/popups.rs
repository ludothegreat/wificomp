use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::data::{AdapterDirInfo, SessionInfo};

/// Centered popup helper
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// A simple dialog popup
pub struct Dialog<'a> {
    title: &'a str,
    message: &'a str,
    options: &'a [&'a str],
    selected: usize,
}

impl<'a> Dialog<'a> {
    pub fn new(title: &'a str, message: &'a str, options: &'a [&'a str]) -> Self {
        Self {
            title,
            message,
            options,
            selected: 0,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a> Widget for Dialog<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate size
        let width = 50.min(area.width);
        let height = (4 + self.options.len() as u16 + 2).min(area.height);
        let popup_area = centered_rect(width, height, area);

        // Clear background
        Clear.render(popup_area, buf);

        // Draw block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Draw message
        let msg = Paragraph::new(self.message).alignment(Alignment::Center);
        let msg_area = Rect::new(inner.x, inner.y, inner.width, 2);
        msg.render(msg_area, buf);

        // Draw options
        for (i, option) in self.options.iter().enumerate() {
            let y = inner.y + 3 + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let style = if i == self.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if i == self.selected { "▶ " } else { "  " };
            let text = format!("{}{}", prefix, option);
            let x = inner.x + (inner.width.saturating_sub(text.len() as u16)) / 2;
            buf.set_string(x, y, &text, style);
        }
    }
}

/// Input popup for text entry
pub struct InputPopup<'a> {
    title: &'a str,
    prompt: &'a str,
    value: &'a str,
    cursor_pos: usize,
}

impl<'a> InputPopup<'a> {
    pub fn new(title: &'a str, prompt: &'a str, value: &'a str) -> Self {
        Self {
            title,
            prompt,
            value,
            cursor_pos: value.len(),
        }
    }

    pub fn cursor_pos(mut self, pos: usize) -> Self {
        self.cursor_pos = pos;
        self
    }
}

impl<'a> Widget for InputPopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 45.min(area.width);
        let height = 7.min(area.height);
        let popup_area = centered_rect(width, height, area);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Prompt
        buf.set_string(inner.x + 1, inner.y, self.prompt, Style::default());

        // Input field
        let input_y = inner.y + 2;
        let input_width = inner.width.saturating_sub(2);

        // Draw input box
        buf.set_string(
            inner.x + 1,
            input_y,
            "─".repeat(input_width as usize),
            Style::default().fg(Color::DarkGray),
        );

        // Draw value
        let display_value = if self.value.len() > input_width as usize - 2 {
            &self.value[self.value.len() - input_width as usize + 2..]
        } else {
            self.value
        };
        buf.set_string(
            inner.x + 1,
            input_y,
            display_value,
            Style::default().fg(Color::White),
        );

        // Cursor
        let cursor_x = inner.x + 1 + self.cursor_pos.min(input_width as usize - 1) as u16;
        buf.set_string(cursor_x, input_y, "▌", Style::default().fg(Color::Yellow));

        // Help
        buf.set_string(
            inner.x + 1,
            inner.y + inner.height - 1,
            "[Enter] OK  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        );
    }
}

/// File picker browsing level
#[derive(Debug, Clone, PartialEq)]
pub enum BrowseLevel {
    /// Showing adapter directories
    Adapters,
    /// Showing sessions in a specific adapter directory
    Sessions { adapter_path: PathBuf, adapter_name: String },
}

impl Default for BrowseLevel {
    fn default() -> Self {
        BrowseLevel::Adapters
    }
}

/// File picker state with two-level navigation
#[derive(Debug, Default)]
pub struct FilePickerState {
    /// Current browsing level
    pub level: BrowseLevel,
    /// Display strings for current level
    pub items: Vec<String>,
    /// Currently selected index
    pub selected: usize,
    /// Adapter directories (when at Adapters level)
    pub adapter_dirs: Vec<AdapterDirInfo>,
    /// Session infos (when at Sessions level)
    pub session_infos: Vec<SessionInfo>,
}

impl FilePickerState {
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1).min(self.items.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Check if we're at the adapter level
    pub fn is_at_adapters(&self) -> bool {
        matches!(self.level, BrowseLevel::Adapters)
    }

    /// Check if we're at the sessions level
    pub fn is_at_sessions(&self) -> bool {
        matches!(self.level, BrowseLevel::Sessions { .. })
    }

    /// Get the currently selected adapter directory (when at Adapters level)
    pub fn get_selected_adapter(&self) -> Option<&AdapterDirInfo> {
        if self.is_at_adapters() {
            self.adapter_dirs.get(self.selected)
        } else {
            None
        }
    }

    /// Get the currently selected session (when at Sessions level)
    pub fn get_selected_session(&self) -> Option<&SessionInfo> {
        if self.is_at_sessions() {
            self.session_infos.get(self.selected)
        } else {
            None
        }
    }

    /// Enter an adapter directory
    pub fn enter_adapter(&mut self, adapter: &AdapterDirInfo, sessions: Vec<SessionInfo>) {
        self.level = BrowseLevel::Sessions {
            adapter_path: adapter.path.clone(),
            adapter_name: adapter.name.clone(),
        };
        self.items = sessions.iter().map(|s| s.display_string()).collect();
        self.session_infos = sessions;
        self.selected = 0;
    }

    /// Go back to adapter list
    pub fn go_back(&mut self, adapters: Vec<AdapterDirInfo>) {
        self.level = BrowseLevel::Adapters;
        self.items = adapters.iter().map(|a| a.display_string()).collect();
        self.adapter_dirs = adapters;
        self.selected = 0;
    }

    /// Initialize with adapter list
    pub fn set_adapters(&mut self, adapters: Vec<AdapterDirInfo>) {
        self.level = BrowseLevel::Adapters;
        self.items = adapters.iter().map(|a| a.display_string()).collect();
        self.adapter_dirs = adapters;
        self.selected = 0;
    }

    /// Get current directory name for display
    pub fn current_dir_name(&self) -> Option<&str> {
        match &self.level {
            BrowseLevel::Adapters => None,
            BrowseLevel::Sessions { adapter_name, .. } => Some(adapter_name),
        }
    }
}

// Legacy compatibility
impl FilePickerState {
    #[allow(dead_code)]
    pub fn files(&self) -> &Vec<String> {
        &self.items
    }
}

/// File picker popup
pub struct FilePicker<'a> {
    title: &'a str,
    state: &'a FilePickerState,
}

impl<'a> FilePicker<'a> {
    pub fn new(title: &'a str, state: &'a FilePickerState) -> Self {
        Self { title, state }
    }
}

impl<'a> Widget for FilePicker<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 60.min(area.width);
        let height = 15.min(area.height);
        let popup_area = centered_rect(width, height, area);

        Clear.render(popup_area, buf);

        // Build title with current path
        let title = match &self.state.level {
            BrowseLevel::Adapters => format!(" {} ", self.title),
            BrowseLevel::Sessions { adapter_name, .. } => {
                format!(" {} > {} ", self.title, adapter_name)
            }
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if self.state.items.is_empty() {
            let msg = if self.state.is_at_adapters() {
                "No adapters found"
            } else {
                "No sessions found"
            };
            buf.set_string(
                inner.x + 1,
                inner.y + inner.height / 2,
                msg,
                Style::default().fg(Color::DarkGray),
            );

            // Still show help for going back
            if self.state.is_at_sessions() {
                buf.set_string(
                    inner.x + 1,
                    inner.y + inner.height - 1,
                    "[Bksp] Back  [Esc] Cancel",
                    Style::default().fg(Color::DarkGray),
                );
            }
            return;
        }

        let visible_height = inner.height.saturating_sub(2) as usize;
        let offset = if self.state.selected >= visible_height {
            self.state.selected - visible_height + 1
        } else {
            0
        };

        for (i, item) in self.state.items.iter().skip(offset).take(visible_height).enumerate() {
            let y = inner.y + i as u16;
            let is_selected = offset + i == self.state.selected;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let max_len = inner.width as usize - 4;
            let display = if item.len() > max_len {
                format!("{}{}...", prefix, &item[..max_len - 3])
            } else {
                format!("{}{}", prefix, item)
            };
            buf.set_string(inner.x + 1, y, &display, style);
        }

        // Help text depends on level
        let help = if self.state.is_at_adapters() {
            "[Enter] Open  [Esc] Cancel"
        } else {
            "[Enter] Load  [Bksp] Back  [Esc] Cancel"
        };
        buf.set_string(
            inner.x + 1,
            inner.y + inner.height - 1,
            help,
            Style::default().fg(Color::DarkGray),
        );
    }
}
