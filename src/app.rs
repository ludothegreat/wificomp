use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;

use crate::config::Config;
use crate::config::ExcludedAp;
use crate::data::{
    export, list_session_infos, load_session_validated, save_session, Adapter, ScanResult, Session,
    SessionInfo,
};
use crate::scanner::{detect_adapters, scan_wifi};
use crate::ui::popups::FilePickerState;
use crate::ui::{CompareState, HistoryState, LiveState};

/// Result from background scan thread
type ScanResultMsg = Result<ScanResult, String>;

/// Current screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Live,
    History,
    Compare,
}

/// Popup state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    None,
    AdapterCollision { selected: usize },
    RenameAdapter { input: String, cursor: usize },
    TimerSetup { input: String, cursor: usize },
    FilePicker,
    ExportChoice { selected: usize },
    Error { message: String },
    /// Confirm quit with unsaved data
    ConfirmQuit { selected: usize },
    /// Exclude AP options (session or permanent)
    ExcludeAp { bssid: String, ssid: String, selected: usize },
    /// Session has issues warning
    SessionWarning { message: String, path: std::path::PathBuf },
}

/// Main application state
pub struct App {
    pub running: bool,
    pub screen: Screen,
    pub popup: Popup,
    pub config: Config,

    // Screen states
    pub live: LiveState,
    pub history: HistoryState,
    pub compare: CompareState,

    // File picker state
    pub file_picker: FilePickerState,
    pub session_infos: Vec<SessionInfo>,

    // Current session
    pub current_session: Option<Session>,
    pub session_modified: bool,

    // Timing
    pub last_scan: Option<Instant>,
    pub session_start: Option<Instant>,

    // Background scan
    scan_receiver: Option<Receiver<ScanResultMsg>>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load().unwrap_or_default();

        let mut live = LiveState::default();
        live.auto_scan_interval = config.auto_scan_interval_secs;
        live.timer_target_secs = Some(config.default_timer_secs);
        live.show_channel = config.show_channel;
        live.show_band = config.show_band;
        live.highlight_best = config.highlight_best;
        live.frequency_filter = config.frequency_filter;
        live.sort_by = config.sort_by;

        let mut history = HistoryState::default();
        history.time_window_mins = config.history_time_window_mins;
        history.show_average = config.history_show_average;

        let mut compare = CompareState::default();
        compare.match_by = config.compare_match_by;
        compare.metric = config.compare_metric;

