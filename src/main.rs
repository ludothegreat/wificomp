mod app;
mod config;
mod data;
mod scanner;
mod ui;
mod utils;

use std::io;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
    Frame, Terminal,
};

use app::{App, Popup, Screen};
use ui::popups::{Dialog, FilePicker, InputPopup};
use ui::{CompareScreen, HistoryScreen, LiveScreen};

#[derive(Parser)]
#[command(name = "wificomp")]
#[command(about = "WiFi adapter comparison tool")]
#[command(version)]
struct Cli {
    /// Interface to use (auto-detects if not specified)
    #[arg(short, long)]
    interface: Option<String>,

    /// Disable auto-scan on startup
    #[arg(long)]
    no_auto_scan: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    if cli.no_auto_scan {
        app.live.auto_scan = false;
    }

    // Initialize
    if let Err(e) = app.init() {
        // Cleanup before showing error
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        return Err(e);
    }

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
        return Err(e);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);

    while app.running {
        terminal.draw(|f| draw(f, app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key.code, key.modifiers);
            }
        }

        app.tick();
    }

    Ok(())
}

fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Check minimum size
    if size.width < 60 || size.height < 15 {
        let msg = format!(
            "Terminal too small\nNeed: 60x15\nHave: {}x{}",
            size.width, size.height
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" wificomp ");
        let inner = block.inner(size);
        f.render_widget(block, size);
        f.render_widget(
            ratatui::widgets::Paragraph::new(msg)
                .style(Style::default().fg(Color::Red)),
            inner,
        );
        return;
    }

    // Main layout
    let chunks = Layout::vertical([
        Constraint::Length(1), // Tab bar
        Constraint::Min(10),   // Content
    ])
    .split(size);

    // Tab bar
    draw_tabs(f, app, chunks[0]);

    // Content
    let content_area = chunks[1];
    match app.screen {
        Screen::Live => {
            f.render_widget(LiveScreen::new(&app.live), content_area);
        }
        Screen::History => {
            f.render_widget(HistoryScreen::new(&app.history), content_area);
        }
        Screen::Compare => {
            f.render_widget(CompareScreen::new(&app.compare), content_area);
        }
    }

    // Popups
    draw_popup(f, app, size);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["[1]Live", "[2]Hist", "[3]Cmp"];
    let selected = match app.screen {
        Screen::Live => 0,
        Screen::History => 1,
        Screen::Compare => 2,
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    // Render with title
    let title = " wificomp ";
    let block = Block::default().title(title);

    // Calculate position for tabs (after title)
    let title_len = title.len() as u16;
    let tabs_area = Rect::new(
        area.x + title_len,
        area.y,
        area.width.saturating_sub(title_len),
        1,
    );

    f.render_widget(block, area);
    f.render_widget(tabs, tabs_area);
}

