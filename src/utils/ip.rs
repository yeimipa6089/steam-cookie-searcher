use crate::models::proxy::ProxyInfo;

pub async fn get_ip_country(ip: &str) -> String {
    if let Ok(res) = reqwest::get(&format!("https://get.geojs.io/v1/ip/country/{}.json", ip)).await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(cc) = json["country"].as_str()
    {
        return cc.to_string();
    }
    "UNKNOWN".to_string()
}

pub async fn check_proxy_geoip(proxy: String) -> Option<ProxyInfo> {
    let mut clean_proxy = proxy.clone();
    if !clean_proxy.starts_with("http") && !clean_proxy.starts_with("socks") {
        clean_proxy = format!("http://{}", clean_proxy);
    }

    let client_builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5));

    let client = if let Ok(p) = reqwest::Proxy::all(&clean_proxy) {
        client_builder
            .proxy(p)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    } else {
        return None;
    };

    if let Ok(res) = client
        .get("https://get.geojs.io/v1/ip/country.json")
        .send()
        .await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(cc) = json["country"].as_str()
    {
        return Some(ProxyInfo {
            url: clean_proxy,
            country_code: cc.to_string(),
        });
    }
    None
}
