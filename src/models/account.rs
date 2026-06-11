use serde::{Deserialize, Serialize};

pub fn default_valid() -> bool {
    true
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct AccountData {
    #[serde(default = "default_valid")]
    pub is_valid: bool,
    pub steam_id: String,
    pub username: String,
    pub custom_url: String,
    pub profile_url: String,
    pub level: String,
    pub member_since: String,
    pub email: String,
    pub wallet_balance: String,
    pub hold_balance: String,
    pub inventory_balance: String,
    pub guard: String,
    pub phone: String,
    pub country: String,
    pub steam_points: String,
    pub games_count: String,
    pub friends_count: String,
    pub inventory_cs2: String,
    pub inventory_dota: String,
    pub inventory_tf2: String,
    pub inventory_pubg: String,
    pub inventory_rust: String,
    pub inventory_steam: String,
    pub market_active_listings: String,
    pub sih_status: String,
    pub owned_apps: String,
    pub wishlist_count: String,
    pub vac: String,
    pub trade_ban: String,
    pub community_ban: String,
    pub limited: String,
    pub market: String,
    pub cs_prime: String,
    pub family_view: String,
    pub hours_played: String,
    pub badges: String,
    pub name_history: Vec<(String, String)>,
    pub token_ip: String,
    pub token_exp: String,
    pub token_aud: String,
    pub token_issued: String,
    pub refresh_exp: String,
    pub token_country: String,
    pub cookies: Vec<crate::models::cookie::SteamCookie>,
}

impl AccountData {
    pub fn new(steam_id: String) -> Self {
        Self {
            steam_id,
            is_valid: true,
            ..Default::default()
        }
    }

    pub fn display(&self, field: &str) -> &str {
        let val = match field {
            "username" => &self.username,
            "custom_url" => &self.custom_url,
            "profile_url" => &self.profile_url,
            "level" => &self.level,
            "member_since" => &self.member_since,
            "email" => &self.email,
            "wallet_balance" => &self.wallet_balance,
            "hold_balance" => &self.hold_balance,
            "inventory_balance" => &self.inventory_balance,
            "guard" => &self.guard,
            "phone" => &self.phone,
            "country" => &self.country,
            "steam_points" => &self.steam_points,
            "games_count" => &self.games_count,
            "friends_count" => &self.friends_count,
            "inventory_cs2" => &self.inventory_cs2,
            "inventory_dota" => &self.inventory_dota,
            "inventory_tf2" => &self.inventory_tf2,
            "inventory_pubg" => &self.inventory_pubg,
            "inventory_rust" => &self.inventory_rust,
            "inventory_steam" => &self.inventory_steam,
            "market_active_listings" => &self.market_active_listings,
            "sih_status" => &self.sih_status,
            "owned_apps" => &self.owned_apps,
            "wishlist_count" => &self.wishlist_count,
            "vac" => &self.vac,
            "trade_ban" => &self.trade_ban,
            "community_ban" => &self.community_ban,
            "limited" => &self.limited,
            "market" => &self.market,
            "cs_prime" => &self.cs_prime,
            "family_view" => &self.family_view,
            "hours_played" => &self.hours_played,
            "badges" => &self.badges,
            _ => "",
        };
        if val.is_empty() { "—" } else { val }
    }
}
