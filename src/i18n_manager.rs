use crate::settings_manager;
use serde_json::Value;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Global cache for loaded locale data to avoid repeated disk I/O and parsing.
static LOCALES: Lazy<HashMap<String, Value>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("en".to_string(), serde_json::from_str(include_str!("../locales/en.json")).unwrap_or_default());
    m.insert("ko".to_string(), serde_json::from_str(include_str!("../locales/ko.json")).unwrap_or_default());
    m
});

/// Retrieves a localized string based on the current language setting.
pub fn get_message(section: &str, key: &str) -> String {
    let settings = settings_manager::load_settings();
    let lang = settings.language.as_str();

    let v = LOCALES.get(lang).or_else(|| LOCALES.get("en")).unwrap();
    
    v[section][key].as_str().unwrap_or(key).to_string()
}
