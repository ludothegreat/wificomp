# wificomp - WiFi Adapter Comparison Tool

A Rust TUI application for testing and comparing WiFi adapter signal strength. Test one adapter at a time, save results, then compare multiple sessions side-by-side to find which adapter performs best in your environment.

## Screenshots

### Live Scan Screen

```
┌─ wificomp ──────────────────────────── [1]Live │ [2]Hist │ [3]Cmp ─┐
│ Intel AX200 "USB Dongle"                                  [r]ename │
│ Timer: 02:34/05:00  Auto: ON 5s  APs: 8                   [t] [a]  │
├────────────────────────────────────────────────────────────────────┤
│ SSID            Signal                                  CH   Band  │
├────────────────────────────────────────────────────────────────────┤
│ MyNetwork       -42 ████████████████████████████████████  36   5G  │
│ Neighbor_5G     -51 █████████████████████████████         40   5G  │
│ CoffeeShop      -58 ████████████████████████              6    2G  │
│ HomeBase        -63 █████████████████████                 11   2G  │
│ Guest_Network   -71 ███████████████                       1    2G  │
│ IoT_Devices     -76 ████████████                          6    2G  │
│ <hidden>        -82 ████████                              11   2G  │
│ FreeWiFi        -89 ████                                  1    2G  │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ [spc]scan [c]h [b]and [f]req [s]ort:Sig [x]clude [e]xp [q]uit      │
└────────────────────────────────────────────────────────────────────┘
```

### History Screen

