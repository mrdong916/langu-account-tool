use crate::{domain::workspace::Settings, error::AppResult};
use tauri::{AppHandle, Manager};

pub fn load(app: &AppHandle) -> AppResult<Settings> {
    let path = app.path().app_config_dir()?.join("settings.json");
    if !path.exists() {
        return Ok(Settings::default());
    }
    let mut settings: Settings = serde_json::from_slice(&std::fs::read(path)?)?;
    settings.validate()?;
    Ok(settings)
}

pub fn save(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    let directory = app.path().app_config_dir()?;
    std::fs::create_dir_all(&directory)?;
    let saved = if settings.remember {
        settings.clone()
    } else {
        Settings {
            remember: false,
            ..Settings::default()
        }
    };
    let temporary = directory.join("settings.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&saved)?)?;
    std::fs::rename(temporary, directory.join("settings.json"))?;
    Ok(())
}
