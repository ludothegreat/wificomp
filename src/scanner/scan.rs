use anyhow::{Context, Result};
use chrono::Utc;
use std::process::Command;

use crate::data::{AccessPoint, ScanResult};

/// Perform a WiFi scan on the given interface
pub fn scan_wifi(interface: &str) -> Result<ScanResult> {
    // Check if we're already root
    let is_root = unsafe { libc::geteuid() } == 0;

    let output = if is_root {
        Command::new("iw")
            .args(["dev", interface, "scan"])
            .output()
    } else {
        Command::new("sudo")
            .args(["iw", "dev", interface, "scan"])
            .output()
    }
    .context("Failed to run 'iw scan'. Is iw installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Operation not permitted") {
            anyhow::bail!("Permission denied. Run with sudo or set CAP_NET_ADMIN capability.");
        } else if stderr.contains("Device or resource busy") {
            anyhow::bail!("Device busy. Another scan may be in progress.");
        } else {
            anyhow::bail!("Scan failed: {}", stderr);
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let access_points = parse_scan_output(&stdout);

    Ok(ScanResult {
        timestamp: Utc::now(),
        access_points,
    })
}

/// Parse the output of `iw dev <iface> scan`
fn parse_scan_output(output: &str) -> Vec<AccessPoint> {
    let mut aps = Vec::new();
    let mut current_ap: Option<AccessPointBuilder> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        // New BSS entry
        if trimmed.starts_with("BSS ") {
            // Save previous AP if valid
            if let Some(builder) = current_ap.take() {
                if let Some(ap) = builder.build() {
                    aps.push(ap);
                }
            }
            // Start new AP
            let bssid = trimmed
                .trim_start_matches("BSS ")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_uppercase();
            current_ap = Some(AccessPointBuilder::new(bssid));
        } else if let Some(ref mut builder) = current_ap {
            // Parse fields
            if let Some(signal) = trimmed.strip_prefix("signal: ") {
                // Format: "-45.00 dBm" or "-45 dBm"
                if let Some(dbm_str) = signal.split_whitespace().next() {
                    if let Ok(dbm) = dbm_str.parse::<f32>() {
                        builder.signal_dbm = Some(dbm.round() as i32);
                    }
                }
            } else if let Some(ssid) = trimmed.strip_prefix("SSID: ") {
                builder.ssid = Some(ssid.to_string());
            } else if let Some(freq) = trimmed.strip_prefix("freq: ") {
                // Frequency can be "2437" or "2437.0"
                if let Ok(f) = freq.parse::<f32>() {
                    builder.frequency_mhz = Some(f.round() as u32);
                }
            } else if let Some(ds) = trimmed.strip_prefix("DS Parameter set: channel ") {
                if let Ok(ch) = ds.parse::<u32>() {
                    builder.channel = Some(ch);
                }
            } else if trimmed.starts_with("* primary channel: ") {
                if let Some(ch_str) = trimmed.strip_prefix("* primary channel: ") {
                    if let Ok(ch) = ch_str.parse::<u32>() {
                        builder.channel = Some(ch);
                    }
                }
            }
        }
    }

    // Don't forget the last AP
    if let Some(builder) = current_ap {
        if let Some(ap) = builder.build() {
            aps.push(ap);
        }
    }

    aps
}

/// Helper to build AccessPoint
struct AccessPointBuilder {
    bssid: String,
    ssid: Option<String>,
    signal_dbm: Option<i32>,
    channel: Option<u32>,
    frequency_mhz: Option<u32>,
}

impl AccessPointBuilder {
    fn new(bssid: String) -> Self {
        Self {
            bssid,
            ssid: None,
            signal_dbm: None,
            channel: None,
            frequency_mhz: None,
        }
    }

    fn build(self) -> Option<AccessPoint> {
        let signal_dbm = self.signal_dbm?;
        let frequency_mhz = self.frequency_mhz?;
        let channel = self.channel.unwrap_or_else(|| freq_to_channel(frequency_mhz));

        Some(AccessPoint {
            bssid: self.bssid,
            ssid: self.ssid.unwrap_or_default(),
            signal_dbm,
            channel,
            frequency_mhz,
        })
    }
}

/// Convert frequency to channel number
fn freq_to_channel(freq_mhz: u32) -> u32 {
    match freq_mhz {
        // 2.4 GHz
        2412 => 1,
        2417 => 2,
        2422 => 3,
        2427 => 4,
        2432 => 5,
        2437 => 6,
        2442 => 7,
        2447 => 8,
        2452 => 9,
        2457 => 10,
        2462 => 11,
        2467 => 12,
        2472 => 13,
        2484 => 14,
        // 5 GHz (common channels)
        5180 => 36,
        5200 => 40,
        5220 => 44,
        5240 => 48,
        5260 => 52,
        5280 => 56,
        5300 => 60,
        5320 => 64,
        5500 => 100,
        5520 => 104,
        5540 => 108,
        5560 => 112,
        5580 => 116,
        5600 => 120,
        5620 => 124,
        5640 => 128,
        5660 => 132,
        5680 => 136,
        5700 => 140,
        5720 => 144,
        5745 => 149,
        5765 => 153,
        5785 => 157,
        5805 => 161,
        5825 => 165,
        // 6 GHz (some channels)
        5955 => 1,
        5975 => 5,
        5995 => 9,
        6015 => 13,
        // Default: calculate
        _ => {
            if freq_mhz < 3000 {
                // 2.4 GHz
                ((freq_mhz - 2407) / 5) as u32
            } else if freq_mhz < 5900 {
                // 5 GHz
                ((freq_mhz - 5000) / 5) as u32
            } else {
                // 6 GHz
                ((freq_mhz - 5950) / 5) as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scan_output() {
        // Test with decimal frequencies (as seen on real hardware)
        let output = r#"BSS aa:bb:cc:dd:ee:ff(on wlan0)
	last seen: 1234.567s [boottime]
	TSF: 1234567890 usec (0d, 00:20:34)
	freq: 5180.0
	beacon interval: 100 TUs
	capability: ESS Privacy ShortSlotTime RadioMeasure (0x1411)
	signal: -45.00 dBm
	last seen: 0 ms ago
	SSID: MyNetwork
	Supported rates: 6.0* 9.0 12.0* 18.0 24.0* 36.0 48.0 54.0
	DS Parameter set: channel 36
BSS 11:22:33:44:55:66(on wlan0)
	freq: 2437.0
	signal: -67.00 dBm
	SSID: OtherNetwork
	DS Parameter set: channel 6
"#;
        let aps = parse_scan_output(output);
        assert_eq!(aps.len(), 2);
        assert_eq!(aps[0].bssid, "AA:BB:CC:DD:EE:FF");
        assert_eq!(aps[0].ssid, "MyNetwork");
        assert_eq!(aps[0].signal_dbm, -45);
        assert_eq!(aps[0].channel, 36);
        assert_eq!(aps[0].frequency_mhz, 5180);

        assert_eq!(aps[1].bssid, "11:22:33:44:55:66");
        assert_eq!(aps[1].ssid, "OtherNetwork");
        assert_eq!(aps[1].signal_dbm, -67);
        assert_eq!(aps[1].channel, 6);
    }

    #[test]
    fn test_freq_to_channel() {
        assert_eq!(freq_to_channel(2412), 1);
        assert_eq!(freq_to_channel(2437), 6);
        assert_eq!(freq_to_channel(5180), 36);
        assert_eq!(freq_to_channel(5745), 149);
    }
}
