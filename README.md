# TA-TA Switch

**English** | [한국어](./docs/README.ko.md)

![Preview](./docs/preview.png)

[Download Latest Release](https://github.com/kimcatchy/TA-TA-Switch/releases/download/v1.0.0/TA-TA-Switch.exe)

A lightweight, high-performance account manager for "Arknights: Endfield (Global)". This tool is designed to switch between multiple accounts with a single click by swapping locally stored session cache files, with a minimal memory footprint (< 10MB) and zero background CPU usage.

⚠️ **Note: This tool is compatible with "Arknights: Endfield" installed via the [Native Launcher](https://endfield.gryphline.com/) and the Epic Games Store. Installations via Google Play Games are currently not supported.**

## ✨ Features

- **Profile Swapping**: Instantly switch between multiple accounts by replacing session files.
- **Memory Efficient & Native UI**: Uses native Windows Win32 API for menus and dialogs, optimized to stay under 10MB of RAM.
- **Auto Detection**: Automatically finds your game installation and session folders. (Native Launcher prioritized)
- **Secure Backups**: Keeps account session data safe in independent storage.
- **Admin Elevation**: Automatically requests administrator privileges to modify session files.
- **Localized**: Full support for English and Korean.

## 📂 Data Storage & Backups
Settings and backup data are stored in the user's **Documents** folder instead of the executable's directory. This ensures your data remains safe even if you move or delete the executable.

- **Path**: `%USERPROFILE%\Documents\TA-TA\switch`
- **Contents**:
  - `settings.ini`: App configuration and language preferences.
  - `backups/`: Folder containing independent session data for each account.

## 🚀 How to Use

1. **Launch**: Run `TA-TA-Switch.exe`.
2. **Auto Detect**: Click the tray icon and select **Auto Detect Paths**. Prioritizes installations via the native launcher; if not found, checks for the Epic Games Store path.
3. **Save Session**: Log in to an account in-game, then select **Save Current Session** from the tray to name and save that profile.
4. **Switch**: Close the game and select a profile from the **Select Profile** menu.
5. **Launch Game**: Click **Launch Endfield** to start the game with the selected profile.

## 🛠️ Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (Stable channel)

### Steps
1. Clone the repository:
   ```bash
   git clone https://github.com/kimcatchy/TA-TA-Switch.git
   cd TA-TA-Switch
   ```
2. Build in release mode:
   ```bash
   cargo build --release
   ```
3. The executable will be in `target/release/TA-TA-Switch.exe`.

## 🛠️ Technical Details

- **Language**: Rust 2021
- **UI Framework**: [Native Windows GUI (NWG)](https://github.com/gabdube/native-windows-gui)
- **Tray Implementation**: [tray-icon](https://github.com/tauri-apps/tray-icon)
- **Resource Management**: Icons and Locales are embedded directly into the binary at compile time.

## ⚠️ Disclaimer
- This tool does not modify the game client; it only manages local cache files. Usage is at your own risk.
- This tool is not affiliated with, endorsed by, or officially supported by Hypergryph, Arknights: Endfield, or the Epic Games Store.
