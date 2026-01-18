use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

const APP_DIRECTORY: &str = "vda5050-analysis-tool";
const RECENT_FILES_NAME: &str = "recent-files.json";
const MAX_RECENT_FILES: usize = 10;

/// Load the files that were successfully opened in previous sessions.
pub(crate) fn load() -> Vec<String> {
    let Some(storage_path) = storage_path() else {
        return Vec::new();
    };

    let Ok(contents) = fs::read_to_string(&storage_path) else {
        return Vec::new();
    };

    let Ok(stored_paths) = serde_json::from_str::<Vec<String>>(&contents) else {
        return Vec::new();
    };

    let mut paths = stored_paths;
    let original_paths = paths.clone();
    let mut seen = HashSet::new();
    paths.retain(|path| Path::new(path).is_file() && seen.insert(path.clone()));
    paths.truncate(MAX_RECENT_FILES);

    // Clean up deleted files and old entries the next time the app starts.
    if paths != original_paths {
        save(&paths);
    }

    paths
}

/// Promote a successfully opened file to the front of the recent-file list.
pub(crate) fn remember(paths: &mut Vec<String>, file_path: &Path) {
    let normalized_path = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.to_path_buf())
        .to_string_lossy()
        .into_owned();

    paths.retain(|path| path != &normalized_path);
    paths.insert(0, normalized_path);
    paths.truncate(MAX_RECENT_FILES);
    save(paths);
}

fn storage_path() -> Option<PathBuf> {
    dirs::config_dir().map(|config_dir| config_dir.join(APP_DIRECTORY).join(RECENT_FILES_NAME))
}

fn save(paths: &[String]) {
    let Some(storage_path) = storage_path() else {
        return;
    };

    let Some(parent) = storage_path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let Ok(contents) = serde_json::to_string_pretty(paths) else {
        return;
    };

    let _ = fs::write(storage_path, contents);
}
