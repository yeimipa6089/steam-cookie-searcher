pub mod auth;
pub mod browser;
pub mod cli;
pub mod models;
pub mod net;
pub mod parser;
pub mod scraper;
pub mod utils;

use browser::chromedriver::setup_chromedriver;
use cli::app::{App, AppMode};
use cli::tui::{init_terminal, restore_terminal};
use cli::ui::draw_ui;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures::stream::{self, StreamExt};
use models::proxy::ProxyInfo;
use parser::input::parse_netscape_lines;
use parser::input::process_bulk_input;
use scraper::orchestrator::scrape_all_data;
use tokio::sync::mpsc;

fn parse_proxies_from_text(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && l.contains(':') && !l.contains(' '))
        .collect()
}

async fn load_proxies_from_path(
    path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let metadata = tokio::fs::metadata(path).await?;
    let path_obj = std::path::Path::new(path);
    let mut list = Vec::new();

    if metadata.is_file() {
        if path_obj
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            list.extend(load_proxies_from_zip(path)?);
        } else {
            let content = tokio::fs::read_to_string(path).await?;
            list.extend(parse_proxies_from_text(&content));
        }
    } else if metadata.is_dir() {
        list.extend(load_proxies_from_dir(path).await?);
    } else {
        return Err("Invalid path".into());
    }
    Ok(list)
}

fn load_proxies_from_zip(
    path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut list = Vec::new();
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            let mut contents = String::new();
            use std::io::Read;
            if file.read_to_string(&mut contents).is_ok() {
                list.extend(parse_proxies_from_text(&contents));
            }
        }
    }
    Ok(list)
}

async fn load_proxies_from_dir(
    path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut list = Vec::new();
    let mut entries = tokio::fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        if entry_path.is_file()
            && entry_path.extension().is_some_and(|ext| ext == "txt")
            && let Ok(content) = tokio::fs::read_to_string(&entry_path).await
        {
            list.extend(parse_proxies_from_text(&content));
        }
    }
    Ok(list)
}

async fn test_proxies(proxies: Vec<String>, tx: mpsc::UnboundedSender<String>) -> Vec<ProxyInfo> {
    if proxies.is_empty() {
        return Vec::new();
    }
    let _ = tx.send(format!("Testing {} proxies...", proxies.len()));
    let stream = stream::iter(proxies)
        .map(|p| {
            let tx = tx.clone();
            async move {
                let res = crate::utils::ip::check_proxy_geoip(p.clone()).await;
                if let Some(ref info) = res {
                    let _ = tx.send(format!("Proxy alive: {} ({})", p, info.country_code));
                }
                res
            }
        })
        .buffer_unordered(30);

    let results: Vec<Option<ProxyInfo>> = stream.collect().await;
    let working: Vec<ProxyInfo> = results.into_iter().flatten().collect();
    let _ = tx.send(format!("Proxy check complete: {} alive", working.len()));
    working
}

fn save_accounts(accounts: &[crate::models::account::AccountData]) {
    if let Ok(serialized) = serde_json::to_string_pretty(accounts) {
        let _ = std::fs::write("accounts.json", serialized);
    }
}

fn load_accounts() -> Vec<crate::models::account::AccountData> {
    if let Ok(data) = std::fs::read_to_string("accounts.json")
        && let Ok(accounts) = serde_json::from_str(&data)
    {
        return accounts;
    }
    Vec::new()
}

use crate::models::network::NetworkRequest;

