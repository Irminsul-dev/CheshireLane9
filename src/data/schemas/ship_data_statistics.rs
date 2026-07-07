use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ShipDataStatistics(pub HashMap<String, ShipDataStatisticsEntity>);

#[derive(Debug, Deserialize)]
pub struct ShipDataStatisticsEntity {
    pub skin_id: u32,
}
