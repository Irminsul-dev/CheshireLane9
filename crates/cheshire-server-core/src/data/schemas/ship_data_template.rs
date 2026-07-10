use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ShipDataTemplate(pub HashMap<String, ShipDataTemplateEntity>);

#[derive(Debug, Deserialize)]
pub struct ShipDataTemplateEntity {
    pub buff_list: Vec<u32>,
    pub energy: u32,
    pub equip_id_1: u32,
    pub equip_id_2: u32,
    pub equip_id_3: u32,
    pub group_type: u32,
    pub id: u32,
    pub star: u32,
    pub star_max: u32,
}
