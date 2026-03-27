use configparser::ini::Ini;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppSettings {
    pub session_path: String,
    pub game_path: String,
    pub language: String,
}

pub fn get_app_data_dir() -> PathBuf {
    let mut path = dirs::document_dir().expect("Failed to get Documents directory");
    path.push("TA-TA");
    path.push("switch");
    
    if !path.exists() {
        std::fs::create_dir_all(&path).expect("Failed to create app data directory");
    }
    path
}

pub fn get_ini_path() -> PathBuf {
    get_app_data_dir().join("settings.ini")
}

pub fn load_settings() -> AppSettings {
    let mut config = Ini::new();
    let ini_path = get_ini_path();
    
    if let Ok(_map) = config.load(ini_path) {
        AppSettings {
            session_path: config.get("paths", "session_path").unwrap_or_default(),
            game_path: config.get("paths", "game_path").unwrap_or_default(),
            language: config.get("paths", "language").unwrap_or_else(|| "en".to_string()),
        }
    } else {
        AppSettings {
            language: "en".to_string(),
            ..Default::default()
        }
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let mut config = Ini::new();
    let ini_path = get_ini_path();
    
    config.set("paths", "session_path", Some(settings.session_path.clone()));
    config.set("paths", "game_path", Some(settings.game_path.clone()));
    config.set("paths", "language", Some(settings.language.clone()));
    
    config.write(ini_path).map_err(|e| e.to_string())?;
    Ok(())
}
