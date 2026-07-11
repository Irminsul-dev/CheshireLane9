pub mod schemas;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use schemas::chapter_template::ChapterTemplate;
use schemas::ship_data_statistics::ShipDataStatistics;
use schemas::ship_data_template::ShipDataTemplate;
use schemas::ship_skin_template::ShipSkinTemplate;

pub mod chapter_template_data {
    use super::*;
    pub static DATA: OnceLock<ChapterTemplate> = OnceLock::new();
}

pub mod ship_data_template_data {
    use super::*;
    pub static DATA: OnceLock<ShipDataTemplate> = OnceLock::new();
}

pub mod ship_data_statistics_data {
    use super::*;
    pub static DATA: OnceLock<ShipDataStatistics> = OnceLock::new();
}

pub mod ship_skin_template_data {
    use super::*;
    pub static DATA: OnceLock<ShipSkinTemplate> = OnceLock::new();
}

static LOAD_LOCK: Mutex<()> = Mutex::new(());
static LOADED_ASSETS_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn load_all(assets_dir: impl AsRef<Path>) -> Result<()> {
    let assets_dir = fs::canonicalize(assets_dir.as_ref()).with_context(|| {
        format!(
            "resolve game assets directory {}",
            assets_dir.as_ref().display()
        )
    })?;
    let _load_guard = LOAD_LOCK
        .lock()
        .map_err(|_| anyhow!("game assets loader lock is poisoned"))?;

    if let Some(loaded) = LOADED_ASSETS_DIR.get() {
        if loaded == &assets_dir {
            return Ok(());
        }
        bail!(
            "game assets are already loaded from {}; restart the application to use {}",
            loaded.display(),
            assets_dir.display()
        );
    }

    let game_dir = assets_dir.join("game");
    let chapter = read(&game_dir.join("sharecfgdata/chapter_template.json"))?;
    let ship = read(&game_dir.join("sharecfgdata/ship_data_template.json"))?;
    let ship_statistics = read(&game_dir.join("sharecfgdata/ship_data_statistics.json"))?;
    let skin = read(&game_dir.join("ShareCfg/ship_skin_template.json"))?;

    let chapter = serde_json::from_str(&chapter)?;
    let ship = serde_json::from_str(&ship)?;
    let ship_statistics = serde_json::from_str(&ship_statistics)?;
    let skin = serde_json::from_str(&skin)?;

    chapter_template_data::DATA
        .set(chapter)
        .map_err(|_| anyhow!("chapter data was initialized outside the game assets loader"))?;
    ship_data_template_data::DATA
        .set(ship)
        .map_err(|_| anyhow!("ship data was initialized outside the game assets loader"))?;
    ship_data_statistics_data::DATA
        .set(ship_statistics)
        .map_err(|_| anyhow!("ship statistics were initialized outside the game assets loader"))?;
    ship_skin_template_data::DATA
        .set(skin)
        .map_err(|_| anyhow!("ship skin data was initialized outside the game assets loader"))?;
    LOADED_ASSETS_DIR
        .set(assets_dir)
        .map_err(|_| anyhow!("game assets directory was initialized concurrently"))?;

    Ok(())
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read game data {}", path.display()))
}
