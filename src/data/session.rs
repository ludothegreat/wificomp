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

/// Ensure an adapter subdirectory exists
pub fn ensure_adapter_dir(adapter: &super::models::Adapter) -> Result<PathBuf> {
    let base_dir = ensure_sessions_dir()?;
    let adapter_dir = base_dir.join(adapter.safe_name());
    fs::create_dir_all(&adapter_dir).context("Failed to create adapter directory")?;
    Ok(adapter_dir)
}

/// Generate a session filename (without adapter prefix since it's in a subdirectory now)
pub fn session_filename() -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    format!("{}.json", timestamp)
}

/// Save a session to disk (in adapter subdirectory)
pub fn save_session(session: &Session) -> Result<PathBuf> {
    let adapter_dir = ensure_adapter_dir(&session.adapter)?;
    let filename = session_filename();
    let path = adapter_dir.join(&filename);

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

/// Adapter directory info
#[derive(Debug, Clone)]
pub struct AdapterDirInfo {
    pub path: PathBuf,
    pub name: String,
    pub session_count: usize,
}

impl AdapterDirInfo {
    pub fn display_string(&self) -> String {
        format!("📁 {} ({} sessions)", self.name, self.session_count)
    }
}

/// List all adapter directories
pub fn list_adapter_dirs() -> Result<Vec<AdapterDirInfo>> {
    let dir = match sessions_dir() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut adapters: Vec<AdapterDirInfo> = Vec::new();

    for entry in fs::read_dir(&dir).context("Failed to read sessions directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Count sessions in this directory
            let session_count = fs::read_dir(&path)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
                        .count()
                })
                .unwrap_or(0);

            if session_count > 0 {
                adapters.push(AdapterDirInfo {
                    path,
                    name,
                    session_count,
                });
            }
        }
    }

    // Sort by name
    adapters.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(adapters)
}

/// List sessions in a specific adapter directory
pub fn list_sessions_in_dir(adapter_dir: &Path) -> Result<Vec<PathBuf>> {
    if !adapter_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<PathBuf> = fs::read_dir(adapter_dir)
        .context("Failed to read adapter directory")?
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

/// List all saved sessions (legacy - scans all directories)
pub fn list_sessions() -> Result<Vec<PathBuf>> {
    let dir = match sessions_dir() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<PathBuf> = Vec::new();

    // Check for legacy sessions in root directory
    for entry in fs::read_dir(&dir).context("Failed to read sessions directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
            sessions.push(path);
        } else if path.is_dir() {
            // Scan subdirectory for sessions
            if let Ok(sub_sessions) = list_sessions_in_dir(&path) {
                sessions.extend(sub_sessions);
            }
        }
    }

    // Sort by modification time, newest first
    sessions.sort_by(|a, b| {
        let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
        let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    Ok(sessions)
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

    /// Display string for file picker (shorter, no adapter name since we're in adapter dir)
    pub fn display_string(&self) -> String {
        format!(
            "{} - {} scans",
            self.started_at,
            self.scan_count
        )
    }

    /// Full display string with adapter name
    pub fn display_string_full(&self) -> String {
        format!(
            "{} ({}) - {} scans",
            self.adapter_name,
            self.started_at,
            self.scan_count
        )
    }
}

/// List sessions with info from a specific adapter directory
pub fn list_session_infos_in_dir(adapter_dir: &Path) -> Result<Vec<SessionInfo>> {
    let paths = list_sessions_in_dir(adapter_dir)?;
    let mut infos = Vec::new();
    for path in paths {
        if let Ok(info) = SessionInfo::from_path(&path) {
            infos.push(info);
        }
    }
    Ok(infos)
}

/// List all sessions with info (legacy)
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