        Ok(Self {
            running: true,
            screen: Screen::Live,
            popup: Popup::None,
            config,
            live,
            history,
            compare,
            file_picker: FilePickerState::default(),
            session_infos: Vec::new(),
            current_session: None,
            session_modified: false,
            last_scan: None,
            session_start: None,
            scan_receiver: None,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        // Detect adapters
        match detect_adapters() {
            Ok(adapters) => {
                if let Some(adapter) = adapters.into_iter().next() {
                    self.set_adapter(adapter);
                }
            }
            Err(e) => {
                self.show_error(format!("Failed to detect adapters: {}", e));
            }
        }
        Ok(())
    }

    fn set_adapter(&mut self, adapter: Adapter) {
        self.live.adapter = Some(adapter.clone());

        // Create new session
        let duration = self.live.timer_target_secs.map(Duration::from_secs);
        self.current_session = Some(Session::new(adapter, duration));
        self.session_start = Some(Instant::now());
        self.session_modified = false;
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.popup = Popup::None;

        // Load session into history view when switching
        if screen == Screen::History {
            if let Some(session) = &self.current_session {
                self.history.session = Some(session.clone());
            }
        }
    }

    pub fn tick(&mut self) {
        // Update elapsed time
        if let Some(start) = self.session_start {
            self.live.elapsed_secs = start.elapsed().as_secs();
        }

        // Check for scan results from background thread
        if let Some(receiver) = &self.scan_receiver {
            match receiver.try_recv() {
                Ok(Ok(result)) => {
                    self.live.access_points = result.access_points.clone();
                    self.live.last_scan_error = None;

                    // Add to session
                    if let Some(session) = &mut self.current_session {
                        session.add_scan(result);
                        self.session_modified = true;
                    }

                    self.last_scan = Some(Instant::now());
                    self.live.scanning = false;
                    self.scan_receiver = None;
                }
                Ok(Err(e)) => {
                    self.live.last_scan_error = Some(e);
                    self.live.scanning = false;
                    self.scan_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still scanning, do nothing
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.live.last_scan_error = Some("Scan thread crashed".to_string());
                    self.live.scanning = false;
                    self.scan_receiver = None;
                }
            }
        }

        // Check for auto-scan
        if self.live.auto_scan && self.screen == Screen::Live && self.popup == Popup::None {
            let should_scan = match self.last_scan {
                Some(last) => last.elapsed().as_secs() >= self.live.auto_scan_interval,
                None => true,
            };

            if should_scan && !self.live.scanning {
                self.perform_scan();
            }
        }
    }

    pub fn perform_scan(&mut self) {
        // Don't start a new scan if one is already in progress
        if self.live.scanning {
            return;
        }

        let Some(adapter) = &self.live.adapter else {
            return;
        };

        self.live.scanning = true;
        self.live.last_scan_error = None;

        // Spawn background thread for scanning
        let (tx, rx): (Sender<ScanResultMsg>, Receiver<ScanResultMsg>) = mpsc::channel();
        let interface = adapter.interface.clone();

        thread::spawn(move || {
            let result = scan_wifi(&interface).map_err(|e| e.to_string());
            let _ = tx.send(result);
        });

        self.scan_receiver = Some(rx);
    }

    pub fn save_current_session(&mut self) -> Result<PathBuf> {
        let session = self
            .current_session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session to save"))?;

        let path = save_session(session)?;
        self.session_modified = false;
        Ok(path)
    }

    pub fn load_session_file(&mut self, path: &PathBuf) -> Result<()> {
        let (session, validation) = load_session_validated(path)?;

        // Show warning if session has issues but still load it
        if !validation.warnings.is_empty() {
            let warning_msg = format!(
                "Session loaded with warnings:\n{}",
                validation.warnings.join("\n")
            );
            // We'll show this after loading
            self.popup = Popup::SessionWarning {
                message: warning_msg,
                path: path.clone(),
            };
        }

        match self.screen {
            Screen::History => {
                self.history.session = Some(session);
            }
            Screen::Compare => {
                self.compare.add_session(session);
                // Ensure visibility after adding
                let len = self.compare.sessions.len();
                self.compare.selected_session_idx = len.saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn refresh_session_list(&mut self) -> Result<()> {
        self.session_infos = list_session_infos()?;
        self.file_picker.files = self
            .session_infos
            .iter()
            .map(|info| info.display_string())
            .collect();
        self.file_picker.selected = 0;
        Ok(())
    }

    pub fn get_selected_session_path(&self) -> Option<PathBuf> {
        self.session_infos
            .get(self.file_picker.selected)
            .map(|info| info.path.clone())
    }

    pub fn show_file_picker(&mut self) {
        if let Err(e) = self.refresh_session_list() {
            self.show_error(format!("Failed to list sessions: {}", e));
            return;
        }
        self.popup = Popup::FilePicker;
    }

    pub fn show_error(&mut self, message: String) {
        self.popup = Popup::Error { message };
    }

    pub fn show_rename_popup(&mut self) {
        let current = self
            .live
            .adapter
            .as_ref()
            .and_then(|a| a.label.clone())
            .unwrap_or_default();
        self.popup = Popup::RenameAdapter {
            input: current.clone(),
            cursor: current.len(),
        };
    }

    pub fn show_timer_popup(&mut self) {
        let current = self
            .live
            .timer_target_secs
            .map(|s| (s / 60).to_string())
            .unwrap_or_default();
        self.popup = Popup::TimerSetup {
            input: current.clone(),
            cursor: current.len(),
        };
    }

    pub fn apply_rename(&mut self, name: String) {
        if let Some(adapter) = &mut self.live.adapter {
            adapter.label = if name.is_empty() { None } else { Some(name.clone()) };
        }
        if let Some(session) = &mut self.current_session {
            session.adapter.label = if name.is_empty() { None } else { Some(name) };
        }
        self.popup = Popup::None;
    }

    pub fn apply_timer(&mut self, mins_str: String) {
        if let Ok(mins) = mins_str.parse::<u64>() {
            self.live.timer_target_secs = if mins == 0 { None } else { Some(mins * 60) };
            if let Some(session) = &mut self.current_session {
                session.duration_target_secs = self.live.timer_target_secs;
            }
        }
        self.popup = Popup::None;
    }

    pub fn export_current(&mut self, csv: bool) -> Result<PathBuf> {
        let session = match self.screen {
            Screen::History => self.history.session.as_ref(),
            _ => self.current_session.as_ref(),
        };

        let session = session.ok_or_else(|| anyhow::anyhow!("No session to export"))?;

        let ext = if csv { "csv" } else { "json" };
        let filename = format!(
            "wificomp_export_{}.{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            ext
        );
        let path = PathBuf::from(&filename);

        if csv {
            export::export_csv(session, &path)?;
        } else {
            export::export_json(session, &path)?;
        }

        Ok(path)
    }

    pub fn save_config(&self) -> Result<()> {
        let mut config = self.config.clone();
        config.auto_scan_interval_secs = self.live.auto_scan_interval;
        config.default_timer_secs = self.live.timer_target_secs.unwrap_or(300);
        config.show_channel = self.live.show_channel;
        config.show_band = self.live.show_band;
        config.highlight_best = self.live.highlight_best;
        config.frequency_filter = self.live.frequency_filter;
        config.sort_by = self.live.sort_by;
        config.history_time_window_mins = self.history.time_window_mins;
        config.history_show_average = self.history.show_average;
        config.compare_match_by = self.compare.match_by;
        config.compare_metric = self.compare.metric;
        config.save()?;
        Ok(())
    }

    /// Request quit - shows confirmation if there's unsaved data or active scan
    pub fn request_quit(&mut self) {
        if self.live.scanning || self.session_modified {
            self.popup = Popup::ConfirmQuit { selected: 0 };
        } else {
            self.force_quit();
        }
    }

    /// Force quit without confirmation
    pub fn force_quit(&mut self) {
        // Save session if modified
        if self.session_modified {
            if let Err(e) = self.save_current_session() {
                eprintln!("Warning: Failed to save session: {}", e);
            }
        }

        // Save config
        if let Err(e) = self.save_config() {
            eprintln!("Warning: Failed to save config: {}", e);
        }

        self.running = false;
    }

    /// Quit without saving session
    pub fn quit_no_save(&mut self) {
        // Save config only
        if let Err(e) = self.save_config() {
            eprintln!("Warning: Failed to save config: {}", e);
        }
        self.running = false;
    }

    /// Show exclude AP popup for the selected AP
    pub fn show_exclude_popup(&mut self) {
        if let Some(ap) = self.live.get_selected_ap() {
            self.popup = Popup::ExcludeAp {
                bssid: ap.bssid.clone(),
                ssid: ap.ssid.clone(),
                selected: 0,
            };
        }
    }

    /// Exclude AP for this session only
    pub fn exclude_session(&mut self, bssid: &str) {
        self.live.exclude_session(bssid);
        self.popup = Popup::None;
    }

    /// Exclude AP permanently (add to config)
    pub fn exclude_permanent(&mut self, bssid: &str, ssid: &str) {
        self.config.excluded_aps.push(ExcludedAp {
            bssid: bssid.to_string(),
            ssid: ssid.to_string(),
        });
        self.live.exclude_session(bssid);
        self.popup = Popup::None;
    }

    /// Check if AP is permanently excluded
    pub fn is_permanently_excluded(&self, bssid: &str) -> bool {
        self.config.excluded_aps.iter().any(|ap| ap.bssid == bssid)
    }
}
