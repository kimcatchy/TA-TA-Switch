use std::fs;
use std::path::{Path, PathBuf};
use crate::settings_manager::get_app_data_dir;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use sysinfo::{System, ProcessRefreshKind, RefreshKind, ProcessesToUpdate};

pub static ACTIVE_ACCOUNT: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn is_game_running() -> bool {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything())
    );
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());
    
    for process in sys.processes().values() {
        if process.name().to_string_lossy().to_lowercase().contains("endfield.exe") {
            return true;
        }
    }
    false
}

fn get_backups_dir() -> PathBuf {
    get_app_data_dir().join("backups")
}

pub fn get_active_account() -> Option<String> {
    ACTIVE_ACCOUNT.lock().unwrap().clone()
}

pub fn set_active_account(name: Option<String>) {
    let mut active = ACTIVE_ACCOUNT.lock().unwrap();
    *active = name;
}

pub fn save_account_session(session_path: &str, account_name: &str) -> Result<(), String> {
    if is_game_running() {
        return Err("Cannot save session while game is running. Please close Endfield first.".to_string());
    }
    let backups_dir = get_backups_dir();
    let account_dir = backups_dir.join(account_name);
    
    fs::create_dir_all(&account_dir).map_err(|e| e.to_string())?;
    
    let source_base = Path::new(session_path);
    let files_to_copy = ["gf_login_cache", "gf_login_cache.crc"];
    
    for file in files_to_copy {
        let src = source_base.join(file);
        let dst = account_dir.join(file);
        if src.exists() {
            fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        }
    }
    
    Ok(())
}

pub fn load_account_session(session_path: &str, account_name: &str) -> Result<(), String> {
    if is_game_running() {
        return Err("Cannot switch profiles while game is running. Please close Endfield first.".to_string());
    }
    let backups_dir = get_backups_dir();
    let account_dir = backups_dir.join(account_name);
    
    if !account_dir.exists() {
        return Err("Account backup not found".to_string());
    }
    
    let target_base = Path::new(session_path);
    let files_to_copy = ["gf_login_cache", "gf_login_cache.crc"];
    
    for file in files_to_copy {
        let src = account_dir.join(file);
        let dst = target_base.join(file);
        if src.exists() {
            fs::copy(&src, &dst).map_err(|e| e.to_string())?;
        }
    }
    
    // Set active account on success
    set_active_account(Some(account_name.to_string()));
    
    Ok(())
}

pub fn get_saved_accounts() -> Result<Vec<String>, String> {
    let backups_dir = get_backups_dir();
    
    if !backups_dir.exists() {
        return Ok(vec![]);
    }
    
    let mut accounts = vec![];
    let entries = fs::read_dir(&backups_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        if let Ok(entry) = entry {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    accounts.push(name.to_string());
                }
            }
        }
    }
    
    Ok(accounts)
}

pub fn delete_account(account_name: &str) -> Result<(), String> {
    let backups_dir = get_backups_dir();
    let account_dir = backups_dir.join(account_name);
    
    if account_dir.exists() {
        fs::remove_dir_all(account_dir).map_err(|e| e.to_string())?;
    }
    
    // If deleted the active account, clear it
    let mut active = ACTIVE_ACCOUNT.lock().unwrap();
    if active.as_ref() == Some(&account_name.to_string()) {
        *active = None;
    }
    
    Ok(())
}
