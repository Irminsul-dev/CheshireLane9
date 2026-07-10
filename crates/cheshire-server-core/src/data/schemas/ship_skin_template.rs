use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ShipSkinTemplate(pub HashMap<String, ShipSkinTemplateEntity>);

#[derive(Debug, Deserialize)]
pub struct ShipSkinTemplateEntity {
    pub id: u32,
    pub ship_group: u32,
}
