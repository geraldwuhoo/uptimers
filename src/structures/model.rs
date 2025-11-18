use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
#[serde(deny_unknown_fields)]
pub struct SiteModel {
    pub site: String,
    pub name: String,
    pub success_codes: Option<Vec<u16>>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case, dead_code)]
pub struct SiteFactModel {
    pub site: String,
    pub tstamp: time::OffsetDateTime,
    pub success: bool,
    pub status_code: i16,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case)]
pub struct SiteFullModel {
    pub site: String,
    pub name: String,
    pub tstamp: time::OffsetDateTime,
    pub success: bool,
    pub status_code: i16,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(non_snake_case, dead_code)]
pub struct SiteStatModel {
    pub site: String,
    pub name: String,
    pub tstamp: time::OffsetDateTime,
    pub success: bool,
    pub status_code: i16,
    pub avg: Option<f64>,
}
