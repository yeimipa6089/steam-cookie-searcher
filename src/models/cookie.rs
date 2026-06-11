use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamCookie {
    pub domain: String,
    pub secure: bool,
    pub name: String,
    pub value: String,
}

pub type BulkResult = Vec<(String, Vec<SteamCookie>)>;
