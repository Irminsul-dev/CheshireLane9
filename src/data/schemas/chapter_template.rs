use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ChapterTemplate(pub HashMap<String, Value>);
