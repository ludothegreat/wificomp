use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

use super::models::Session;

/// Get the sessions directory path
pub fn sessions_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not find data directory")?
        .join("wificomp")
        .join("sessions");
    Ok(data_dir)
}

/// Ensure the sessions directory exists
pub fn ensure_sessions_dir() -> Result<PathBuf> {
    let dir = sessions_dir()?;
    fs::create_dir_all(&dir).context("Failed to create sessions directory")?;
    Ok(dir)
}

/// Generate a session filename from adapter
pub fn session_filename(adapter: &super::models::Adapter) -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let safe_name = adapter.safe_name();
    format!("{}_{}.json", safe_name, timestamp)
}

/// Save a session to disk
pub fn save_session(session: &Session) -> Result<PathBuf> {
    let dir = ensure_sessions_dir()?;
    let filename = session_filename(&session.adapter);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(session).context("Failed to serialize session")?;
    fs::write(&path, json).context("Failed to write session file")?;

    Ok(path)
}

/// Load a session from disk
pub fn load_session(path: &Path) -> Result<Session> {
    let contents = fs::read_to_string(path).context("Failed to read session file")?;
    let session: Session = serde_json::from_str(&contents).context("Failed to parse session file")?;
    Ok(session)
}

/// Session validation result
#[derive(Debug)]
pub struct SessionValidation {
    pub is_valid: bool,
    pub has_scans: bool,
    pub scan_count: usize,
    pub ap_count: usize,
    pub warnings: Vec<String>,
}

/// Validate a session for integrity
pub fn validate_session(session: &Session) -> SessionValidation {
    let mut warnings = Vec::new();

    let has_scans = !session.scans.is_empty();
    let scan_count = session.scans.len();

    // Count unique APs
    let ap_count = session.unique_aps().len();

    if !has_scans {
        warnings.push("Session has no scan data".to_string());
    }

    if session.adapter.interface.is_empty() {
        warnings.push("Session has no adapter interface".to_string());
    }

    // Check for scans with no APs
    let empty_scans = session.scans.iter().filter(|s| s.access_points.is_empty()).count();
    if empty_scans > 0 && empty_scans == scan_count {
        warnings.push("All scans are empty (no APs detected)".to_string());
    }

    let is_valid = has_scans && ap_count > 0;

    SessionValidation {
        is_valid,
        has_scans,
        scan_count,
        ap_count,
        warnings,
    }
}

/// Load and validate a session
pub fn load_session_validated(path: &Path) -> Result<(Session, SessionValidation)> {
    let session = load_session(path)?;
    let validation = validate_session(&session);
    Ok((session, validation))
}

/// List all saved sessions
pub fn list_sessions() -> Result<Vec<PathBuf>> {
    let dir = match sessions_dir() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<PathBuf> = fs::read_dir(&dir)
        .context("Failed to read sessions directory")?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();

    // Sort by modification time, newest first
    sessions.sort_by(|a, b| {
        let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
        let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    Ok(sessions)
}

/// Check if a session exists for the given adapter
pub fn find_existing_session(interface: &str) -> Result<Option<PathBuf>> {
    let sessions = list_sessions()?;
    for path in sessions {
        if let Ok(session) = load_session(&path) {
            if session.adapter.interface == interface {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

/// Session info for listing purposes
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub path: PathBuf,
    pub adapter_name: String,
    pub interface: String,
    pub chipset: String,
    pub label: Option<String>,
    pub started_at: String,
    pub scan_count: usize,
}

impl SessionInfo {
    pub fn from_path(path: &Path) -> Result<Self> {
        let session = load_session(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            adapter_name: session.adapter.display_name(),
            interface: session.adapter.interface,
            chipset: session.adapter.chipset,
            label: session.adapter.label,
            started_at: session.started_at.format("%m-%d %H:%M").to_string(),
            scan_count: session.scans.len(),
        })
    }

    /// Display string for file picker
    pub fn display_string(&self) -> String {
        format!(
            "{} ({}) - {} scans",
            self.adapter_name,
            self.started_at,
            self.scan_count
        )
    }
}

/// List sessions with info
pub fn list_session_infos() -> Result<Vec<SessionInfo>> {
    let paths = list_sessions()?;
    let mut infos = Vec::new();
    for path in paths {
        if let Ok(info) = SessionInfo::from_path(&path) {
            infos.push(info);
        }
    }
    Ok(infos)
}
