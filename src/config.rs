use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::data::{CompareMetric, FrequencyFilter, MatchBy, SortBy, TimerMode};

/// Excluded AP entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExcludedAp {
    pub bssid: String,
    pub ssid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auto_scan_interval")]
    pub auto_scan_interval_secs: u64,

    #[serde(default = "default_timer")]
    pub default_timer_secs: u64,

    #[serde(default)]
    pub timer_mode: TimerMode,

    #[serde(default = "default_true")]
    pub show_channel: bool,

    #[serde(default = "default_true")]
    pub show_band: bool,

    #[serde(default = "default_true")]
    pub highlight_best: bool,

    #[serde(default)]
    pub sort_by: SortBy,

    #[serde(default)]
    pub frequency_filter: FrequencyFilter,

    #[serde(default)]
    pub alert_threshold_dbm: Option<i32>,

    #[serde(default = "default_time_window")]
    pub history_time_window_mins: u64,

    #[serde(default)]
    pub history_show_average: bool,

    #[serde(default)]
    pub compare_match_by: MatchBy,

    #[serde(default)]
    pub compare_metric: CompareMetric,

    /// Permanently excluded APs (by BSSID)
    #[serde(default)]
    pub excluded_aps: Vec<ExcludedAp>,
}

fn default_auto_scan_interval() -> u64 {
    5
}

fn default_timer() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

fn default_time_window() -> u64 {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_scan_interval_secs: 5,
            default_timer_secs: 300,
            timer_mode: TimerMode::Countdown,
            show_channel: true,
            show_band: true,
            highlight_best: true,
            sort_by: SortBy::Signal,
            frequency_filter: FrequencyFilter::All,
            alert_threshold_dbm: None,
            history_time_window_mins: 5,
            history_show_average: false,
            compare_match_by: MatchBy::Bssid,
            compare_metric: CompareMetric::Avg,
            excluded_aps: Vec::new(),
        }
    }
}

impl Config {
    /// Get config file path
    pub fn path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("wificomp");
        Ok(config_dir.join("config.json"))
    }

    /// Load config from disk, or create default
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let contents = fs::read_to_string(&path).context("Failed to read config file")?;
            let config: Config =
                serde_json::from_str(&contents).context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let json = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, json).context("Failed to write config file")?;
        Ok(())
    }
}
