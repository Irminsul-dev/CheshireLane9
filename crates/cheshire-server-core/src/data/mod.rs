pub mod schemas;

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result};
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

pub fn load_all(assets_dir: impl AsRef<Path>) -> Result<()> {
    let game_dir = assets_dir.as_ref().join("game");
    let chapter = read(&game_dir.join("sharecfgdata/chapter_template.json"))?;
    let ship = read(&game_dir.join("sharecfgdata/ship_data_template.json"))?;
    let ship_statistics = read(&game_dir.join("sharecfgdata/ship_data_statistics.json"))?;
    let skin = read(&game_dir.join("ShareCfg/ship_skin_template.json"))?;

    let _ = chapter_template_data::DATA.set(serde_json::from_str(&chapter)?);
    let _ = ship_data_template_data::DATA.set(serde_json::from_str(&ship)?);
    let _ = ship_data_statistics_data::DATA.set(serde_json::from_str(&ship_statistics)?);
    let _ = ship_skin_template_data::DATA.set(serde_json::from_str(&skin)?);

    Ok(())
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read game data {}", path.display()))
}
