use super::village::VillageRow;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct UpdatePayload {
    pub villages: Vec<VillageRow>,
}

#[derive(Deserialize)]
pub struct MetaUpdatePayload {
    pub meta: BTreeMap<String, String>,
}
