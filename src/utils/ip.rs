use crate::models::proxy::ProxyInfo;

async fn parse_country_response(res: Result<reqwest::Response, reqwest::Error>) -> Option<String> {
    if let Ok(response) = res
        && let Ok(json) = response.json::<serde_json::Value>().await
        && let Some(cc) = json["country"].as_str()
    {
        return Some(cc.to_string());
    }
    None
}

pub async fn get_ip_country(ip: &str) -> String {
    let res = reqwest::get(&format!("https://get.geojs.io/v1/ip/country/{}.json", ip)).await;
    parse_country_response(res).await.unwrap_or_else(|| "UNKNOWN".to_string())
}

pub async fn check_proxy_geoip(proxy: String) -> Option<ProxyInfo> {
    let clean_proxy = if proxy.starts_with("http") || proxy.starts_with("socks") {
        proxy.clone()
    } else {
        format!("http://{}", proxy)
    };

    let p = reqwest::Proxy::all(&clean_proxy).ok()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .proxy(p)
        .build()
        .ok()?;

    let res = client.get("https://get.geojs.io/v1/ip/country.json").send().await;

    parse_country_response(res).await.map(|cc| ProxyInfo {
        url: clean_proxy,
        country_code: cc,
    })
}
