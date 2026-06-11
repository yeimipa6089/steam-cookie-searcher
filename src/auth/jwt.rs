use crate::models::account::AccountData;
use crate::models::cookie::SteamCookie;
use crate::utils::ip::get_ip_country;
use crate::utils::time::format_timestamp;

pub fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload.to_string(),
    };
    let standard = padded.replace('-', "+").replace('_', "/");
    let bytes = base64_decode(&standard)?;
    serde_json::from_slice(&bytes).ok()
}

pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf: Vec<u8> = Vec::new();
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = TABLE.iter().position(|&x| x == b)? as u32;
        acc = (acc << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            buf.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(buf)
}

pub fn extract_jwt_from_cookie(cookie_value: &str) -> Option<serde_json::Value> {
    let decoded = urlencoding::decode(cookie_value).ok()?;
    let token = decoded
        .split("||")
        .nth(1)
        .or_else(|| decoded.split("%7C%7C").nth(1))?;
    decode_jwt_payload(token)
}

pub async fn extract_token_info(cookies: &[SteamCookie], data: &mut AccountData) {
    if let Some(c) = cookies.iter().find(|c| c.name == "steamLoginSecure")
        && let Some(jwt) = extract_jwt_from_cookie(&c.value)
    {
        if let Some(ip) = jwt.get("ip_subject").and_then(|v| v.as_str()) {
            data.token_ip = ip.to_string();
            data.token_country = get_ip_country(ip).await;
        }
        if let Some(exp) = jwt.get("exp").and_then(|v| v.as_u64()) {
            data.token_exp = format_timestamp(exp);
        }
        if let Some(iat) = jwt.get("iat").and_then(|v| v.as_u64()) {
            data.token_issued = format_timestamp(iat);
        }
        if let Some(aud) = jwt.get("aud").and_then(|v| v.as_array()) {
            let audiences: Vec<String> = aud
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            data.token_aud = audiences.join(", ");
        }
    }

    if let Some(c) = cookies.iter().find(|c| c.name == "steamRefresh_steam")
        && let Some(jwt) = extract_jwt_from_cookie(&c.value)
    {
        if let Some(exp) = jwt.get("exp").and_then(|v| v.as_u64()) {
            data.refresh_exp = format_timestamp(exp);
        }
        if data.token_ip.is_empty()
            && let Some(ip) = jwt.get("ip_subject").and_then(|v| v.as_str())
        {
            data.token_ip = ip.to_string();
            data.token_country = get_ip_country(ip).await;
        }
    }
}
