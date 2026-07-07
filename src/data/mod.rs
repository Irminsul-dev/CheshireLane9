pub mod schemas;

use std::fs;
use std::sync::OnceLock;

use anyhow::Result;
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

pub fn load_all() -> Result<()> {
    let chapter = fs::read_to_string("assets/game/sharecfgdata/chapter_template.json")?;
    let ship = fs::read_to_string("assets/game/sharecfgdata/ship_data_template.json")?;
    let ship_statistics = fs::read_to_string("assets/game/sharecfgdata/ship_data_statistics.json")?;
    let skin = fs::read_to_string("assets/game/ShareCfg/ship_skin_template.json")?;

    let _ = chapter_template_data::DATA.set(serde_json::from_str(&chapter)?);
    let _ = ship_data_template_data::DATA.set(serde_json::from_str(&ship)?);
    let _ = ship_data_statistics_data::DATA.set(serde_json::from_str(&ship_statistics)?);
    let _ = ship_skin_template_data::DATA.set(serde_json::from_str(&skin)?);

    Ok(())
}
