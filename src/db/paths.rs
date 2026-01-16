use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

pub fn default_db_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("", "", "sw_galaxy_map")
        .context("Unable to determine OS app data directory")?;

    let dir = proj.data_local_dir();
    std::fs::create_dir_all(dir).context("Unable to create app data directory")?;

    Ok(dir.join("sw_planets.sqlite"))
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create directory: {}", parent.display()))?;
    }
    Ok(())
}
