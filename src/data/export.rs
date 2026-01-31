use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::models::Session;

/// Export a session to JSON
pub fn export_json(session: &Session, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(session).context("Failed to serialize session")?;
    fs::write(path, json).context("Failed to write JSON file")?;
    Ok(())
}

/// Export a session to CSV
pub fn export_csv(session: &Session, path: &Path) -> Result<()> {
    let mut csv = String::new();

    // Header
    csv.push_str("timestamp,bssid,ssid,signal_dbm,channel,frequency_mhz,band\n");

    // Data rows
    for scan in &session.scans {
        let timestamp = scan.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        for ap in &scan.access_points {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                timestamp,
                ap.bssid,
                escape_csv(&ap.ssid),
                ap.signal_dbm,
                ap.channel,
                ap.frequency_mhz,
                ap.band().short_name()
            ));
        }
    }

    fs::write(path, csv).context("Failed to write CSV file")?;
    Ok(())
}

/// Export comparison results to CSV
pub fn export_comparison_csv(
    sessions: &[Session],
    ap_bssid: &str,
    ap_ssid: &str,
    path: &Path,
) -> Result<()> {
    let mut csv = String::new();

    // Header
    csv.push_str("adapter,interface,label,avg_signal,min_signal,max_signal,scan_count\n");

    // Data rows
    for session in sessions {
        if let Some(stats) = session.ap_stats(ap_bssid) {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                escape_csv(&session.adapter.chipset),
                session.adapter.interface,
                session.adapter.label.as_deref().unwrap_or(""),
                stats.avg,
                stats.min,
                stats.max,
                stats.count
            ));
        } else {
            csv.push_str(&format!(
                "{},{},{},N/A,N/A,N/A,0\n",
                escape_csv(&session.adapter.chipset),
                session.adapter.interface,
                session.adapter.label.as_deref().unwrap_or("")
            ));
        }
    }

    fs::write(path, csv).context("Failed to write CSV file")?;
    Ok(())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
