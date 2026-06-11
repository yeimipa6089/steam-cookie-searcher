#[derive(Debug, Clone)]
pub struct NetworkRequest {
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub req_headers: Vec<(String, String)>,
    pub res_headers: Vec<(String, String)>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Log(String),
    Network(NetworkRequest),
}
