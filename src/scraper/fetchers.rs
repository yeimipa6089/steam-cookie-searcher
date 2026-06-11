use crate::models::account::AccountData;
use crate::net::client::{default_headers, fetch_json, fetch_text, post_text};
use crate::utils::regex::regex_extract;
use crate::utils::xml::xml_tag;
use regex::Regex;
use scraper::{Html, Selector};

pub async fn fetch_fundwalletinfo(client: &reqwest::Client, data: &mut AccountData) -> bool {
    let url = "https://store.steampowered.com/api/getfundwalletinfo/?l=english";
    if let Some(json) = fetch_json(client, url).await {
        if let Some(wallet) = json.get("user_wallet") {
            let amount_opt = wallet.get("amount").and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            });
            let delayed_opt = wallet
                .get("balance_delayed")
                .or_else(|| wallet.get("delayed_balance"))
                .or_else(|| wallet.get("balance_pending"))
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                });

            if let Some(amount) = amount_opt {
                let currency_code = wallet.get("currency").and_then(|v| v.as_u64()).unwrap_or(0);
                let currency = match currency_code {
                    1 => "USD",
                    2 => "GBP",
                    3 => "EUR",
                    4 => "CHF",
                    5 => "RUB",
                    6 => "PLN",
                    7 => "BRL",
                    8 => "JPY",
                    9 => "NOK",
                    10 => "IDR",
                    11 => "MYR",
                    12 => "PHP",
                    13 => "SGD",
                    14 => "THB",
                    15 => "VND",
                    16 => "KRW",
                    17 => "TRY",
                    18 => "UAH",
                    19 => "MXN",
                    20 => "CAD",
                    21 => "AUD",
                    22 => "NZD",
                    23 => "CNY",
                    24 => "INR",
                    25 => "CLP",
                    26 => "PEN",
                    27 => "COP",
                    28 => "ZAR",
                    29 => "HKD",
                    30 => "TWD",
                    31 => "SAR",
                    32 => "AED",
                    34 => "ARS",
                    35 => "ILS",
                    37 => "KZT",
                    38 => "KWD",
                    39 => "QAR",
                    40 => "CRC",
                    41 => "UYU",
                    _ => "Unknown",
                };
                let formatted = format!("{:.2} {}", amount as f64 / 100.0, currency);
                if data.wallet_balance == "—" || data.wallet_balance.is_empty() {
                    data.wallet_balance = formatted;
                }

                if let Some(delayed) = delayed_opt {
                    data.hold_balance = format!("{:.2} {}", delayed as f64 / 100.0, currency);
                } else {
                    data.hold_balance = "0.00".to_string();
                }
                data.inventory_balance = "0.00".to_string();
            }
        }
        if let Some(cc) = json.get("country_code").and_then(|v| v.as_str()) {
            data.country = cc.to_string();
        }
        return true;
    }
    false
}
pub async fn fetch_ajaxlistfriends(client: &reqwest::Client, data: &mut AccountData) {
    let url = "https://steamcommunity.com/actions/ajaxlistfriends";
    if let Some(json) = fetch_json(client, url).await
        && let Some(friends) = json.get("friends").and_then(|v| v.as_array())
    {
        data.friends_count = friends.len().to_string();
    }
}

pub async fn check_inventory_all(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let apps = [
        ("730", &mut data.inventory_cs2),
        ("570", &mut data.inventory_dota),
        ("440", &mut data.inventory_tf2),
        ("578080", &mut data.inventory_pubg),
        ("252490", &mut data.inventory_rust),
        ("753", &mut data.inventory_steam),
    ];
    for (appid, target) in apps {
        let url = format!(
            "https://steamcommunity.com/inventory/{}/{}/2",
            steam_id, appid
        );

        let mut retries = 0;
        let max_retries = 3;

        while retries < max_retries {
            if let Ok(res) = client.get(&url).headers(default_headers()).send().await {
                let _status = res.status().as_u16();
                if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    retries += 1;
                    continue;
                }

                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
                        *target = assets.len().to_string();
                    } else if json.get("error").is_some() {
                        *target = "Private/Empty".to_string();
                    } else {
                        *target = "0".to_string();
                    }
                } else {
                    *target = "Private/Empty".to_string();
                }
            } else {
                *target = "Private/Empty".to_string();
            }
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub async fn fetch_mylistings(client: &reqwest::Client, data: &mut AccountData) {
    let url = "https://steamcommunity.com/market/mylistings?norender=1";
    if let Some(json) = fetch_json(client, url).await
        && let Some(num) = json.get("num_active_listings").and_then(|v| v.as_u64())
    {
        data.market_active_listings = num.to_string();
    }
}

