use std::path::PathBuf;
use std::env;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

pub fn auto_detect_session_path() -> Result<String, String> {
    let user_profile = env::var("USERPROFILE").map_err(|e| e.to_string())?;
    let base_path = PathBuf::from(user_profile).join("AppData\\LocalLow\\Gryphline\\Endfield");
    
    if !base_path.exists() {
        return Err("Endfield data directory not found".to_string());
    }
    
    let entries = std::fs::read_dir(base_path).map_err(|e| e.to_string())?;
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("pre_") || name.contains("_") {
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    
    Err("Could not detect session path".to_string())
}

#[cfg(windows)]
fn find_all_install_locations(display_name: &str) -> Vec<String> {
    let mut locations = Vec::new();
    let roots = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER];
    let paths = [
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ];

    for root in roots {
        let root_key = RegKey::predef(root);
        for path in paths {
            if let Ok(uninstall_key) = root_key.open_subkey(path) {
                for name in uninstall_key.enum_keys().filter_map(|x| x.ok()) {
                    if let Ok(subkey) = uninstall_key.open_subkey(&name) {
                        let d_name: String = subkey.get_value("DisplayName").unwrap_or_else(|_| "".to_string());
                        if d_name.to_uppercase().contains(&display_name.to_uppercase()) {
                            let mut location: String = subkey.get_value("InstallLocation").unwrap_or_else(|_| "".to_string());
                            location = location.trim_matches('"').to_string();
                            
                            if location.is_empty() {
                                // Try UninstallString as fallback if InstallLocation is empty
                                let uninst: String = subkey.get_value("UninstallString").unwrap_or_else(|_| "".to_string());
                                if !uninst.is_empty() {
                                    let uninst_path = PathBuf::from(uninst.trim_matches('"'));
                                    if let Some(parent) = uninst_path.parent() {
                                        location = parent.to_string_lossy().to_string();
                                    }
                                }
                            }

                            if !location.is_empty() {
                                locations.push(location);
                            }
                        }
                    }
                }
            }
        }
    }
    locations
}

pub fn auto_detect_game_path() -> Result<String, String> {
    #[cfg(windows)]
    {
        // 1. Try GRYPHLINK (Native Launcher)
        for location in find_all_install_locations("GRYPHLINK") {
            let game_path = PathBuf::from(&location).join("games\\Arknights Endfield\\Endfield.exe");
            if game_path.exists() {
                return Ok(game_path.to_string_lossy().to_string());
            }
        }

        // 2. Try Epic Games Launcher
        for location in find_all_install_locations("Epic Games Launcher") {
            let launcher_path = PathBuf::from(&location);
            let mut current = Some(launcher_path.as_path());
            let mut depth = 0;
            while let Some(path) = current {
                let game_path = path.join("ArknightsEndfieldgowoU\\games\\EndField Game\\Endfield.exe");
                if game_path.exists() {
                    return Ok(game_path.to_string_lossy().to_string());
                }
                current = path.parent();
                depth += 1;
                if depth > 3 { break; }
            }
        }
    }

    // 3. Fallback to drive scan
    let drives = ["C", "D", "E", "F", "G"];
    let relative_paths = [
        "Program Files\\GRYPHLINK\\games\\Arknights Endfield\\Endfield.exe",
        "GRYPHLINK\\games\\Arknights Endfield\\Endfield.exe",
        "Program Files\\Epic Games\\ArknightsEndfieldgowoU\\games\\EndField Game\\Endfield.exe",
        "Epic Games\\ArknightsEndfieldgowoU\\games\\EndField Game\\Endfield.exe",
    ];
    
    for drive in drives {
        for rel in &relative_paths {
            let path = PathBuf::from(format!("{}:\\{}", drive, rel));
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }
    
    Err("Endfield.exe not found automatically".to_string())
}