fn draw_popup(f: &mut Frame, app: &App, area: Rect) {
    match &app.popup {
        Popup::None => {}
        Popup::AdapterCollision { selected } => {
            let dialog = Dialog::new(
                "Adapter Already Used",
                "A session exists for this adapter.",
                &["Overwrite", "Append", "New Session"],
            )
            .selected(*selected);
            f.render_widget(dialog, area);
        }
        Popup::RenameAdapter { input, cursor } => {
            let popup = InputPopup::new("Rename Adapter", "Enter label:", input)
                .cursor_pos(*cursor);
            f.render_widget(popup, area);
        }
        Popup::TimerSetup { input, cursor } => {
            let popup = InputPopup::new("Set Timer", "Duration (minutes, 0=off):", input)
                .cursor_pos(*cursor);
            f.render_widget(popup, area);
        }
        Popup::FilePicker => {
            let picker = FilePicker::new("Load Session", &app.file_picker);
            f.render_widget(picker, area);
        }
        Popup::ExportChoice { selected } => {
            let dialog = Dialog::new("Export Format", "Choose export format:", &["JSON", "CSV"])
                .selected(*selected);
            f.render_widget(dialog, area);
        }
        Popup::Error { message } => {
            let dialog = Dialog::new("Error", message, &["OK"]);
            f.render_widget(dialog, area);
        }
        Popup::ConfirmQuit { selected } => {
            let msg = if app.live.scanning {
                "Scan in progress. Quit anyway?"
            } else {
                "Unsaved session data. Quit anyway?"
            };
            let dialog = Dialog::new("Confirm Quit", msg, &["Save & Quit", "Quit Without Save", "Cancel"])
                .selected(*selected);
            f.render_widget(dialog, area);
        }
        Popup::ExcludeAp { ssid, selected, .. } => {
            let msg = format!("Exclude '{}'?", if ssid.is_empty() { "<hidden>" } else { ssid });
            let dialog = Dialog::new("Exclude AP", &msg, &["This Session", "Permanently", "Cancel"])
                .selected(*selected);
            f.render_widget(dialog, area);
        }
        Popup::SessionWarning { message, .. } => {
            let dialog = Dialog::new("Warning", message, &["OK"]);
            f.render_widget(dialog, area);
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Handle popups first
    match &mut app.popup {
        Popup::None => {}
        Popup::AdapterCollision { selected } => {
            match code {
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(2),
                KeyCode::Enter => {
                    // Handle selection
                    app.popup = Popup::None;
                }
                KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::RenameAdapter { input, cursor } => {
            match code {
                KeyCode::Char(c) => {
                    input.insert(*cursor, c);
                    *cursor += 1;
                }
                KeyCode::Backspace => {
                    if *cursor > 0 {
                        *cursor -= 1;
                        input.remove(*cursor);
                    }
                }
                KeyCode::Left => *cursor = cursor.saturating_sub(1),
                KeyCode::Right => *cursor = (*cursor + 1).min(input.len()),
                KeyCode::Enter => {
                    let name = input.clone();
                    app.apply_rename(name);
                }
                KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::TimerSetup { input, cursor } => {
            match code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    input.insert(*cursor, c);
                    *cursor += 1;
                }
                KeyCode::Backspace => {
                    if *cursor > 0 {
                        *cursor -= 1;
                        input.remove(*cursor);
                    }
                }
                KeyCode::Left => *cursor = cursor.saturating_sub(1),
                KeyCode::Right => *cursor = (*cursor + 1).min(input.len()),
                KeyCode::Enter => {
                    let mins = input.clone();
                    app.apply_timer(mins);
                }
                KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::FilePicker => {
            match code {
                KeyCode::Up => app.file_picker.select_prev(),
                KeyCode::Down => app.file_picker.select_next(),
                KeyCode::Enter => {
                    if app.file_picker.is_at_adapters() {
                        // Enter adapter directory
                        if let Err(e) = app.file_picker_enter_adapter() {
                            app.show_error(format!("Failed to open adapter: {}", e));
                        }
                    } else {
                        // Load selected session
                        if let Some(path) = app.get_selected_session_path() {
                            if let Err(e) = app.load_session_file(&path) {
                                app.show_error(format!("Failed to load: {}", e));
                            } else {
                                app.popup = Popup::None;
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    // Go back to adapter list
                    if app.file_picker.is_at_sessions() {
                        if let Err(e) = app.file_picker_go_back() {
                            app.show_error(format!("Failed to go back: {}", e));
                        }
                    }
                }
                KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::ExportChoice { selected } => {
            match code {
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(1),
                KeyCode::Enter => {
                    let csv = *selected == 1;
                    match app.export_current(csv) {
                        Ok(path) => {
                            app.popup = Popup::None;
                            app.show_error(format!("Exported to {}", path.display()));
                        }
                        Err(e) => {
                            app.show_error(format!("Export failed: {}", e));
                        }
                    }
                }
                KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::Error { .. } => {
            match code {
                KeyCode::Enter | KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
        Popup::ConfirmQuit { selected } => {
            let sel = *selected;
            match code {
                KeyCode::Up => {
                    app.popup = Popup::ConfirmQuit { selected: sel.saturating_sub(1) };
                }
                KeyCode::Down => {
                    app.popup = Popup::ConfirmQuit { selected: (sel + 1).min(2) };
                }
                KeyCode::Char('1') => app.force_quit(),
                KeyCode::Char('2') => app.quit_no_save(),
                KeyCode::Char('3') | KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Enter => match sel {
                    0 => app.force_quit(),
                    1 => app.quit_no_save(),
                    _ => app.popup = Popup::None,
                },
                _ => {}
            }
            return;
        }
        Popup::ExcludeAp { bssid, ssid, selected } => {
            let b = bssid.clone();
            let s = ssid.clone();
            let sel = *selected;
            match code {
                KeyCode::Up => {
                    app.popup = Popup::ExcludeAp { bssid: b, ssid: s, selected: sel.saturating_sub(1) };
                }
                KeyCode::Down => {
                    app.popup = Popup::ExcludeAp { bssid: b, ssid: s, selected: (sel + 1).min(2) };
                }
                KeyCode::Char('1') => app.exclude_session(&b),
                KeyCode::Char('2') => app.exclude_permanent(&b, &s),
                KeyCode::Char('3') | KeyCode::Esc => app.popup = Popup::None,
                KeyCode::Enter => match sel {
                    0 => app.exclude_session(&b),
                    1 => app.exclude_permanent(&b, &s),
                    _ => app.popup = Popup::None,
                },
                _ => {}
            }
            return;
        }
        Popup::SessionWarning { .. } => {
            match code {
                KeyCode::Enter | KeyCode::Esc => app.popup = Popup::None,
                _ => {}
            }
            return;
        }
    }

    // Global keys
    match code {
        KeyCode::Char('q') => app.request_quit(),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.request_quit(),
        KeyCode::Char('1') => app.switch_screen(Screen::Live),
        KeyCode::Char('2') => app.switch_screen(Screen::History),
        KeyCode::Char('3') => app.switch_screen(Screen::Compare),
        _ => {
            // Screen-specific keys
            match app.screen {
                Screen::Live => handle_live_key(app, code),
                Screen::History => handle_history_key(app, code),
                Screen::Compare => handle_compare_key(app, code),
            }
        }
    }
}

fn handle_live_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(' ') => app.perform_scan(),
        KeyCode::Char('a') => app.live.toggle_auto_scan(),
        KeyCode::Char('t') => app.show_timer_popup(),
        KeyCode::Char('r') => app.show_rename_popup(),
        KeyCode::Char('c') => app.live.toggle_channel(),
        KeyCode::Char('b') => app.live.toggle_band(),
        KeyCode::Char('f') => app.live.cycle_filter(),
        KeyCode::Char('s') => app.live.cycle_sort(),
        KeyCode::Char('h') => app.live.toggle_highlight(),
        KeyCode::Char('x') => app.show_exclude_popup(),
        KeyCode::Char('e') => app.popup = Popup::ExportChoice { selected: 0 },
        KeyCode::Up => app.live.ap_list_state.select_prev(),
        KeyCode::Down => {
            let len = app.live.access_points.len();
            app.live.ap_list_state.select_next(len);
        }
        _ => {}
    }
}

fn handle_history_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('l') | KeyCode::Char('+') => app.show_file_picker(),
        KeyCode::Char('w') => app.history.cycle_time_window(),
        KeyCode::Char('d') => app.history.toggle_average(),
        KeyCode::Char('e') => app.popup = Popup::ExportChoice { selected: 0 },
        KeyCode::Up => app.history.select_prev_ap(),
        KeyCode::Down => app.history.select_next_ap(),
        _ => {}
    }
}

fn handle_compare_key(app: &mut App, code: KeyCode) {
    // Visible height for session list (approximate, actual may vary with terminal size)
    // The render function uses 4-6 based on terminal height
    const SESSION_LIST_HEIGHT: usize = 6;

    match code {
        KeyCode::Char('+') => app.show_file_picker(),
        KeyCode::Char('x') => app.compare.remove_selected_session(),
        KeyCode::Char('m') => app.compare.cycle_match(),
        KeyCode::Char('M') => app.compare.cycle_metric(),
        KeyCode::Char('e') => app.popup = Popup::ExportChoice { selected: 0 },
        KeyCode::Up => app.compare.select_prev_ap(),
        KeyCode::Down => app.compare.select_next_ap(),
        KeyCode::Left => {
            app.compare.select_prev_session();
            app.compare.ensure_session_visible(SESSION_LIST_HEIGHT);
        }
        KeyCode::Right => {
            app.compare.select_next_session();
            app.compare.ensure_session_visible(SESSION_LIST_HEIGHT);
        }
        _ => {}
    }
}
