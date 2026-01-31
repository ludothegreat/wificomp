use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

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

/// File picker state
#[derive(Debug, Default)]
pub struct FilePickerState {
    pub files: Vec<String>,
    pub selected: usize,
}

impl FilePickerState {
    pub fn select_next(&mut self) {
        if !self.files.is_empty() {
            self.selected = (self.selected + 1).min(self.files.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
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

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if self.state.files.is_empty() {
            buf.set_string(
                inner.x + 1,
                inner.y + inner.height / 2,
                "No sessions found",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let visible_height = inner.height.saturating_sub(2) as usize;
        let offset = if self.state.selected >= visible_height {
            self.state.selected - visible_height + 1
        } else {
            0
        };

        for (i, file) in self.state.files.iter().skip(offset).take(visible_height).enumerate() {
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
            let display = if file.len() > inner.width as usize - 4 {
                format!("{}...{}", prefix, &file[file.len() - inner.width as usize + 7..])
            } else {
                format!("{}{}", prefix, file)
            };
            buf.set_string(inner.x + 1, y, &display, style);
        }

        // Help text
        buf.set_string(
            inner.x + 1,
            inner.y + inner.height - 1,
            "[Enter] Select  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        );
    }
}
