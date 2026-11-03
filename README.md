# 📦 PackMan

> A fast, unified TUI for all your package managers — built with Rust.

![Rust](https://img.shields.io/badge/built_with-Rust-orange?style=flat-square&logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey?style=flat-square)

PackMan lets you browse, search, install, remove, and upgrade packages across **pip, npm, cargo, apt, pacman, AUR, rpm, flatpak, snap, and brew** — all from one beautiful terminal interface.

---

## ✨ Features

- **Auto-detects** all of the package managers installed on your system
- **Unified package list** — switch between managers with Tab or number keys
- **Live search** — filter packages as you type
- **Install / Remove / Upgrade** packages without leaving the TUI
- **Distro detection** — shows your current Linux distro in the header
- **TokyoNight color scheme** — easy on the eyes
- Supports **10 package managers**: pip, npm, cargo, apt, pacman, AUR (yay/paru), rpm, flatpak, snap, brew

---

## 📦 Supported Package Managers

| Manager  | Platform       | Notes                        |
|----------|----------------|------------------------------|
| `pip`    | Linux / macOS  | Uses `pip3` automatically    |
| `npm`    | Linux / macOS  | Global packages only         |
| `cargo`  | Linux / macOS  | Installed crates             |
| `brew`   | macOS / Linux  | Homebrew                     |
| `apt`    | Debian/Ubuntu  | Uses `dpkg-query`            |
| `pacman` | Arch Linux     | Official repos               |
| `aur`    | Arch Linux     | AUR only via yay/paru (`-Qem`) |
| `rpm`    | Fedora/RHEL    |                              |
| `flatpak`| Linux          | Flathub apps                 |
| `snap`   | Ubuntu         |                              |

---

## 🚀 Installation

### From source

```bash
git clone https://github.com/aliabdoxd14-sudo/packman
cd packman
cargo build --release
sudo cp target/release/packman /usr/local/bin/
```

### Requirements

- Rust 1.70+
- Any of the supported package managers installed on your system

---

## ⌨️ Keyboard Shortcuts

| Key            | Action                                      |
|----------------|---------------------------------------------|
| `↑` / `k`      | Move up                                     |
| `↓` / `j`      | Move down                                   |
| `/`            | Toggle search mode                          |
| `i`            | Install package (type name in search first) |
| `u`            | Upgrade selected package                    |
| `r`            | Remove selected package                     |
| `Tab`          | Switch to next package manager              |
| `1`–`5`        | Switch to package manager by index          |
| `Ctrl+R`       | Refresh package list                        |
| `Ctrl+O`       | Toggle outdated packages only               |
| `Esc`          | Exit search mode / Quit                   |
| `q`            | Quit                                        |

---

## 🛠 Usage

```bash
# Launch PackMan
packman

# Show help
packman --help
```

**To install a package:**
1. Press `/` to enter search mode
2. Type the package name
3. Press `i` to install

---

## 🏗 Built With

- [Ratatui](https://github.com/ratatui-org/ratatui) — TUI framework
- [Crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [Tokio](https://tokio.rs) — async runtime
- [Serde JSON](https://serde.rs) — JSON parsing for npm/pip output

---

## 📄 License

MIT
