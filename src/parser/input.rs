use crate::models::cookie::{BulkResult, SteamCookie};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn get_filepath_from_input() -> Result<String, std::io::Error> {
    print!("\x1b[35m⟡\x1b[0m Drop cookies file here: ");
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    let mut trimmed = buffer.trim();
    if trimmed.starts_with("& ") {
        trimmed = &trimmed[2..];
        trimmed = trimmed.trim();
    }

    let final_path = trimmed.trim_matches('"').trim_matches('\'').to_string();
    Ok(final_path)
}

pub fn parse_netscape_cookies_from_file(
    filename: &str,
) -> Result<Vec<SteamCookie>, std::io::Error> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    Ok(parse_netscape_lines(reader.lines().map_while(Result::ok)))
}

pub fn get_cookies_from_paste() -> Result<Vec<SteamCookie>, Box<dyn std::error::Error>> {
    println!("  \x1b[35m── Paste Cookie Text ──────────\x1b[0m");
    println!("  \x1b[2m  Paste your raw log text below.\x1b[0m");
    println!("  \x1b[2m  Press ENTER twice on a new line when finished.\x1b[0m");
    println!("  \x1b[35m───────────────────────────────────────\x1b[0m");
    io::stdout().flush().ok();

    let mut lines = Vec::new();
    let mut consecutive_empty = 0;
    loop {
        let mut buffer = String::new();
        if io::stdin().read_line(&mut buffer).is_err() {
            break;
        }

        if buffer.trim().is_empty() {
            consecutive_empty += 1;
            if consecutive_empty >= 2 {
                break;
            }
        } else {
            consecutive_empty = 0;
        }
        lines.push(buffer);
    }

    let cookies = parse_netscape_lines(lines.into_iter());
    Ok(cookies)
}

pub fn parse_netscape_lines<I>(lines: I) -> Vec<SteamCookie>
where
    I: Iterator<Item = String>,
{
    let mut cookies = Vec::new();
    let text = lines.collect::<Vec<String>>().join("\n");

    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text)
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
        if !cookies.is_empty() {
            return cookies;
        }
    }

    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();

        let fallback_parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 && fallback_parts.len() >= 7 {
            let value = fallback_parts[6..].join(" ");
            let domain = fallback_parts[0].trim().to_string();
            let flag = fallback_parts[3].trim().to_uppercase();
            if domain.contains('.') && (flag == "TRUE" || flag == "FALSE") {
                cookies.push(SteamCookie {
                    domain,
                    secure: flag == "TRUE",
                    name: fallback_parts[5].trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }

        if parts.len() >= 7 {
            let domain = parts[0].trim().to_string();
            let flag = parts[3].trim().to_uppercase();
            if domain.contains('.') && (flag == "TRUE" || flag == "FALSE") {
                cookies.push(SteamCookie {
                    domain,
                    secure: flag == "TRUE",
                    name: parts[5].trim().to_string(),
                    value: parts[6].trim().to_string(),
                });
            }
        }
    }
    cookies
}

pub fn get_bulk_path_from_input() -> Result<String, std::io::Error> {
    print!("\x1b[35m⟡\x1b[0m Drop folder or .zip file here: ");
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    let mut trimmed = buffer.trim();
    if trimmed.starts_with("& ") {
        trimmed = &trimmed[2..];
        trimmed = trimmed.trim();
    }

    Ok(trimmed.trim_matches('"').trim_matches('\'').to_string())
}

pub fn process_bulk_input(path: &str) -> Result<BulkResult, Box<dyn std::error::Error>> {
    let path_obj = std::path::Path::new(path);
    let mut results = Vec::new();

    if !path_obj.exists() {
        return Err(format!("Path does not exist: {}", path).into());
    }

    let is_zip = path_obj
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));

    if is_zip {
        println!("  \x1b[2m↳ Extracting ZIP archive...\x1b[0m");
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.is_file() {
                let name = file.name().to_string();
                let mut contents = String::new();
                use std::io::Read;
                if file.read_to_string(&mut contents).is_ok() {
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
        }
    } else if path_obj.is_dir() {
        println!("  \x1b[2m↳ Scanning folder...\x1b[0m");
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
    } else if path_obj.is_file()
        && path_obj
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
    {
        println!("  \x1b[2m↳ Scanning text file...\x1b[0m");
        if let Ok(parsed) = parse_netscape_cookies_from_file(path)
            && !parsed.is_empty()
        {
            let grouped = crate::parser::input::group_cookies_by_account(parsed);
            for (id, cookie_group) in grouped {
                let acc_name = format!("{} ({})", path, id);
                results.push((acc_name, cookie_group));
            }
        }
    } else {
        return Err("Path is neither a directory, zip file, nor a .txt file.".into());
    }

    println!(
        "  \x1b[32m✔ Found {} valid cookie files.\x1b[0m\n",
        results.len()
    );
    Ok(results)
}

pub fn group_cookies_by_account(
    cookies: Vec<SteamCookie>,
) -> std::collections::HashMap<String, Vec<SteamCookie>> {
    let mut map = std::collections::HashMap::new();

    let mut all_ids = std::collections::HashSet::new();
    for c in &cookies {
        if (c.name == "steamLoginSecure" || c.name == "steamMachineAuth")
            && let Some(idx) = c.value.find("%7C%7C").or_else(|| c.value.find("||"))
        {
            let id = c.value[..idx].to_string();
            if id.chars().all(char::is_numeric) {
                all_ids.insert(id);
            }
        }
    }

    if all_ids.len() <= 1 {
        let id = all_ids
            .into_iter()
            .next()
            .unwrap_or_else(|| "unknown".to_string());
        map.insert(id, cookies);
        return map;
    }

    let mut current_group = Vec::new();
    let mut active_id = "unknown".to_string();

    for c in cookies {
        if c.name == "steamLoginSecure"
            && let Some(idx) = c.value.find("%7C%7C").or_else(|| c.value.find("||"))
        {
            let id = c.value[..idx].to_string();
            if id.chars().all(char::is_numeric) {
                if active_id != "unknown" && active_id != id && !current_group.is_empty() {
                    map.entry(active_id.clone())
                        .or_insert_with(Vec::new)
                        .extend(current_group);
                    current_group = Vec::new();
                }
                active_id = id;
            }
        }
        current_group.push(c);
    }

    if !current_group.is_empty() && active_id != "unknown" {
        map.entry(active_id)
            .or_insert_with(Vec::new)
            .extend(current_group);
    }

    map
}
