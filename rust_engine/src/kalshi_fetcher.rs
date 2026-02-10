use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;
use crate::config::Config;
use chrono::Utc;

#[derive(Debug, Deserialize)]
struct KalshiResponse {
    markets: Vec<KalshiMarket>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct KalshiMarket {
    #[serde(default)]
    ticker: String,
    #[serde(default)]
    event_ticker: Option<String>,
    #[serde(default)]
    title: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    yes_bid: Option<f64>,
    #[serde(default)]
    yes_ask: Option<f64>,
    #[serde(default)]
    no_bid: Option<f64>,
    #[serde(default)]
    no_ask: Option<f64>,
    #[serde(default)]
    volume: Option<f64>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    close_time: Option<String>,
    #[serde(default)]
    expiration_time: Option<String>,
}

pub struct KalshiFetcher {
    client: Client,
    base_url: String,
}

impl KalshiFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self {
            client,
            base_url: "https://api.elections.kalshi.com/trade-api/v2".to_string(),
        }
    }

    pub async fn fetch_all_markets(&self) -> Result<Vec<crate::engine::Market>, Box<dyn Error>> {
        // Load config for dynamic settings
        let config = Config::load();
        let max_pages = config.max_pages_kalshi;
        let category_keywords = config.category_keywords();
        let filter_enabled = !config.enabled_categories.is_empty();

        println!("[Kalshi] Starting fetch (max {} pages)...", max_pages);
        let mut all_markets = Vec::new();
        let mut cursor: Option<String> = None;
        let mut page_count = 0;
        
        loop {
            if page_count >= max_pages {
                println!("[Kalshi] Reached max pages limit ({})", max_pages);
                break;
            }
            let url = if let Some(ref c) = cursor {
                format!("{}/markets?limit=200&status=open&cursor={}", self.base_url, c)
            } else {
                format!("{}/markets?limit=200&status=open", self.base_url)
            };

            println!("[Kalshi] Requesting: {}", url);

            let response: KalshiResponse = self.client
                .get(&url)
                .send()
                .await?
                .json()
                .await?;

            println!("[Kalshi] Received {} markets", response.markets.len());

            for market in response.markets {
                // Filter expired
                let close_date = market.close_time.or(market.expiration_time);
                if let Some(ref cd) = close_date {
                    if let Ok(dt) = cd.parse::<chrono::DateTime<Utc>>() {
                        if dt < Utc::now() {
                            continue;
                        }
                    }
                }

                // Apply category filter if enabled
                if filter_enabled {
                    let title_lower = market.title.to_lowercase();
                    let subtitle_lower = market.subtitle.as_ref()
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();
                    let full_text = format!("{} {}", title_lower, subtitle_lower);
                    
                    let matches_category = category_keywords.iter()
                        .any(|kw| full_text.contains(kw));
                    
                    if !matches_category {
                        continue;
                    }
                }

                let yes_price = market.yes_ask.or(market.yes_bid).unwrap_or(0.0) / 100.0;
                let no_price = market.no_ask.or(market.no_bid).unwrap_or(0.0) / 100.0;

                let liquidity = market.volume.unwrap_or(0.0);

                all_markets.push(crate::engine::Market {
                    id: market.ticker.clone(),
                    question: None,
                    title: Some(market.title),
                    subtitle: market.subtitle,
                    outcome_prices: vec![yes_price, no_price],
                    platform: "Kalshi".to_string(),
                    liquidity,
                    close_date,
                    url: Some(format!("https://kalshi.com/markets/{}", 
                        market.event_ticker.as_deref().unwrap_or(&market.ticker))),
                });
            }

            cursor = response.cursor;
            page_count += 1;
            if cursor.is_none() {
                break;
            }
        }

        println!("[Kalshi] Total: {} markets fetched (filtered)", all_markets.len());
        Ok(all_markets)
    }
}