pub async fn auth_sih(client: &reqwest::Client, data: &mut AccountData) {
    let auth_url = "https://core.steaminventoryhelper.com/sih/auth";
    if let Some(json) = fetch_json(client, auth_url).await {
        if let Some(openid_url) = json.get("url").and_then(|v| v.as_str()) {
            if let Some(html) = post_text(client, openid_url).await {
                if html.contains("Steam Community :: Error") {
                    data.sih_status = "\x1b[31mFailed\x1b[0m (Error)".to_string();
                } else {
                    data.sih_status = "\x1b[32mLinked\x1b[0m".to_string();
                }
            } else {
                data.sih_status = "\x1b[31mFailed\x1b[0m (Connection)".to_string();
            }
        }
    } else {
        data.sih_status = "\x1b[31mFailed\x1b[0m (No URL)".to_string();
    }
}

pub async fn fetch_licenses(client: &reqwest::Client, data: &mut AccountData) {
    let url = "https://store.steampowered.com/account/licenses/?l=english";
    if let Some(html) = fetch_text(client, url).await {
        let count = html.matches(r#"<tr class="account_table_row">"#).count();
        if count > 0 {
            data.owned_apps = count.to_string();
        }
    }
}

pub async fn fetch_games_tab(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let url = format!(
        "https://steamcommunity.com/profiles/{}/games?tab=all",
        steam_id
    );
    if let Some(html) = fetch_text(client, &url).await {
        let re = Regex::new(r#"appid["']?\s*:\s*(\d+)"#).unwrap();
        let matches = re.find_iter(&html).count();
        if matches > 0 {
            data.games_count = matches.to_string();
        }

        let mut total_hours = 0.0;
        let hours_re = Regex::new(r#""hours_forever"\s*:\s*"([0-9,.]+)"#).unwrap();
        for cap in hours_re.captures_iter(&html) {
            if let Some(h) = cap.get(1) {
                let h_str = h.as_str().replace(",", "");
                if let Ok(h_f) = h_str.parse::<f64>() {
                    total_hours += h_f;
                }
            }
        }
        if total_hours > 0.0 {
            data.hours_played = format!("{:.1}h", total_hours);
        }
    }
}

pub async fn scrape_xml_profile(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let url = format!("https://steamcommunity.com/profiles/{}?xml=1", steam_id);
    let Some(xml) = fetch_text(client, &url).await else {
        return;
    };

    let username = xml_tag(&xml, "steamID");
    if !username.is_empty() {
        data.username = username;
    }

    let custom = xml_tag(&xml, "customURL");
    if !custom.is_empty() {
        data.custom_url = custom;
    }

    let member = xml_tag(&xml, "memberSince");
    if !member.is_empty() {
        data.member_since = member;
    }

    let vac = xml_tag(&xml, "vacBanned");
    data.vac = if vac == "1" {
        "✗".into()
    } else {
        "✓".into()
    };

    let trade = xml_tag(&xml, "tradeBanState");
    data.trade_ban = if trade.eq_ignore_ascii_case("none") || trade.is_empty() {
        "✓".into()
    } else {
        "✗".into()
    };

    let limited = xml_tag(&xml, "isLimitedAccount");
    data.limited = if limited == "1" {
        "✗ Limited".into()
    } else {
        "✓".into()
    };

    let privacy = xml_tag(&xml, "privacyState");
    if privacy.eq_ignore_ascii_case("private") {
        data.community_ban = "Private".into();
    }
}

pub async fn scrape_html_profile(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let url = format!("https://steamcommunity.com/profiles/{}", steam_id);
    let Some(html) = fetch_text(client, &url).await else {
        return;
    };
    let doc = Html::parse_document(&html);

    if data.username.is_empty()
        && let Some(el) = doc
            .select(&Selector::parse(".actual_persona_name").expect("Invalid selector"))
            .next()
    {
        data.username = el.text().collect::<String>().trim().to_string();
    }

    if let Some(el) = doc
        .select(&Selector::parse(".friendPlayerLevelNum").expect("Invalid selector"))
        .next()
    {
        let level = el.text().collect::<String>().trim().to_string();
        if !level.is_empty() {
            data.level = level;
        }
    }

    let games = regex_extract(
        &html,
        r"(?s)Games.*?profile_count_link_total[^>]*>\s*([\d,]+)",
    );
    if !games.is_empty() {
        data.games_count = games;
    }

    let friends = regex_extract(
        &html,
        r"(?s)Friends.*?profile_count_link_total[^>]*>\s*([\d,]+)",
    );
    if !friends.is_empty() {
        data.friends_count = friends;
    }

    let inventory = regex_extract(
        &html,
        r"(?s)Inventory.*?profile_count_link_total[^>]*>\s*([\d,]+)",
    );
    if !inventory.is_empty() {
        data.inventory_steam = inventory;
    }

    let badges = regex_extract(
        &html,
        r"(?s)Badges.*?profile_count_link_total[^>]*>\s*([\d,]+)",
    );
    if !badges.is_empty() {
        data.badges = badges;
    }

    if html.contains("profile_ban_status")
        && !regex_extract(&html, r"(?si)profile_ban.*?(community ban)").is_empty()
    {
        data.community_ban = "✗".into();
    }
}

pub async fn scrape_store_account(client: &reqwest::Client, data: &mut AccountData) {
    let Some(html) = fetch_text(client, "https://store.steampowered.com/account/").await else {
        return;
    };

    if html.contains("login") && !html.contains("account_setting") {
        return;
    }

    let email = regex_extract(
        &html,
        r#"(?s)account_setting_sub_block.*?([a-zA-Z0-9_.+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})"#,
    );
    if !email.is_empty() {
        data.email = email;
    }

    let wallet = regex_extract(&html, r#"(?s)header_wallet_balance[^>]*>\s*([^<]+)"#);
    if !wallet.is_empty() {
        data.wallet_balance = wallet;
    } else {
        let alt = regex_extract(
            &html,
            r#"(?s)accountBalance[^>]*>.*?(\$[\d,.]+|[\d,.]+\s*USD)"#,
        );
        if !alt.is_empty() {
            data.wallet_balance = alt;
        }
    }

    if html.contains("Mobile Authenticator") || html.contains("phone_verified") {
        data.guard = "Mobile".into();
    } else if html.contains("Steam Guard") && html.contains("email") {
        data.guard = "Email".into();
    } else if html.contains("steamguard_icon") {
        data.guard = "Enabled".into();
    }

    if html.contains("parental_locked") || html.contains("parental_status_locked") {
        data.family_view = "✗ Locked".into();
    } else if html.contains("parental_unlocked") || html.contains("parental_status_unlocked") {
        data.family_view = "✓ Unlocked".into();
    } else if html.contains("parental_notice")
        || html.contains("header_parental_link")
        || html.contains("data-parental")
    {
        data.family_view = "Enabled".into();
    } else {
        data.family_view = "—".into();
    }

    let country = regex_extract(&html, r#"store_country[^"]*"[^"]*value="([^"]+)"#);
    if !country.is_empty() {
        data.country = country;
    }

    let phone = regex_extract(&html, r#"(?s)phone.*?account_data_field[^>]*>\s*([^<]+)"#);
    if !phone.is_empty() {
        data.phone = phone.trim().to_string();
    }
}

pub async fn fetch_dynamic_userdata(client: &reqwest::Client, data: &mut AccountData) {
    let Some(json) = fetch_json(
        client,
        "https://store.steampowered.com/dynamicstore/userdata/",
    )
    .await
    else {
        return;
    };

    if let Some(apps) = json.get("rgOwnedApps").and_then(|v| v.as_array()) {
        data.owned_apps = apps.len().to_string();
    }
    if let Some(wl) = json.get("rgWishlist").and_then(|v| v.as_array()) {
        data.wishlist_count = wl.len().to_string();
    }
}

pub async fn fetch_name_history(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let url = format!(
        "https://steamcommunity.com/profiles/{}/ajaxaliases/",
        steam_id
    );
    if let Some(json) = fetch_json(client, &url).await
        && let Some(arr) = json.as_array()
    {
        data.name_history = arr
            .iter()
            .filter_map(|v| {
                let name = v.get("newname")?.as_str()?.to_string();
                let time = v
                    .get("timechanged")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                Some((name, time))
            })
            .collect();
    }
}

pub async fn fetch_steam_points(client: &reqwest::Client, data: &mut AccountData) {
    let url = "https://store.steampowered.com/pointssummary/ajaxgetasyncconfig";
    if let Some(json) = fetch_json(client, url).await {
        let points = json
            .pointer("/data/summary/points")
            .or_else(|| json.pointer("/data/points"))
            .and_then(|v| {
                v.as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| v.as_str().map(|s| s.to_string()))
            });
        if let Some(p) = points {
            data.steam_points = p;
        }
    }

    if data.steam_points.is_empty()
        && let Some(html) = fetch_text(client, "https://store.steampowered.com/points/shop/").await
    {
        let pts = regex_extract(&html, r#"(?s)loyaltypoints_amount[^>]*>\s*([\d,]+)"#);
        if !pts.is_empty() {
            data.steam_points = pts;
        }
    }
}

pub async fn check_market(client: &reqwest::Client, data: &mut AccountData) {
    let Some(html) = fetch_text(client, "https://steamcommunity.com/market/").await else {
        return;
    };

    if html.contains("market_listing_table")
        || html.contains("market_search_sidebar")
        || html.contains("market_headertip_container")
    {
        data.market = "✓".into();
    } else {
        data.market = "✗".into();
    }
}

pub async fn check_cs_prime(client: &reqwest::Client, steam_id: &str, data: &mut AccountData) {
    let url = format!("https://steamcommunity.com/profiles/{}/gcpd/730", steam_id);
    let Some(html) = fetch_text(client, &url).await else {
        return;
    };

    if html.contains("Prime Status") || html.contains("prime") {
        data.cs_prime = "✓".into();
    } else if html.contains("gcpd") || html.contains("personal_game_data") {
        data.cs_prime = "✘".into();
    }
}
