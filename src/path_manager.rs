use std::path::PathBuf;
use std::env;

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

pub fn auto_detect_game_path() -> Result<String, String> {
    let drives = ["C", "D", "E", "F", "G"];
    let relative_path = "Program Files\\Epic Games\\ArknightsEndfieldgowoU\\games\\EndField Game\\Endfield.exe";
    let alternate_relative_path = "Epic Games\\ArknightsEndfieldgowoU\\games\\EndField Game\\Endfield.exe";
    
    for drive in drives {
        let path1 = PathBuf::from(format!("{}:\\{}", drive, relative_path));
        if path1.exists() {
            return Ok(path1.to_string_lossy().to_string());
        }
        let path2 = PathBuf::from(format!("{}:\\{}", drive, alternate_relative_path));
        if path2.exists() {
            return Ok(path2.to_string_lossy().to_string());
        }
    }
    
    Err("Endfield.exe not found automatically".to_string())
}
