use crate::auth::jwt::extract_token_info;
use crate::auth::session::refresh_session_cookies;
use crate::models::account::AccountData;
use crate::models::cookie::SteamCookie;
use crate::models::proxy::ProxyInfo;
use crate::net::client::build_http_client;
use crate::scraper::fetchers::*;
use tokio::sync::mpsc::UnboundedSender;

pub async fn scrape_all_data(
    cookies: &mut Vec<SteamCookie>,
    tx: &UnboundedSender<String>,
    proxies: &[ProxyInfo],
) -> Option<(AccountData, Option<String>)> {
    let steam_id = extract_steam_id(cookies)?;
    let mut data = AccountData::new(steam_id.clone());

    let _ = tx.send("  -> Decoding JWT tokens & resolving GeoIP...".into());
    extract_token_info(cookies, &mut data).await;

    let mut selected_proxy = None;
    if !proxies.is_empty() {
        if !data.token_country.is_empty()
            && data.token_country != "UNKNOWN"
            && let Some(p) = proxies
                .iter()
                .find(|p| p.country_code == data.token_country)
        {
            selected_proxy = Some(p.url.clone());
        }
        if selected_proxy.is_none() {
            selected_proxy = Some(proxies[0].url.clone());
        }
    }

    let (client, cookie_jar) = build_http_client(cookies, selected_proxy.as_deref());
    refresh_session_cookies(&client, &cookie_jar, cookies, tx).await;

    let _ = tx.send("  -> Fetching profile (XML)...".into());
    scrape_xml_profile(&client, &steam_id, &mut data).await;

    let _ = tx.send("  -> Fetching profile (HTML)...".into());
    scrape_html_profile(&client, &steam_id, &mut data).await;

    let _ = tx.send("  -> Fetching account details...".into());
    scrape_store_account(&client, &mut data).await;

    let _ = tx.send("  -> Fetching accurate wallet & friends...".into());
    fetch_fundwalletinfo(&client, &mut data).await;
    fetch_ajaxlistfriends(&client, &mut data).await;

    let _ = tx.send("  -> Fetching Steam Points...".into());
    fetch_steam_points(&client, &mut data).await;

    let _ = tx.send("  -> Checking Market & listings...".into());
    check_market(&client, &mut data).await;
    fetch_mylistings(&client, &mut data).await;

    let _ = tx.send("  -> Scanning inventory...".into());
    check_inventory_all(&client, &steam_id, &mut data).await;

    let _ = tx.send("  -> Fetching licenses & games...".into());
    fetch_licenses(&client, &mut data).await;
    fetch_games_tab(&client, &steam_id, &mut data).await;
    fetch_dynamic_userdata(&client, &mut data).await;

    let _ = tx.send("  -> Checking CS2 Prime status...".into());
    check_cs_prime(&client, &steam_id, &mut data).await;

    let _ = tx.send("  -> Attempting SIH linkage...".into());
    auth_sih(&client, &mut data).await;

    let _ = tx.send("  -> Checking previous names...".into());
    fetch_name_history(&client, &steam_id, &mut data).await;

    let _ = tx.send("  -> Data extracted successfully.".into());

    if data.profile_url.is_empty() {
        data.profile_url = format!("https://steamcommunity.com/profiles/{}/", steam_id);
    }

    data.cookies = cookies.clone();

    Some((data, selected_proxy))
}

pub fn extract_steam_id(cookies: &[SteamCookie]) -> Option<String> {
    cookies
        .iter()
        .find(|c| c.name == "steamLoginSecure")
        .and_then(|c| {
            if let Some(idx) = c.value.find("%7C%7C") {
                Some(c.value[..idx].to_string())
            } else {
                c.value.find("||").map(|idx| c.value[..idx].to_string())
            }
        })
}