```
┌─ History ───────────────────────────── [1]Live │ [2]Hist │ [3]Cmp ─┐
│ Intel AX200 "USB Dongle" | 5:00 | 60 scans                 [l]oad  │
├────────────────────────────────────────────────────────────────────┤
│ AP: MyNetwork (AA:BB:CC:DD:EE:FF)                          [↑][↓]  │
│ Time: [5m] 10m 30m All   Data: [Raw] Avg                           │
├────────────────────────────────────────────────────────────────────┤
│ -40│                    ██                                         │
│    │         ██ ██ ██ ████ ██                                      │
│ -50│██ ██ ████████████████████████ ██                              │
│    │                              ████████                         │
│ -60│                                      ████ ██                  │
│    │                                          ████                 │
│ -70│                                                               │
│    └─────────────────────────────────────────────────────          │
│     14:30       14:32       14:34       14:36       14:38          │
├────────────────────────────────────────────────────────────────────┤
│ Avg: -48 dBm   Min: -61 dBm   Max: -42 dBm   Samples: 60           │
├────────────────────────────────────────────────────────────────────┤
│ [w]indow [d]ata [e]xport [q]uit                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Compare Screen

```
┌─ Compare ───────────────────────────── [1]Live │ [2]Hist │ [3]Cmp ─┐
│ Sessions: 3 loaded                                  [+]add [x]del  │
├────────────────────────────────────────────────────────────────────┤
│ 1. USB Dongle      Intel AX200    01-31 14:30   12 scans           │
│ 2. PCIe Card       Intel AX210    01-31 14:45   15 scans           │
│ 3. Cheap Stick     RTL8812AU      01-31 15:00   10 scans           │
├────────────────────────────────────────────────────────────────────┤
│ AP: MyNetwork                                                [↑↓]  │
│ Match: [BSSID] SSID Both   Metric: [Avg] Min Max                   │
├────────────────────────────────────────────────────────────────────┤
│ USB Dongle    -45 ████████████████████████████████████████████  ★  │
│ PCIe Card     -52 ████████████████████████████████████             │
│ Cheap Stick   -68 ████████████████████████                         │
├────────────────────────────────────────────────────────────────────┤
│ Best: USB Dongle (7/8 APs strongest)                               │
├────────────────────────────────────────────────────────────────────┤
│ [+]load [m]atch [M]etric [e]xport [q]uit                           │
└────────────────────────────────────────────────────────────────────┘
```

## Features

- **Live Scanning**: Real-time WiFi scanning with configurable auto-scan interval
- **Session Recording**: Automatically logs all scan data for later analysis
- **History View**: Time-series graphs showing signal strength over time
- **Comparison Mode**: Compare multiple adapter sessions side-by-side
- **AP Exclusion**: Hide APs you don't care about (per-session or permanently)
- **Export**: Save sessions as JSON or CSV

## About

This tool was originally built for the [Hackberry Pi CM5](https://github.com/ZitaoTech/Hackberry-Pi_Zero) - a handheld Linux device with an 80x24 character terminal display. This explains several design decisions:

- **Compact UI**: All screens fit within 80x24 characters
- **Abbreviated labels**: Short key hints like `[spc]`, `[c]h`, `[f]req` to save space
- **Single-adapter workflow**: Test one adapter at a time, then compare sessions (the Hackberry has limited USB ports)
- **Keyboard-only navigation**: No mouse support - optimized for thumb keyboards
- **Low refresh rate**: 250ms tick rate to conserve battery and reduce CPU usage

The tool works great on any Linux terminal, but if you're wondering why everything is so compact - that's why!

## Requirements

- Linux with `iw` command available
- WiFi adapter
- Terminal with at least 60x15 size (80x24 recommended for best experience)

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

## Signal Strength Guide

| dBm Range | Quality | Bar Fill |
|-----------|---------|----------|
| -30 to -50 | Excellent | ████████████████ |
| -50 to -60 | Good | ████████████ |
| -60 to -70 | Fair | ████████ |
| -70 to -80 | Weak | ████ |
| -80 to -90 | Poor | ██ |

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

Sessions are stored as JSON in `~/.local/share/wificomp/sessions/`. Each file captures the complete scan history for one adapter testing session.

### Example Session File

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
        },
        {
          "bssid": "11:22:33:44:55:66",
          "ssid": "Neighbor_5G",
          "signal_dbm": -52,
          "channel": 40,
          "frequency_mhz": 5200
        },
        {
          "bssid": "AA:11:BB:22:CC:33",
          "ssid": "CoffeeShop",
          "signal_dbm": -67,
          "channel": 6,
          "frequency_mhz": 2437
        }
      ]
    },
    {
      "timestamp": "2026-01-31T14:30:10Z",
      "access_points": [
        {
          "bssid": "AA:BB:CC:DD:EE:FF",
          "ssid": "MyNetwork",
          "signal_dbm": -43,
          "channel": 36,
          "frequency_mhz": 5180
        },
        {
          "bssid": "11:22:33:44:55:66",
          "ssid": "Neighbor_5G",
          "signal_dbm": -54,
          "channel": 40,
          "frequency_mhz": 5200
        },
        {
          "bssid": "AA:11:BB:22:CC:33",
          "ssid": "CoffeeShop",
          "signal_dbm": -65,
          "channel": 6,
          "frequency_mhz": 2437
        }
      ]
    }
  ]
}
```

### Field Descriptions

| Field | Description |
|-------|-------------|
| `version` | File format version for compatibility |
| `adapter.interface` | Linux network interface (e.g., wlan0, wlan1) |
| `adapter.driver` | Kernel driver name |
| `adapter.chipset` | Hardware chipset identifier |
| `adapter.label` | User-defined friendly name |
| `started_at` | Session start time (UTC ISO 8601) |
| `duration_target_secs` | Timer setting in seconds (null if disabled) |
| `scans[].timestamp` | When this scan was taken |
| `scans[].access_points[]` | All APs detected in this scan |
| `signal_dbm` | Signal strength in dBm (higher = better, typical range -30 to -90) |
| `channel` | WiFi channel number |
| `frequency_mhz` | Frequency in MHz (2400s = 2.4GHz, 5000s = 5GHz, 6000s = 6GHz) |

## Tips

- **Consistent testing**: Test adapters in the same location and time period for fair comparison
- **Use labels**: Press `r` to label adapters - makes comparison much easier
- **Exclude noise**: Use `x` to hide APs you don't want scanned
- **Match by SSID**: If BSSIDs differ between scans (AP roaming), use SSID matching in Compare
- **Check stability**: The History graph shows signal stability, not just strength
- **5GHz vs 2.4GHz**: Use frequency filter (`f`) to compare performance on specific bands

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

MIT License - see [LICENSE](LICENSE) file for details.
