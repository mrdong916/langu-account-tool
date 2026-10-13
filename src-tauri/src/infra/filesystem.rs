use std::path::PathBuf;

pub fn workspace_dir() -> Option<PathBuf> {
    std::env::current_dir().ok()
}