fn handle_tui_events(
    app: &mut App,
    event: crossterm::event::Event,
    log_tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
    proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    match event {
        Event::Paste(text) => {
            if app.mode == AppMode::InputPath
                || app.mode == AppMode::InputProxyPath
                || app.mode == AppMode::PasteText
                || app.mode == AppMode::PasteProxyText
            {
                app.input_buffer.push_str(&text);
            }
        }
        Event::Mouse(mouse_event) => {
            if app.mode == AppMode::Normal || app.mode == AppMode::Scanning {
                match mouse_event.kind {
                    crossterm::event::MouseEventKind::ScrollUp => {
                        if app.active_tab == crate::cli::app::AppTab::System {
                            let i = app
                                .logs_state
                                .selected()
                                .unwrap_or(app.logs.len().saturating_sub(1));
                            app.logs_state.select(Some(i.saturating_sub(1)));
                        } else {
                            app.network_scroll = app.network_scroll.saturating_sub(1);
                        }
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        if app.active_tab == crate::cli::app::AppTab::System {
                            let i = app
                                .logs_state
                                .selected()
                                .unwrap_or(app.logs.len().saturating_sub(1));
                            app.logs_state
                                .select(Some((i + 1).min(app.logs.len().saturating_sub(1))));
                        } else {
                            app.network_scroll = app.network_scroll.saturating_add(1);
                        }
                    }
                    _ => {}
                }
            }
        }
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            handle_key_events(app, key, log_tx, data_tx, proxy_tx);
        }
        _ => {}
    }
}

fn handle_key_events(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
    proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    match app.mode {
        AppMode::Normal | AppMode::Scanning => handle_normal_mode_keys(app, key, log_tx),
        AppMode::SelectCookiesMethod => handle_cookie_method_keys(app, key, log_tx, data_tx),
        AppMode::SelectProxiesMethod => handle_proxy_method_keys(app, key, log_tx, proxy_tx),
        AppMode::InputPath => handle_input_path_keys(app, key, log_tx, data_tx),
        AppMode::InputProxyPath => handle_input_proxy_path_keys(app, key, log_tx, proxy_tx),
        AppMode::PasteText => handle_paste_cookies_keys(app, key, log_tx, data_tx),
        AppMode::PasteProxyText => handle_paste_proxies_keys(app, key, log_tx, proxy_tx),
    }
}

