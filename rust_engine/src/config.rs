use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_categories")]
    pub enabled_categories: Vec<String>,
    #[serde(default = "default_poly_pages")]
    pub max_pages_polymarket: i32,
    #[serde(default = "default_kalshi_pages")]
    pub max_pages_kalshi: i32,
    #[serde(default = "default_roi")]
    pub min_roi_percent: f64,
    #[serde(default = "default_profit")]
    pub min_profit_threshold: f64,
    #[serde(default = "default_interval")]
    pub scan_interval_seconds: u64,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
}

fn default_categories() -> Vec<String> {
    vec!["politics".to_string(), "sports".to_string(), "crypto".to_string(), "economics".to_string()]
}

fn default_poly_pages() -> i32 { 10 }
fn default_kalshi_pages() -> i32 { 5 }
fn default_roi() -> f64 { 1.0 }
fn default_profit() -> f64 { 0.05 }
fn default_interval() -> u64 { 5 }
fn default_true() -> bool { true }

impl Config {
    pub fn load() -> Self {
        let config_path = "../config.json";
        
        if let Ok(content) = fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
        
        // Return default config
        Config {
            enabled_categories: default_categories(),
            max_pages_polymarket: default_poly_pages(),
            max_pages_kalshi: default_kalshi_pages(),
            min_roi_percent: default_roi(),
            min_profit_threshold: default_profit(),
            scan_interval_seconds: default_interval(),
            notifications_enabled: default_true(),
        }
    }
    
    pub fn category_keywords(&self) -> Vec<String> {
        let mut keywords = Vec::new();
        
        let category_map: std::collections::HashMap<&str, Vec<&str>> = [
            ("politics", vec!["election", "president", "congress", "senate", "governor", "trump", "biden", "harris", "republican", "democrat"]),
            ("sports", vec!["nba", "nfl", "mlb", "nhl", "soccer", "football", "basketball", "baseball", "game", "championship"]),
            ("crypto", vec!["bitcoin", "ethereum", "btc", "eth", "crypto", "blockchain", "defi", "nft"]),
            ("economics", vec!["fed", "interest rate", "inflation", "gdp", "recession", "stock", "market", "economy"]),
            ("entertainment", vec!["oscar", "grammy", "movie", "tv", "celebrity", "award"]),
            ("tech", vec!["ai", "apple", "google", "microsoft", "tesla", "spacex", "technology"]),
            ("world", vec!["war", "ukraine", "russia", "china", "nato", "un", "world"]),
        ].iter().cloned().collect();
        
        for cat in &self.enabled_categories {
            if let Some(kws) = category_map.get(cat.as_str()) {
                for kw in kws {
                    keywords.push(kw.to_string());
                }
            }
        }
        
        keywords
    }
}
