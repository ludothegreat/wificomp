# wificomp - WiFi Adapter Comparison Tool

A Rust TUI application for testing and comparing WiFi adapter signal strength. Test one adapter at a time, save results, then compare multiple sessions side-by-side to find which adapter performs best in your environment.

## Features

- **Live Scanning**: Real-time WiFi scanning with configurable auto-scan interval
- **Session Recording**: Automatically logs all scan data for later analysis
- **History View**: Time-series graphs showing signal strength over time
- **Comparison Mode**: Compare multiple adapter sessions side-by-side
- **AP Exclusion**: Hide APs you don't care about (per-session or permanently)
- **Export**: Save sessions as JSON or CSV

## Requirements

- Linux with `iw` command available
- WiFi adapter
- Terminal with at least 60x15 size (80x24 recommended)

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/wificomp`.

### Permissions

WiFi scanning requires elevated permissions. Either run with sudo:

```bash
sudo ./target/release/wificomp
```

Or grant the binary `CAP_NET_ADMIN` capability:

```bash
sudo setcap cap_net_admin+ep ./target/release/wificomp
```

## Usage

```bash
# Auto-detect adapter and start scanning
sudo wificomp

# Specify interface
sudo wificomp --interface wlan0

# Disable auto-scan
sudo wificomp --no-auto-scan
```

## Workflow

### Testing a Single Adapter

1. Connect your WiFi adapter
2. Run `sudo wificomp`
3. The app auto-detects your adapter and starts scanning
4. Press `r` to give the adapter a memorable label (e.g., "USB Dongle")
5. Optionally set a timer with `t` (e.g., 5 minutes)
6. Let it run, collecting signal data
7. Press `q` to quit - session is saved automatically

### Comparing Multiple Adapters

1. Test each adapter one at a time (as above)
2. Run wificomp and switch to Compare tab (`3`)
3. Press `+` to load saved sessions
4. Navigate with `↑/↓` to select an AP of interest
5. View the comparison bars showing signal strength per adapter
6. The "Best" summary shows which adapter won the most APs

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `1` | Live scan screen |
| `2` | History screen |
| `3` | Compare screen |
| `q` | Quit (prompts if unsaved data) |
| `Ctrl+C` | Quit |

### Live Scan Screen

| Key | Action |
|-----|--------|
| `Space` | Manual scan |
| `a` | Toggle auto-scan |
| `t` | Set session timer |
| `r` | Rename adapter |
| `c` | Toggle channel column |
| `b` | Toggle band column (2G/5G/6G) |
| `f` | Cycle frequency filter (All/2.4G/5G/6G) |
| `s` | Cycle sort mode (Signal/SSID/Channel) |
| `h` | Toggle highlight best signal |
| `x` | Exclude selected AP |
| `e` | Export session |
| `↑/↓` | Navigate AP list |

### History Screen

| Key | Action |
|-----|--------|
| `l` / `+` | Load session file |
| `w` | Cycle time window (5m/10m/30m/All) |
| `d` | Toggle raw/average data |
| `e` | Export session |
| `↑/↓` | Select AP |

### Compare Screen

| Key | Action |
|-----|--------|
| `+` | Load session to compare |
| `x` | Remove selected session |
| `m` | Cycle AP match mode (BSSID/SSID/Both) |
| `M` | Cycle metric (Avg/Min/Max) |
| `e` | Export comparison |
| `↑/↓` | Select AP |
| `←/→` | Select session |

## File Locations

| Type | Location |
|------|----------|
| Sessions | `~/.local/share/wificomp/sessions/` |
| Config | `~/.config/wificomp/config.json` |

## Configuration

Settings are automatically saved between sessions:

- Auto-scan interval
- Default timer duration
- Column visibility (channel, band)
- Sort and filter preferences
- History time window
- Compare match/metric modes
- Permanently excluded APs

## Session File Format

Sessions are stored as JSON:

```json
{
  "version": "1.0",
  "adapter": {
    "interface": "wlan0",
    "driver": "iwlwifi",
    "chipset": "Intel AX200",
    "label": "USB Dongle"
  },
  "started_at": "2026-01-31T14:30:00Z",
  "duration_target_secs": 300,
  "scans": [
    {
      "timestamp": "2026-01-31T14:30:05Z",
      "access_points": [
        {
          "bssid": "AA:BB:CC:DD:EE:FF",
          "ssid": "MyNetwork",
          "signal_dbm": -45,
          "channel": 36,
          "frequency_mhz": 5180
        }
      ]
    }
  ]
}
```

## Tips

- **Consistent testing**: Test adapters in the same location and time period for fair comparison
- **Use labels**: Press `r` to label adapters - makes comparison much easier
- **Exclude noise**: Use `x` to hide APs you don't care about
- **Match by SSID**: If BSSIDs differ between scans (AP roaming), use SSID matching in Compare
- **Check stability**: The History graph shows signal stability, not just strength

## Troubleshooting

**"Failed to detect adapters"**
- Ensure a WiFi adapter is connected
- Check that `iw dev` shows your interface

**"Scan failed" or permission errors**
- Run with `sudo` or set capabilities (see Installation)

**No APs showing**
- Adapter might need a moment to initialize
- Try manual scan with `Space`
- Check if adapter supports monitor mode

**Terminal too small**
- Resize to at least 60x15 characters

**Sessions not loading in Compare**
- Session may have been interrupted before any scans completed
- Check the session file for scan data

## License

MIT