fn handle_normal_mode_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Tab => {
            if app.active_tab == crate::cli::app::AppTab::System {
                app.active_tab = crate::cli::app::AppTab::Network;
            } else {
                app.active_tab = crate::cli::app::AppTab::System;
            }
        }
        KeyCode::Down => {
            if app.active_tab == crate::cli::app::AppTab::Network {
                if !app.network_requests.is_empty() {
                    app.selected_network_request =
                        (app.selected_network_request + 1) % app.network_requests.len();
                    app.network_scroll = 0;
                    app.network_state.select(Some(app.selected_network_request));
                }
            } else {
                app.next_account();
            }
        }
        KeyCode::Up => {
            if app.active_tab == crate::cli::app::AppTab::Network {
                if !app.network_requests.is_empty() {
                    if app.selected_network_request > 0 {
                        app.selected_network_request -= 1;
                    } else {
                        app.selected_network_request = app.network_requests.len() - 1;
                    }
                    app.network_scroll = 0;
                    app.network_state.select(Some(app.selected_network_request));
                }
            } else {
                app.prev_account();
            }
        }
        KeyCode::Right => {
            if app.active_tab == crate::cli::app::AppTab::Network {
                app.network_inner_tab = match app.network_inner_tab {
                    crate::cli::app::InnerNetworkTab::Headers => {
                        crate::cli::app::InnerNetworkTab::Body
                    }
                    crate::cli::app::InnerNetworkTab::Body => {
                        crate::cli::app::InnerNetworkTab::Cookies
                    }
                    crate::cli::app::InnerNetworkTab::Cookies => {
                        crate::cli::app::InnerNetworkTab::Headers
                    }
                };
                app.network_scroll = 0;
            }
        }
        KeyCode::Left => {
            if app.active_tab == crate::cli::app::AppTab::Network {
                app.network_inner_tab = match app.network_inner_tab {
                    crate::cli::app::InnerNetworkTab::Headers => {
                        crate::cli::app::InnerNetworkTab::Cookies
                    }
                    crate::cli::app::InnerNetworkTab::Body => {
                        crate::cli::app::InnerNetworkTab::Headers
                    }
                    crate::cli::app::InnerNetworkTab::Cookies => {
                        crate::cli::app::InnerNetworkTab::Body
                    }
                };
                app.network_scroll = 0;
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => app.mode = AppMode::SelectCookiesMethod,
        KeyCode::Char('p') | KeyCode::Char('P') => app.mode = AppMode::SelectProxiesMethod,
        KeyCode::Char('o') | KeyCode::Char('O') => {
            if !app.accounts.is_empty() {
                let acc = &app.accounts[app.selected_account];
                let cookies = acc.cookies.clone();
                let tx = log_tx.clone();
                let proxy = app.proxies.first().map(|p| p.url.clone());
                tokio::spawn(async move {
                    let proxy_ref = proxy.as_deref();
                    if let Err(e) =
                        crate::browser::cdp::open_browser_session(&cookies, &tx, proxy_ref).await
                    {
                        let _ = tx.send(format!("Browser error: {}", e));
                    }
                });
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') if !app.accounts.is_empty() => {
            app.accounts.remove(app.selected_account);
            if app.selected_account >= app.accounts.len() && app.selected_account > 0 {
                app.selected_account -= 1;
            }
            app.account_state.select(Some(app.selected_account));
            save_accounts(&app.accounts);
        }
        KeyCode::Char('e') | KeyCode::Char('E') => export_accounts(app),
        KeyCode::Char('i') | KeyCode::Char('I') => import_accounts(app),
        KeyCode::PageUp => {
            let inner_h = 10;
            if app.active_tab == crate::cli::app::AppTab::System {
                let i = app
                    .logs_state
                    .selected()
                    .unwrap_or(app.logs.len().saturating_sub(1));
                app.logs_state.select(Some(i.saturating_sub(inner_h)));
            } else if app.active_tab == crate::cli::app::AppTab::Network {
                app.network_scroll = app.network_scroll.saturating_sub(inner_h);
            }
        }
        KeyCode::PageDown => {
            let inner_h = 10;
            if app.active_tab == crate::cli::app::AppTab::System {
                let i = app
                    .logs_state
                    .selected()
                    .unwrap_or(app.logs.len().saturating_sub(1));
                app.logs_state
                    .select(Some((i + inner_h).min(app.logs.len().saturating_sub(1))));
            } else if app.active_tab == crate::cli::app::AppTab::Network {
                app.network_scroll += inner_h;
            }
        }
        _ => {}
    }
}

fn export_accounts(app: &mut App) {
    let _ = std::fs::create_dir_all("Export/Cookies");
    let mut exported = 0;
    for (i, acc) in app.accounts.iter().enumerate() {
        let mut text = String::new();
        text.push_str(&format!("Login: {}\n", acc.display("username")));
        text.push_str(&format!("Profile: {}\n", acc.display("profile_url")));
        text.push_str(&format!("Custom URL: {}\n", acc.display("custom_url")));
        text.push_str(&format!("Steam ID: {}\n", acc.display("steam_id")));
        text.push_str(&format!("Balance: {}\n", acc.display("wallet_balance")));
        text.push_str(&format!("Hold Balance: {}\n", acc.display("hold_balance")));
        text.push_str(&format!(
            "Inventory Balance: {}\n",
            acc.display("inventory_balance")
        ));
        text.push_str(&format!("Email: {}\n", acc.display("email")));
        text.push_str(&format!("Phone: {}\n", acc.display("phone")));
        text.push_str(&format!("Country: {}\n", acc.display("country")));
        text.push_str(&format!("Family View: {}\n", acc.display("family_view")));
        text.push_str(&format!("Created: {}\n", acc.display("created_date")));
        text.push_str(&format!("Guard: {}\n", acc.display("guard")));
        text.push_str(&format!("Level: {}\n", acc.display("level")));
        text.push_str(&format!("Points: {}\n", acc.display("steam_points")));
        text.push_str(&format!(
            "Community Status: {}\n",
            acc.display("community_ban")
        ));
        text.push_str(&format!("Trade Ban: {}\n", acc.display("trade_ban")));
        text.push_str(&format!("Account Type: {}\n", acc.display("account_type")));
        text.push_str(&format!("Market Status: {}\n", acc.display("market")));
        text.push_str(&format!(
            "Active Lots For Sale: {}\n",
            acc.display("market_active_listings")
        ));
        text.push_str(&format!("VAC Status: {}\n", acc.display("vac")));
        text.push_str(&format!("CS Prime: {}\n", acc.display("cs_prime")));
        text.push_str(&format!("SIH Link: {}\n", acc.display("sih_link")));
        text.push_str(&format!("Games: {}\n", acc.display("games_count")));
        text.push_str(&format!("Hours Played: {}\n", acc.display("hours_played")));
        text.push_str(&format!("Friends: {}\n", acc.display("friends_count")));
        text.push_str(&format!("Wishlist: {}\n", acc.display("wishlist_count")));
        text.push_str(&format!(
            "Active Sales: {}\n",
            acc.display("active_sales_count")
        ));
        text.push_str(&format!("Badges: {}\n", acc.display("badges_count")));
        text.push_str("\n----- Game Inventories -----\n");
        text.push_str(&format!("CS2: {}\n", acc.display("inv_cs2")));
        text.push_str(&format!("Dota 2: {}\n", acc.display("inv_dota2")));
        text.push_str(&format!("TF2: {}\n", acc.display("inv_tf2")));
        text.push_str(&format!("PUBG: {}\n", acc.display("inv_pubg")));
        text.push_str(&format!("Rust: {}\n", acc.display("inv_rust")));
        text.push_str(&format!("Steam: {}\n", acc.display("inv_steam")));
        text.push_str("-----Inventory-----\nInventory Empty\n\n---------------------------\n");
        for cookie in &acc.cookies {
            let secure = if cookie.secure { "TRUE" } else { "FALSE" };
            text.push_str(&format!(
                "{}\tFALSE\t/\t{}\t1764547200\t{}\t{}\n",
                cookie.domain, secure, cookie.name, cookie.value
            ));
        }
        text.push_str("---------------------------\n");
        let name = if acc.username.is_empty() || acc.username == "—" {
            format!("Account_{}", i + 1)
        } else {
            acc.username.clone()
        };
        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let filepath = format!("Export/Cookies/{}.txt", safe_name);
        if std::fs::write(&filepath, text).is_ok() {
            exported += 1;
        }
    }
    app.add_log(format!(
        "Exported {} accounts to Export/Cookies/ folder in BL Tools format",
        exported
    ));
}

fn import_accounts(app: &mut App) {
    if let Ok(data) = std::fs::read_to_string("accounts_export.json") {
        if let Ok(imported) =
            serde_json::from_str::<Vec<crate::models::account::AccountData>>(&data)
        {
            let mut new_count = 0;
            for acc in imported {
                if !app.accounts.iter().any(|a| a.steam_id == acc.steam_id) {
                    app.accounts.push(acc);
                    new_count += 1;
                }
            }
            save_accounts(&app.accounts);
            app.add_log(format!(
                "Imported {} new accounts from accounts_export.json",
                new_count
            ));
        } else {
            app.add_log("Failed to parse accounts_export.json".to_string());
        }
    } else {
        app.add_log("accounts_export.json not found".to_string());
    }
}

fn handle_cookie_method_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    _log_tx: &mpsc::UnboundedSender<String>,
    _data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
) {
    match key.code {
        KeyCode::Esc => app.mode = AppMode::Normal,
        KeyCode::Char('1') => {
            app.input_buffer.clear();
            app.mode = AppMode::InputPath;
        }
        KeyCode::Char('2') => {
            app.input_buffer.clear();
            app.mode = AppMode::PasteText;
        }
        _ => {}
    }
}

