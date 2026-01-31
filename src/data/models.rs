use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WiFi frequency band
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Band {
    TwoPointFourGHz,
    FiveGHz,
    SixGHz,
}

impl Band {
    pub fn from_frequency(freq_mhz: u32) -> Self {
        if freq_mhz < 3000 {
            Band::TwoPointFourGHz
        } else if freq_mhz < 5900 {
            Band::FiveGHz
        } else {
            Band::SixGHz
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Band::TwoPointFourGHz => "2G",
            Band::FiveGHz => "5G",
            Band::SixGHz => "6G",
        }
    }
}

/// WiFi adapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adapter {
    pub interface: String,
    pub driver: String,
    pub chipset: String,
    pub label: Option<String>,
}

impl Adapter {
    /// Display name for UI - prioritizes label, then chipset
    pub fn display_name(&self) -> String {
        if let Some(label) = &self.label {
            label.clone()
        } else if !self.chipset.is_empty() && self.chipset != "unknown" {
            self.chipset.clone()
        } else {
            self.interface.clone()
        }
    }

    /// Full display with interface info
    pub fn display_name_full(&self) -> String {
        let name = self.display_name();
        if let Some(label) = &self.label {
            format!("\"{}\" ({})", label, self.interface)
        } else {
            format!("{} ({})", name, self.interface)
        }
    }

    /// Safe name for filenames
    pub fn safe_name(&self) -> String {
        let name = self.display_name();
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }
}

/// Single access point reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPoint {
    pub bssid: String,
    pub ssid: String,
    pub signal_dbm: i32,
    pub channel: u32,
    pub frequency_mhz: u32,
}

impl AccessPoint {
    pub fn band(&self) -> Band {
        Band::from_frequency(self.frequency_mhz)
    }

    /// Calculate signal strength as percentage (0-100)
    /// Maps -100 dBm to 0% and -30 dBm to 100%
    pub fn signal_percent(&self) -> u8 {
        let clamped = self.signal_dbm.clamp(-100, -30);
        ((clamped + 100) as f32 / 70.0 * 100.0) as u8
    }
}

/// Single scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub timestamp: DateTime<Utc>,
    pub access_points: Vec<AccessPoint>,
}

/// Sort options for AP list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortBy {
    #[default]
    Signal,
    Ssid,
    Channel,
}

impl SortBy {
    pub fn next(&self) -> Self {
        match self {
            SortBy::Signal => SortBy::Ssid,
            SortBy::Ssid => SortBy::Channel,
            SortBy::Channel => SortBy::Signal,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SortBy::Signal => "signal",
            SortBy::Ssid => "ssid",
            SortBy::Channel => "channel",
        }
    }
}

/// Frequency filter for AP list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FrequencyFilter {
    #[default]
    All,
    TwoPointFourGHz,
    FiveGHz,
    SixGHz,
}

impl FrequencyFilter {
    pub fn next(&self) -> Self {
        match self {
            FrequencyFilter::All => FrequencyFilter::TwoPointFourGHz,
            FrequencyFilter::TwoPointFourGHz => FrequencyFilter::FiveGHz,
            FrequencyFilter::FiveGHz => FrequencyFilter::SixGHz,
            FrequencyFilter::SixGHz => FrequencyFilter::All,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FrequencyFilter::All => "All",
            FrequencyFilter::TwoPointFourGHz => "2.4G",
            FrequencyFilter::FiveGHz => "5G",
            FrequencyFilter::SixGHz => "6G",
        }
    }

    pub fn matches(&self, band: Band) -> bool {
        match self {
            FrequencyFilter::All => true,
            FrequencyFilter::TwoPointFourGHz => band == Band::TwoPointFourGHz,
            FrequencyFilter::FiveGHz => band == Band::FiveGHz,
            FrequencyFilter::SixGHz => band == Band::SixGHz,
        }
    }
}

/// Timer mode for sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimerMode {
    #[default]
    Countdown,
    Elapsed,
}

/// Compare match mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MatchBy {
    #[default]
    Bssid,
    Ssid,
    Both,
}

impl MatchBy {
    pub fn next(&self) -> Self {
        match self {
            MatchBy::Bssid => MatchBy::Ssid,
            MatchBy::Ssid => MatchBy::Both,
            MatchBy::Both => MatchBy::Bssid,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MatchBy::Bssid => "BSSID",
            MatchBy::Ssid => "SSID",
            MatchBy::Both => "Both",
        }
    }
}

/// Compare metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompareMetric {
    #[default]
    Avg,
    Min,
    Max,
}

impl CompareMetric {
    pub fn next(&self) -> Self {
        match self {
            CompareMetric::Avg => CompareMetric::Min,
            CompareMetric::Min => CompareMetric::Max,
            CompareMetric::Max => CompareMetric::Avg,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CompareMetric::Avg => "Avg",
            CompareMetric::Min => "Min",
            CompareMetric::Max => "Max",
        }
    }
}

/// Complete session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    #[serde(default = "default_version")]
    pub version: String,
    pub adapter: Adapter,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_target_secs: Option<u64>,
    pub scans: Vec<ScanResult>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Session {
    pub fn new(adapter: Adapter, duration_target: Option<Duration>) -> Self {
        Self {
            version: "1.0".to_string(),
            adapter,
            started_at: Utc::now(),
            duration_target_secs: duration_target.map(|d| d.as_secs()),
            scans: Vec::new(),
        }
    }

    pub fn add_scan(&mut self, scan: ScanResult) {
        self.scans.push(scan);
    }

    pub fn duration_target(&self) -> Option<Duration> {
        self.duration_target_secs.map(Duration::from_secs)
    }

    pub fn elapsed(&self) -> Duration {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.started_at);
        Duration::from_secs(elapsed.num_seconds().max(0) as u64)
    }

    /// Get all unique APs seen in this session
    pub fn unique_aps(&self) -> Vec<(String, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut aps = Vec::new();
        for scan in &self.scans {
            for ap in &scan.access_points {
                let key = (ap.bssid.clone(), ap.ssid.clone());
                if seen.insert(key.clone()) {
                    aps.push(key);
                }
            }
        }
        aps
    }

    /// Get signal statistics for a specific AP
    pub fn ap_stats(&self, bssid: &str) -> Option<ApStats> {
        let signals: Vec<i32> = self
            .scans
            .iter()
            .flat_map(|s| s.access_points.iter())
            .filter(|ap| ap.bssid == bssid)
            .map(|ap| ap.signal_dbm)
            .collect();

        if signals.is_empty() {
            return None;
        }

        let sum: i32 = signals.iter().sum();
        let avg = sum as f32 / signals.len() as f32;
        let min = *signals.iter().min().unwrap();
        let max = *signals.iter().max().unwrap();

        Some(ApStats {
            avg: avg.round() as i32,
            min,
            max,
            count: signals.len(),
        })
    }
}

/// Statistics for an access point
#[derive(Debug, Clone)]
pub struct ApStats {
    pub avg: i32,
    pub min: i32,
    pub max: i32,
    pub count: usize,
}

impl ApStats {
    pub fn get(&self, metric: CompareMetric) -> i32 {
        match metric {
            CompareMetric::Avg => self.avg,
            CompareMetric::Min => self.min,
            CompareMetric::Max => self.max,
        }
    }
}
