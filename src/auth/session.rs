use crate::models::cookie::SteamCookie;
use crate::net::client::fetch_text;
use reqwest::cookie::{CookieStore, Jar};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub async fn refresh_session_cookies(
    client: &reqwest::Client,
    jar: &Arc<Jar>,
    cookies: &mut Vec<SteamCookie>,
    tx: &UnboundedSender<String>,
) {
    if !cookies.iter().any(|c| c.name == "steamRefresh_steam") {
        return;
    }
    let _ = tx.send("Requesting fresh session via Steam API...".into());
    let refresh_url = "https://login.steampowered.com/jwt/refresh?redir=https%3A%2F%2Fstore.steampowered.com%2Faccount%2F";
    let _ = fetch_text(client, refresh_url).await;

    if let Ok(store_url) = "https://store.steampowered.com".parse::<reqwest::Url>()
        && let Some(cookie_header) = jar.cookies(&store_url)
        && let Ok(cookie_str) = cookie_header.to_str()
    {
        for part in cookie_str.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("steamLoginSecure=") {
                if let Some(existing) = cookies.iter_mut().find(|c| c.name == "steamLoginSecure") {
                    existing.value = val.to_string();
                } else {
                    cookies.push(SteamCookie {
                        domain: "steamcommunity.com".to_string(),
                        name: "steamLoginSecure".to_string(),
                        value: val.to_string(),
                        secure: true,
                    });
                }
            }
        }
    }
}
