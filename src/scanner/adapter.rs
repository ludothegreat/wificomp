use anyhow::{Context, Result};
use std::process::Command;

use crate::data::Adapter;

/// Detect available wireless adapters
pub fn detect_adapters() -> Result<Vec<Adapter>> {
    let output = Command::new("iw")
        .arg("dev")
        .output()
        .context("Failed to run 'iw dev'. Is iw installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "iw dev failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iw_dev(&stdout)
}

/// Parse output of `iw dev`
fn parse_iw_dev(output: &str) -> Result<Vec<Adapter>> {
    let mut adapters = Vec::new();
    let mut current_interface: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Interface ") {
            current_interface = Some(trimmed.trim_start_matches("Interface ").to_string());
        } else if trimmed.starts_with("type ") && current_interface.is_some() {
            let iface = current_interface.take().unwrap();
            let (driver, chipset) = get_adapter_info(&iface).unwrap_or_else(|_| {
                ("unknown".to_string(), "Unknown Adapter".to_string())
            });
            adapters.push(Adapter {
                interface: iface,
                driver,
                chipset,
                label: None,
            });
        }
    }

    Ok(adapters)
}

/// Get driver and chipset info for an interface
fn get_adapter_info(interface: &str) -> Result<(String, String)> {
    // Try to get info from /sys
    let uevent_path = format!("/sys/class/net/{}/device/uevent", interface);
    let driver = if let Ok(contents) = std::fs::read_to_string(&uevent_path) {
        contents
            .lines()
            .find(|l| l.starts_with("DRIVER="))
            .map(|l| l.trim_start_matches("DRIVER=").to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    // Try udevadm for more info
    let chipset = get_chipset_from_udevadm(interface).unwrap_or_else(|| {
        // Fallback to driver name as chipset
        match driver.as_str() {
            "iwlwifi" => "Intel WiFi".to_string(),
            "ath9k" | "ath10k_pci" | "ath11k" => "Atheros WiFi".to_string(),
            "rtl8xxxu" | "rtw88_pci" | "rtw89_pci" => "Realtek WiFi".to_string(),
            "brcmfmac" => "Broadcom WiFi".to_string(),
            "mt76x2u" | "mt7921e" => "MediaTek WiFi".to_string(),
            _ => format!("{} adapter", driver),
        }
    });

    Ok((driver, chipset))
}

/// Get chipset info from udevadm
fn get_chipset_from_udevadm(interface: &str) -> Option<String> {
    let output = Command::new("udevadm")
        .args(["info", &format!("/sys/class/net/{}", interface)])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for ID_MODEL_FROM_DATABASE or ID_MODEL
    for line in stdout.lines() {
        if line.contains("ID_MODEL_FROM_DATABASE=") {
            return Some(line.split('=').nth(1)?.to_string());
        }
    }
    for line in stdout.lines() {
        if line.contains("ID_MODEL=") {
            return Some(line.split('=').nth(1)?.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iw_dev() {
        let output = r#"phy#0
	Interface wlan0
		ifindex 3
		wdev 0x1
		addr aa:bb:cc:dd:ee:ff
		type managed
		channel 36 (5180 MHz), width: 80 MHz, center1: 5210 MHz
		txpower 22.00 dBm
"#;
        let adapters = parse_iw_dev(output).unwrap();
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].interface, "wlan0");
    }
}