fn handle_proxy_method_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    _log_tx: &mpsc::UnboundedSender<String>,
    _proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    match key.code {
        KeyCode::Esc => app.mode = AppMode::Normal,
        KeyCode::Char('1') => {
            app.input_buffer.clear();
            app.mode = AppMode::InputProxyPath;
        }
        KeyCode::Char('2') => {
            app.input_buffer.clear();
            app.mode = AppMode::PasteProxyText;
        }
        _ => {}
    }
}

fn handle_input_path_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
) {
    match key.code {
        KeyCode::Esc => app.mode = AppMode::Normal,
        KeyCode::Enter => {
            let input = app
                .input_buffer
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !input.is_empty() {
                app.mode = AppMode::Scanning;
                app.add_log(format!("scanning: {}", input));
                let tx = log_tx.clone();
                let d_tx = data_tx.clone();
                let current_proxies = app.proxies.clone();
                tokio::spawn(async move {
                    let all_cookies = match process_bulk_input(&input) {
                        Ok(results) => results,
                        Err(e) => {
                            let _ = tx.send(format!("error: {}", e));
                            return;
                        }
                    };
                    process_cookie_results(all_cookies, &tx, &d_tx, current_proxies).await;
                });
            } else {
                app.mode = AppMode::Normal;
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Tab => {
            app.input_buffer.push('\t');
        }
        _ => {}
    }
}

fn handle_input_proxy_path_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
    proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    match key.code {
        KeyCode::Esc => app.mode = AppMode::Normal,
        KeyCode::Enter => {
            let input = app
                .input_buffer
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !input.is_empty() {
                app.mode = AppMode::Scanning;
                app.add_log(format!("loading proxies from path: {}", input));
                let tx = log_tx.clone();
                let p_tx = proxy_tx.clone();
                tokio::spawn(async move {
                    match load_proxies_from_path(&input).await {
                        Ok(proxies_list) => {
                            let working = test_proxies(proxies_list, tx).await;
                            let _ = p_tx.send(working);
                        }
                        Err(e) => {
                            let _ = tx.send(format!("error: {}", e));
                        }
                    }
                });
            } else {
                app.mode = AppMode::Normal;
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Tab => {
            app.input_buffer.push('\t');
        }
        _ => {}
    }
}

fn submit_pasted_cookies(
    app: &mut App,
    input: String,
    log_tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
) {
    if !input.trim().is_empty() {
        app.mode = AppMode::Scanning;
        app.add_log("scanning pasted cookies...".to_string());
        let tx = log_tx.clone();
        let d_tx = data_tx.clone();
        let current_proxies = app.proxies.clone();
        tokio::spawn(async move {
            let lines = input.lines().map(|s| s.to_string());
            let parsed = parse_netscape_lines(lines);
            if !parsed.is_empty() {
                let grouped = crate::parser::input::group_cookies_by_account(parsed);
                let mut all_cookies = Vec::new();
                for (id, cookie_group) in grouped {
                    all_cookies.push((format!("Pasted ({})", id), cookie_group));
                }
                process_cookie_results(all_cookies, &tx, &d_tx, current_proxies).await;
            } else {
                let _ = tx.send("error: no valid cookies found in pasted text".to_string());
            }
        });
    } else {
        app.mode = AppMode::Normal;
    }
}

fn handle_paste_cookies_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
) {
    match key.code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            app.input_buffer.push('\n');
            if app.input_buffer.ends_with("\n\n")
                && !crossterm::event::poll(std::time::Duration::from_millis(5)).unwrap_or(false) {
                    let input = app.input_buffer.clone();
                    submit_pasted_cookies(app, input, log_tx, data_tx);
                }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Tab => {
            app.input_buffer.push('\t');
        }
        _ => {}
    }
}

fn submit_pasted_proxies(
    app: &mut App,
    input: String,
    log_tx: &mpsc::UnboundedSender<String>,
    proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    if !input.trim().is_empty() {
        app.mode = AppMode::Scanning;
        app.add_log("verifying pasted proxies...".to_string());
        let tx = log_tx.clone();
        let p_tx = proxy_tx.clone();
        tokio::spawn(async move {
            let lines = parse_proxies_from_text(&input);
            let working = test_proxies(lines, tx).await;
            let _ = p_tx.send(working);
        });
    } else {
        app.mode = AppMode::Normal;
    }
}

fn handle_paste_proxies_keys(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    log_tx: &mpsc::UnboundedSender<String>,
    proxy_tx: &mpsc::UnboundedSender<Vec<ProxyInfo>>,
) {
    match key.code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            app.input_buffer.push('\n');
            if app.input_buffer.ends_with("\n\n")
                && !crossterm::event::poll(std::time::Duration::from_millis(5)).unwrap_or(false) {
                    let input = app.input_buffer.clone();
                    submit_pasted_proxies(app, input, log_tx, proxy_tx);
                }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Tab => {
            app.input_buffer.push('\t');
        }
        _ => {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = init_terminal()?;
    let mut app = App::new();
    app.accounts = load_accounts();

    let (log_tx, mut log_rx) = mpsc::unbounded_channel::<String>();
    let (net_tx, mut net_rx) = mpsc::unbounded_channel::<NetworkRequest>();
    let (data_tx, mut data_rx) = mpsc::unbounded_channel::<crate::models::account::AccountData>();
    let (proxy_tx, mut proxy_rx) = mpsc::unbounded_channel::<Vec<ProxyInfo>>();

    net::client::set_global_log_tx(log_tx.clone());
    net::client::set_global_network_tx(net_tx.clone());

    let exe_name = if std::env::consts::OS == "windows" {
        "chromedriver.exe"
    } else {
        "chromedriver"
    };
    let exe_path = std::env::current_dir()?.join(exe_name);

    let _ = setup_chromedriver(&log_tx).await;

    let driver_result = std::process::Command::new(&exe_path)
        .arg("--port=9515")
        .arg("--silent")
        .arg("--log-level=OFF")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut driver_process = driver_result.ok();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let _chromedriver_installed = std::path::Path::new(exe_name).exists();

    let accounts_to_validate = app.accounts.clone();
    let val_tx = data_tx.clone();
    let val_log_tx = log_tx.clone();
    tokio::spawn(async move {
        for mut acc in accounts_to_validate {
            let name = acc.display("username").to_string();
            let display_name = if name.is_empty() || name == "—" {
                acc.steam_id.clone()
            } else {
                name
            };

            let _ = val_log_tx.send(format!(
                "Validating & refreshing session for account: {}...",
                display_name
            ));

            if let Some((mut new_data, _)) =
                crate::scraper::orchestrator::scrape_all_data(&mut acc.cookies, &val_log_tx, &[])
                    .await
            {
                new_data.cookies = acc.cookies;
                let _ = val_log_tx
                    .send("  -> Completed: Session is valid and data refreshed.".to_string());
                let _ = val_tx.send(new_data);
            } else {
                acc.is_valid = false;
                let _ = val_log_tx.send(format!(
                    "  -> [Error] Session expired for: {}",
                    display_name
                ));
                let _ = val_tx.send(acc);
            }
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        }
    });

    loop {
        app.tick();
        terminal.draw(|f| draw_ui(f, &mut app))?;

        while let Ok(msg) = log_rx.try_recv() {
            app.add_log(msg);
        }

        while let Ok(req) = net_rx.try_recv() {
            app.network_requests.push(req);
            if app.network_requests.len() > 1000 {
                app.network_requests.remove(0);
            }
            app.selected_network_request = app.network_requests.len().saturating_sub(1);
            app.network_state.select(Some(app.selected_network_request));
        }

        while let Ok(account) = data_rx.try_recv() {
            if let Some(pos) = app
                .accounts
                .iter()
                .position(|a| a.steam_id == account.steam_id)
            {
                app.accounts[pos] = account;
            } else {
                app.accounts.push(account);
            }
            save_accounts(&app.accounts);
            if app.mode == AppMode::Scanning {
                app.mode = AppMode::Normal;
            }
        }

        while let Ok(working_proxies) = proxy_rx.try_recv() {
            app.proxies.extend(working_proxies);
            app.proxies.dedup_by(|a, b| a.url == b.url);
            if app.mode == AppMode::Scanning {
                app.mode = AppMode::Normal;
            }
        }

        if event::poll(std::time::Duration::from_millis(16))? {
            loop {
                handle_tui_events(&mut app, event::read()?, &log_tx, &data_tx, &proxy_tx);
                if !event::poll(std::time::Duration::from_millis(0))? {
                    break;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal()?;
    if let Some(ref mut proc) = driver_process {
        proc.kill().ok();
    }

    Ok(())
}

async fn process_cookie_results(
    all_cookies: Vec<(String, Vec<crate::models::cookie::SteamCookie>)>,
    tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
    proxies: Vec<ProxyInfo>,
) {
    if all_cookies.is_empty() {
        let _ = tx.send("no cookies found".to_string());
        return;
    }

    let split_cookies = split_cookie_accounts(all_cookies);
    let total = split_cookies.len();
    let _ = tx.send(format!("found {} account(s)", total));

    validate_cookie_accounts(split_cookies, tx, data_tx, proxies).await;
}

fn split_cookie_accounts(
    all_cookies: Vec<(String, Vec<crate::models::cookie::SteamCookie>)>,
) -> Vec<(String, Vec<crate::models::cookie::SteamCookie>)> {
    let mut split_cookies = Vec::new();
    for (source, cookies) in all_cookies {
        split_cookies.extend(split_single_source(source, cookies));
    }
    split_cookies
}

fn split_single_source(
    source: String,
    cookies: Vec<crate::models::cookie::SteamCookie>,
) -> Vec<(String, Vec<crate::models::cookie::SteamCookie>)> {
    let mut split_cookies = Vec::new();
    let mut current_account = Vec::new();
    let mut sub_idx = 1;
    let mut current_steam_id: Option<String> = None;

    for cookie in cookies {
        if cookie.name.eq_ignore_ascii_case("steamLoginSecure") {
            let val = &cookie.value;
            let id = val
                .find("%7C%7C")
                .or_else(|| val.find("||"))
                .map(|idx| val[..idx].to_string());

            if let Some(new_id) = id {
                if let Some(ref current_id) = current_steam_id {
                    if new_id != *current_id {
                        split_cookies.push((
                            format!("{} (#{})", source, sub_idx),
                            std::mem::take(&mut current_account),
                        ));
                        sub_idx += 1;
                        current_steam_id = Some(new_id);
                    }
                } else {
                    current_steam_id = Some(new_id);
                }
            }
        }
        current_account.push(cookie);
    }
    if current_account
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case("steamLoginSecure"))
    {
        let name = if sub_idx > 1 {
            format!("{} (#{})", source, sub_idx)
        } else {
            source
        };
        split_cookies.push((name, current_account));
    }
    split_cookies
}

async fn validate_cookie_accounts(
    split_cookies: Vec<(String, Vec<crate::models::cookie::SteamCookie>)>,
    tx: &mpsc::UnboundedSender<String>,
    data_tx: &mpsc::UnboundedSender<crate::models::account::AccountData>,
    proxies: Vec<ProxyInfo>,
) {
    let total = split_cookies.len();
    for (i, (name, mut cookies)) in split_cookies.into_iter().enumerate() {
        let _ = tx.send(String::new());
        let _ = tx.send(format!(
            "[{}/{}] Validating session for account: {}...",
            i + 1,
            total,
            name
        ));
        if let Some((data, _)) = scrape_all_data(&mut cookies, tx, &proxies).await {
            let _ = data_tx.send(data);
            let _ = tx.send(format!("[{}/{}] Completed: {}", i + 1, total, name));
        } else {
            let _ = tx.send(format!("[{}/{}] Failed: {}", i + 1, total, name));
        }
    }
}
