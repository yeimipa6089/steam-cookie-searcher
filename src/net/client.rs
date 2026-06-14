use crate::models::cookie::SteamCookie;
use crate::models::network::NetworkRequest;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc::UnboundedSender;

static LOG_TX: OnceLock<Mutex<UnboundedSender<String>>> = OnceLock::new();
static NETWORK_TX: OnceLock<Mutex<UnboundedSender<NetworkRequest>>> = OnceLock::new();

pub fn set_global_log_tx(tx: UnboundedSender<String>) {
    let _ = LOG_TX.set(Mutex::new(tx));
}

pub fn set_global_network_tx(tx: UnboundedSender<NetworkRequest>) {
    let _ = NETWORK_TX.set(Mutex::new(tx));
}

pub fn emit_log(msg: String) {
    if let Some(lock) = LOG_TX.get()
        && let Ok(tx) = lock.lock()
    {
        let _ = tx.send(msg);
    }
}

pub fn emit_network(req: NetworkRequest) {
    if let Some(lock) = NETWORK_TX.get()
        && let Ok(tx) = lock.lock()
    {
        let _ = tx.send(req);
    }
}

pub fn build_http_client(
    cookies: &[SteamCookie],
    proxy_url: Option<&str>,
) -> (reqwest::Client, Arc<Jar>) {
    let jar = Arc::new(Jar::default());

    let domain_urls: &[(&str, &str)] = &[
        ("steamcommunity.com", "https://steamcommunity.com"),
        ("store.steampowered.com", "https://store.steampowered.com"),
        ("login.steampowered.com", "https://login.steampowered.com"),
        (
            "checkout.steampowered.com",
            "https://checkout.steampowered.com",
        ),
        ("help.steampowered.com", "https://help.steampowered.com"),
    ];

    for c in cookies {
        for (domain_match, url_str) in domain_urls {
            if c.domain.contains(domain_match) {
                if let Ok(url) = url_str.parse::<reqwest::Url>() {
                    jar.add_cookie_str(&format!("{}={}", c.name, c.value), &url);
                }
                break;
            }
        }
    }

    for (_, url_str) in domain_urls {
        if let Ok(url) = url_str.parse::<reqwest::Url>() {
            jar.add_cookie_str("Steam_Language=english", &url);
        }
    }

    let mut builder = reqwest::Client::builder().cookie_provider(jar.clone());

    if let Some(proxy) = proxy_url
        && let Ok(p) = reqwest::Proxy::all(proxy)
    {
        builder = builder.proxy(p);
    }

    (
        builder.build().unwrap_or_else(|_| reqwest::Client::new()),
        jar,
    )
}

pub fn default_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
    );
    h
}

pub fn short_url(url: &str) -> String {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    if stripped.len() > 55 {
        format!("{}…", &stripped[..52])
    } else {
        stripped.to_string()
    }
}

async fn execute_request(client: &reqwest::Client, req: reqwest::Request) -> Option<String> {
    let method = req.method().as_str().to_string();
    let url = req.url().to_string();

    let req_headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let start = std::time::Instant::now();
    let res = client.execute(req).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    match res {
        Ok(r) => {
            let status = r.status().as_u16();
            let res_headers: Vec<(String, String)> = r
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = r.text().await.ok();

            emit_network(NetworkRequest {
                method,
                url,
                status: Some(status),
                req_headers,
                res_headers,
                response_body: body.clone(),
                duration_ms,
            });
            body
        }
        Err(e) => {
            emit_network(NetworkRequest {
                method,
                url,
                status: None,
                req_headers,
                res_headers: vec![],
                response_body: Some(e.to_string()),
                duration_ms,
            });
            None
        }
    }
}

pub async fn fetch_text(client: &reqwest::Client, url: &str) -> Option<String> {
    let req = client.get(url).headers(default_headers()).build().ok()?;
    execute_request(client, req).await
}

pub async fn fetch_json(client: &reqwest::Client, url: &str) -> Option<serde_json::Value> {
    let req = client.get(url).headers(default_headers()).build().ok()?;
    let body_str = execute_request(client, req).await?;
    serde_json::from_str(&body_str).ok()
}

pub async fn post_text(client: &reqwest::Client, url: &str) -> Option<String> {
    let req = client.post(url).headers(default_headers()).build().ok()?;
    execute_request(client, req).await
}
