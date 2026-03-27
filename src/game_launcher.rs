use std::process::Command;

pub fn launch_endfield(game_path: &str) -> Result<(), String> {
    Command::new(game_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
