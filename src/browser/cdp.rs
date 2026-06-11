use crate::models::cookie::SteamCookie;
use reqwest::cookie::CookieStore;
use std::sync::Arc;
use thirtyfour::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

pub async fn open_browser_session(
    cookies: &[SteamCookie],
    tx: &UnboundedSender<String>,
    proxy_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = tx.send("Launching headless Chromium instance...".into());
    let mut caps = DesiredCapabilities::chrome();

    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;
    caps.add_arg("--log-level=3")?;
    caps.add_arg("--silent")?;
    caps.add_arg("--disable-logging")?;

    if let Some(proxy) = proxy_url {
        let chrome_proxy = proxy
            .replace("socks5://", "")
            .replace("socks5h://", "")
            .replace("http://", "")
            .replace("https://", "");
        if proxy.starts_with("socks5") {
            caps.add_arg(&format!("--proxy-server=socks5://{}", chrome_proxy))?;
        } else {
            caps.add_arg(&format!("--proxy-server=http://{}", chrome_proxy))?;
        }
    }

    let paths = [
        "C:\\Users\\xbl1e\\Downloads\\chrome-win64\\chrome.exe",
        "./chrome-win64/chrome.exe",
        "./chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        "./chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        "./chrome-linux64/chrome",
    ];

    for path in &paths {
        if std::fs::metadata(path).is_ok() {
            if std::path::Path::new(path).is_absolute() {
                caps.set_binary(path)?;
            } else {
                let abs = std::env::current_dir().unwrap_or_default().join(path);
                caps.set_binary(&abs.to_string_lossy())?;
            }
            break;
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "chromedriver.exe", "/T"])
            .creation_flags(0x08000008)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut c| c.wait());
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-f", "chromedriver"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .and_then(|mut c| c.wait());
    }

    let mut driver_cmd = if cfg!(windows) {
        std::process::Command::new("./chromedriver.exe")
    } else {
        std::process::Command::new("./chromedriver")
    };

    driver_cmd.arg("--port=9515")
              .arg("--silent")
              .stdout(std::process::Stdio::null())
              .stderr(std::process::Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        driver_cmd.creation_flags(0x08000008);
    }

    let driver_process = driver_cmd.spawn().ok();

    if driver_process.is_some() {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let driver = match WebDriver::new("http://localhost:9515", caps).await {
        Ok(d) => d,
        Err(e) => {
            if let Some(mut child) = driver_process {
                let _ = child.kill();
            }
            return Err(e.into());
        }
    };

    let _ = tx.send("Refreshing session tokens via reqwest...".into());

    let jar = Arc::new(reqwest::cookie::Jar::default());
    let domains = [
        "https://steamcommunity.com",
        "https://store.steampowered.com",
        "https://login.steampowered.com",
        "https://help.steampowered.com",
        "https://checkout.steampowered.com",
    ];

    for c in cookies {
        for domain_url in &domains {
            if let Ok(url) = domain_url.parse::<reqwest::Url>()
                && domain_url.contains(&c.domain)
            {
                jar.add_cookie_str(&format!("{}={}", c.name, c.value), &url);
            }
        }
    }

    let mut req_builder = reqwest::Client::builder().cookie_provider(Arc::clone(&jar));
    if let Some(proxy) = proxy_url
        && let Ok(p) = reqwest::Proxy::all(proxy)
    {
        req_builder = req_builder.proxy(p);
    }
    let req_client = req_builder
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap_or_default();

    let refresh_targets = [
        "https://login.steampowered.com/jwt/refresh?redir=https%3A%2F%2Fsteamcommunity.com%2Fmy%2Fprofile",
        "https://login.steampowered.com/jwt/refresh?redir=https%3A%2F%2Fstore.steampowered.com%2Faccount%2F",
    ];
    for target in &refresh_targets {
        let _ = req_client.get(*target).send().await;
    }

    let mut fresh_cookies: Vec<(String, String, String)> = Vec::new();
    for domain_url in &domains {
        if let Ok(url) = domain_url.parse::<reqwest::Url>()
            && let Some(header) = jar.cookies(&url)
            && let Ok(cookie_str) = header.to_str()
        {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some(eq) = part.find('=') {
                    let name = part[..eq].to_string();
                    let value = part[eq + 1..].to_string();
                    let domain = url.host_str().unwrap_or("").to_string();
                    fresh_cookies.push((domain, name, value));
                }
            }
        }
    }

    let _ = tx.send("Injecting fresh session into browser...".into());

    let target_domains = [
        "steamcommunity.com",
        "store.steampowered.com",
        "login.steampowered.com",
        "help.steampowered.com",
        "checkout.steampowered.com",
    ];

    let mut injected = 0usize;
    for domain in &target_domains {
        let domain_home = format!("https://{}", domain);
        if driver.goto(&domain_home).await.is_err() {
            continue;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let _ = driver.delete_all_cookies().await;

        for (cookie_domain, name, value) in &fresh_cookies {
            let cd_clean = cookie_domain.trim_start_matches('.');
            if domain.ends_with(cd_clean) || cd_clean.ends_with(*domain) {
                let mut c = thirtyfour::Cookie::new(name.clone(), value.clone());
                c.set_domain(cookie_domain.clone());
                c.set_path("/");
                c.set_secure(true);
                c.set_same_site(thirtyfour::common::cookie::SameSite::None);
                if driver.add_cookie(c).await.is_ok() {
                    injected += 1;
                }
            }
        }

        for c in cookies {
            let cd_clean = c.domain.trim_start_matches('.');
            if domain.ends_with(cd_clean) || cd_clean.ends_with(*domain) {
                let already_fresh = fresh_cookies.iter().any(|(d, n, _)| {
                    let d_clean = d.trim_start_matches('.');
                    (domain.ends_with(d_clean) || d_clean.ends_with(*domain)) && *n == c.name
                });
                if !already_fresh {
                    let mut wc = thirtyfour::Cookie::new(c.name.clone(), c.value.clone());
                    wc.set_domain(c.domain.clone());
                    wc.set_path("/");
                    wc.set_secure(c.secure);
                    wc.set_same_site(thirtyfour::common::cookie::SameSite::None);
                    if driver.add_cookie(wc).await.is_ok() {
                        injected += 1;
                    }
                }
            }
        }
    }

    let _ = tx.send(format!(
        "Injected {} cookies, setting up session...",
        injected
    ));

    driver.goto("https://login.steampowered.com/jwt/refresh?redir=https%3A%2F%2Fsteamcommunity.com%2Fmy%2Fprofile").await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let browser_cookies = driver.get_all_cookies().await?;
    let mut login_secure_val = String::new();
    let mut session_id_val = String::new();
    for cookie in &browser_cookies {
        if cookie.name == "steamLoginSecure" {
            login_secure_val = cookie.value.clone();
        }
        if cookie.name == "sessionid" {
            session_id_val = cookie.value.clone();
        }
    }

    if !login_secure_val.is_empty() {
        let token_parts: Vec<&str> = login_secure_val.split("%7C%7C").collect();
        if token_parts.len() == 2 {
            let access_token = token_parts[1];
            let ls_script = format!(
                "localStorage.setItem('WebUISessionInfo', JSON.stringify({{
                    version: 1,
                    sessionid: '{}',
                    steamLoginSecure: '{}',
                    token: '{}'
                }}));",
                session_id_val, login_secure_val, access_token
            );

            let _ = driver.execute(&ls_script, vec![]).await;

            if driver.goto("https://store.steampowered.com/").await.is_ok() {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let _ = driver.execute(&ls_script, vec![]).await;
            }
        }
    }

    driver.goto("https://steamcommunity.com/my/profile").await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let final_url = driver.current_url().await?.to_string();
    let logged_in = !final_url.contains("login");

    if logged_in {
        let _ = tx.send("Browser session active — logged in!".into());
    } else {
        let _ = tx.send("Cookies injected but Steam rejected session.".into());
    }

    let _ = tx.send("Browser is open. Close the TUI app to terminate.".into());

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
