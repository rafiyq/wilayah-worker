use super::village::VillageRow;
use serde::Serialize;
use wilayah::{Location, Village};

#[derive(Serialize)]
pub struct IndexResponse {
    pub name: String,
    pub version: String,
    pub village_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearest: Option<Vec<Village>>,
}

#[derive(Serialize)]
pub struct NearestResponse {
    pub results: Vec<Village>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<VillageRow>,
}

#[derive(Serialize)]
pub struct CodeResponse {
    pub result: Option<VillageRow>,
}

#[derive(Serialize)]
pub struct CodePrefixResponse {
    pub results: Vec<VillageRow>,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct LocateResponse {
    pub result: Option<Location>,
}

#[derive(Serialize)]
pub struct UpdateResponse {
    pub upserted: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
