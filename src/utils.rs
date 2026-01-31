use std::time::Duration;

/// Format duration as MM:SS
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Format duration as MM:SS/MM:SS for countdown display
pub fn format_timer(elapsed: Duration, target: Option<Duration>) -> String {
    match target {
        Some(t) => {
            let remaining = t.saturating_sub(elapsed);
            format!("{}/{}", format_duration(remaining), format_duration(t))
        }
        None => format_duration(elapsed),
    }
}

/// Calculate signal bar width (max_width is the full bar width for best signal)
pub fn signal_bar_width(signal_dbm: i32, max_width: u16) -> u16 {
    // Map -100 to 0%, -30 to 100%
    let clamped = signal_dbm.clamp(-100, -30);
    let percent = (clamped + 100) as f32 / 70.0;
    (percent * max_width as f32).round() as u16
}

/// Get signal color based on dBm
pub fn signal_color(signal_dbm: i32) -> ratatui::style::Color {
    use ratatui::style::Color;
    if signal_dbm >= -50 {
        Color::Green
    } else if signal_dbm >= -60 {
        Color::LightGreen
    } else if signal_dbm >= -70 {
        Color::Yellow
    } else if signal_dbm >= -80 {
        Color::LightRed
    } else {
        Color::Red
    }
}

/// Truncate string with ellipsis if too long
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(format_duration(Duration::from_secs(65)), "01:05");
        assert_eq!(format_duration(Duration::from_secs(300)), "05:00");
    }

    #[test]
    fn test_signal_bar_width() {
        assert_eq!(signal_bar_width(-30, 28), 28);
        assert_eq!(signal_bar_width(-100, 28), 0);
        assert_eq!(signal_bar_width(-65, 28), 14);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }
}
