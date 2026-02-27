# an8nymous Stats

Lightweight always-on-top system stats overlay for Windows, built with Rust and Tauri v2.

[![Release](https://img.shields.io/github/v/release/Lucagdev/rust-stats-overlay?style=flat&color=c8c1b8&labelColor=1c1b19)](https://github.com/Lucagdev/rust-stats-overlay/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Lucagdev/rust-stats-overlay/total?style=flat&color=c8c1b8&labelColor=1c1b19)](https://github.com/Lucagdev/rust-stats-overlay/releases)
[![License](https://img.shields.io/github/license/Lucagdev/rust-stats-overlay?style=flat&color=c8c1b8&labelColor=1c1b19)](LICENSE)

---

```
CPU 14%  2.40GHz  GPU 68%  1800MHz  RAM 58%  11.2/32.0GB  Disk 0MB/s  ↓0.1↑0.0MB/s
```

> A transparent bar that sits on top of any window — including games running in Borderless Windowed mode.

---

## Features

| Metric | Source |
|--------|--------|
| CPU usage & frequency (GHz) | sysinfo |
| RAM usage (% and GB) | sysinfo |
| GPU usage, temperature, clock, power draw | NVML (NVIDIA only) |
| VRAM usage | NVML (NVIDIA only) |
| Disk I/O read/write (MB/s) | Windows PDH |
| Network download/upload (MB/s) | sysinfo |

- Transparent, borderless, always-on-top window
- Click-through — the overlay never blocks mouse input
- Re-asserts `HWND_TOPMOST` every 500ms to stay above game windows
- Configurable: metrics selection, order, text color, font, size and position
- System tray with show/hide toggle
- Start with Windows option

## Download

**[→ Download latest release](https://github.com/Lucagdev/rust-stats-overlay/releases/latest/download/an8nymous-stats.exe)**

No installer — just run the `.exe`. See the **[download page](https://lucagdev.github.io/rust-stats-overlay/)** for full instructions.

**Requirements:**
- Windows 10 or 11 (64-bit)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) — pre-installed on Windows 11, auto-installs on Windows 10
- NVIDIA GPU for GPU metrics (optional — app works fine without one)

## Usage

1. Run `an8nymous-stats.exe` — the overlay appears in the top-right corner
2. Right-click the **tray icon** to open Settings or toggle visibility
3. In Settings, pick which metrics to show, reorder them, and customize appearance
4. For use with games, set the game to **Borderless Windowed** mode

## Build from source

```bash
# Prerequisites: Rust stable, WebView2 Runtime

git clone https://github.com/Lucagdev/rust-stats-overlay
cd rust-stats-overlay/src-tauri
cargo build --release

# Binary output: src-tauri/target/release/an8nymous-stats.exe
```

## Project structure

```
src/
  overlay.html      # Overlay window (700×30, transparent, always-on-top)
  settings.html     # Settings window (620×720)
src-tauri/
  src/
    lib.rs          # Tauri setup: tray, overlay position, click-through
    commands.rs     # Tauri commands exposed to the frontend
    config.rs       # Config structs, load/save JSON
    stats.rs        # CPU, RAM, Disk I/O, Network collection
    gpu.rs          # NVIDIA GPU stats via NVML
  tauri.conf.json
  Cargo.toml
```

## License

MIT
