use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub fn regex_extract(text: &str, pattern: &str) -> String {
    static REGEX_CACHE: LazyLock<Mutex<HashMap<String, Regex>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut cache = REGEX_CACHE.lock().unwrap();
    let re = cache
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).unwrap());

    re.captures(text)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default()
}
