use crate::models::cookie::{BulkResult, SteamCookie};
use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn parse_netscape_cookies_from_file(
    filename: &str,
) -> Result<Vec<SteamCookie>, std::io::Error> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    Ok(parse_netscape_lines(reader.lines().map_while(Result::ok)))
}

pub fn parse_netscape_lines<I>(lines: I) -> Vec<SteamCookie>
where
    I: Iterator<Item = String>,
{
    let mut cookies = Vec::new();
    let text = lines.collect::<Vec<String>>().join("\n");

    if parse_json_cookies(&text, &mut cookies) {
        return cookies;
    }

    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        parse_tsv_cookie_line(line, &mut cookies);
    }
    cookies
}

fn parse_json_cookies(text: &str, cookies: &mut Vec<SteamCookie>) -> bool {
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(arr) = json_val.as_array()
    {
        for item in arr {
            if let (Some(domain), Some(name), Some(value)) = (
                item.get("domain").and_then(|v| v.as_str()),
                item.get("name").and_then(|v| v.as_str()),
                item.get("value").and_then(|v| v.as_str()),
            ) {
                let secure = item
                    .get("secure")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                cookies.push(SteamCookie {
                    domain: domain.trim().to_string(),
                    secure,
                    name: name.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
        }
        return !cookies.is_empty();
    }
    false
}

fn parse_tsv_cookie_line(line: &str, cookies: &mut Vec<SteamCookie>) {
    let parts: Vec<&str> = line.split('\t').collect();
    let fallback_parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 7 && fallback_parts.len() >= 7 {
        add_cookie_from_parts(&fallback_parts, true, cookies);
        return;
    }

    if parts.len() >= 7 {
        add_cookie_from_parts(&parts, false, cookies);
    }
}

fn add_cookie_from_parts(parts: &[&str], is_fallback: bool, cookies: &mut Vec<SteamCookie>) {
    let domain = parts[0].trim().to_string();
    let flag = parts[3].trim().to_uppercase();
    if domain.contains('.') && (flag == "TRUE" || flag == "FALSE") {
        let value = if is_fallback {
            parts[6..].join(" ")
        } else {
            parts[6].trim().to_string()
        };
        cookies.push(SteamCookie {
            domain,
            secure: flag == "TRUE",
            name: parts[5].trim().to_string(),
            value,
        });
    }
}

pub fn process_bulk_input(path: &str) -> Result<BulkResult, Box<dyn std::error::Error>> {
    let path_obj = std::path::Path::new(path);
    let mut results = Vec::new();

    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path).into());
    }

    if path_obj
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        process_zip_file(path, &mut results)?;
    } else if path_obj.is_dir() {
        process_directory(path, &mut results);
    } else if path_obj.is_file()
        && path_obj
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
    {
        process_txt_file(path, &mut results);
    } else {
        return Err("Path is neither a directory, zip file, nor a .txt file.".into());
    }

    Ok(results)
}

fn process_zip_file(
    path: &str,
    results: &mut BulkResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            let name = file.name().to_string();
            process_zip_entry(&mut file, name, results);
        }
    }
    Ok(())
}

fn process_zip_entry<R: std::io::Read>(mut reader: R, name: String, results: &mut BulkResult) {
    let mut contents = String::new();
    if reader.read_to_string(&mut contents).is_ok() {
        let lines = contents.lines().map(|s| s.to_string());
        let parsed = parse_netscape_lines(lines);
        if !parsed.is_empty() {
            let grouped = crate::parser::input::group_cookies_by_account(parsed);
            for (id, cookie_group) in grouped {
                let acc_name = format!("{} ({})", name, id);
                results.push((acc_name, cookie_group));
            }
        }
    }
}

fn process_directory(path: &str, results: &mut BulkResult) {
    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let file_path = entry.path().to_string_lossy().to_string();
            if let Ok(parsed) = parse_netscape_cookies_from_file(&file_path)
                && !parsed.is_empty()
            {
                let grouped = crate::parser::input::group_cookies_by_account(parsed);
                for (id, cookie_group) in grouped {
                    let acc_name = format!("{} ({})", file_path, id);
                    results.push((acc_name, cookie_group));
                }
            }
        }
    }
}

fn process_txt_file(path: &str, results: &mut BulkResult) {
    if let Ok(parsed) = parse_netscape_cookies_from_file(path)
        && !parsed.is_empty()
    {
        let grouped = crate::parser::input::group_cookies_by_account(parsed);
        for (id, cookie_group) in grouped {
            let acc_name = format!("{} ({})", path, id);
            results.push((acc_name, cookie_group));
        }
    }
}

pub fn group_cookies_by_account(
    cookies: Vec<SteamCookie>,
) -> std::collections::HashMap<String, Vec<SteamCookie>> {
    let mut map = std::collections::HashMap::new();

    let all_ids = collect_steam_ids(&cookies);

    if all_ids.len() <= 1 {
        let id = all_ids
            .into_iter()
            .next()
            .unwrap_or_else(|| "unknown".to_string());
        map.insert(id, cookies);
        return map;
    }

    group_multiple_accounts(cookies, map)
}

fn extract_numeric_steam_id(cookie: &SteamCookie, valid_names: &[&str]) -> Option<String> {
    if valid_names.contains(&cookie.name.as_str())
        && let Some(idx) = cookie
            .value
            .find("%7C%7C")
            .or_else(|| cookie.value.find("||"))
    {
        let id = cookie.value[..idx].to_string();
        if id.chars().all(char::is_numeric) {
            return Some(id);
        }
    }
    None
}

fn collect_steam_ids(cookies: &[SteamCookie]) -> std::collections::HashSet<String> {
    let mut all_ids = std::collections::HashSet::new();
    for c in cookies {
        if let Some(id) = extract_numeric_steam_id(c, &["steamLoginSecure", "steamMachineAuth"]) {
            all_ids.insert(id);
        }
    }
    all_ids
}

fn group_multiple_accounts(
    cookies: Vec<SteamCookie>,
    mut map: std::collections::HashMap<String, Vec<SteamCookie>>,
) -> std::collections::HashMap<String, Vec<SteamCookie>> {
    let mut current_group = Vec::new();
    let mut active_id = "unknown".to_string();

    for c in cookies {
        if let Some(id) = extract_numeric_steam_id(&c, &["steamLoginSecure"]) {
            if active_id != "unknown" && active_id != id && !current_group.is_empty() {
                map.entry(active_id.clone())
                    .or_default()
                    .extend(current_group);
                current_group = Vec::new();
            }
            active_id = id;
        }
        current_group.push(c);
    }

    if !current_group.is_empty() && active_id != "unknown" {
        map.entry(active_id).or_default().extend(current_group);
    }

    map
}
