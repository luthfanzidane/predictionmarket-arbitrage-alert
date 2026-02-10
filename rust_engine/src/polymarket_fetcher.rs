use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;
use crate::config::Config;
use chrono::Utc;

#[derive(Debug, Deserialize, Default)]
struct PolymarketMarket {
    #[serde(default)]
    id: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(rename = "outcomePrices", default)]
    outcome_prices: Option<String>,
    #[serde(default)]
    liquidity: Option<String>,
    #[serde(default)]
    closed: bool,
    #[serde(default)]
    resolved: Option<bool>,
    #[serde(rename = "endDateIso", default)]
    end_date_iso: Option<String>,
    #[serde(rename = "endDate", default)]
    end_date: Option<String>,
    #[serde(default)]
    events: Vec<PolymarketEvent>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct PolymarketEvent {
    #[serde(default)]
    slug: Option<String>,
}

pub struct PolymarketFetcher {
    client: Client,
    base_url: String,
}

impl PolymarketFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self {
            client,
            base_url: "https://gamma-api.polymarket.com".to_string(),
        }
    }

    pub async fn fetch_all_markets(&self) -> Result<Vec<crate::engine::Market>, Box<dyn Error>> {
        // Load config for dynamic settings
        let config = Config::load();
        let max_pages = config.max_pages_polymarket;
        let category_keywords = config.category_keywords();
        let filter_enabled = !config.enabled_categories.is_empty();

        println!("[Polymarket] Starting fetch (max {} pages)...", max_pages);
        let mut all_markets = Vec::new();
        let mut offset = 0;
        let mut page_count = 0;
        const LIMIT: i32 = 100;
        
        loop {
            if page_count >= max_pages {
                println!("[Polymarket] Reached max pages limit ({})", max_pages);
                break;
            }
            
            let url = format!("{}/markets?limit={}&offset={}&closed=false", self.base_url, LIMIT, offset);

            println!("[Polymarket] Page {} - Requesting...", page_count + 1);
            
            let markets: Vec<PolymarketMarket> = self.client
                .get(&url)
                .send()
                .await?
                .json()
                .await?;

            println!("[Polymarket] Received {} markets", markets.len());

            if markets.is_empty() {
                break;
            }

            for market in markets {
                if market.closed {
                    continue;
                }
                if market.resolved.unwrap_or(false) {
                    continue;
                }

                // Parse and check close date
                let close_date = market.end_date_iso.or(market.end_date);
                if let Some(ref cd) = close_date {
                    if let Ok(dt) = cd.parse::<chrono::DateTime<Utc>>() {
                        if dt < Utc::now() {
                            continue; // Expired
                        }
                    }
                }

                // Apply category filter if enabled
                if filter_enabled {
                    let question_lower = market.question.to_lowercase();
                    let matches_category = category_keywords.iter()
                        .any(|kw| question_lower.contains(kw));
                    
                    if !matches_category {
                        continue;
                    }
                }

                let prices: Vec<f64> = if let Some(ref prices_str) = market.outcome_prices {
                    serde_json::from_str(prices_str).unwrap_or_default()
                } else {
                    Vec::new()
                };

                let liquidity = market.liquidity
                    .and_then(|l| l.parse::<f64>().ok())
                    .unwrap_or(0.0);

                // Build correct Polymarket URL
                let url = {
                    let event_slug = market.events.first()
                        .and_then(|e| e.slug.as_deref());
                    let market_slug = market.slug.as_deref();

                    match (event_slug, market_slug) {
                        (Some(es), Some(ms)) => Some(format!("https://polymarket.com/event/{}/{}", es, ms)),
                        (Some(es), None) => Some(format!("https://polymarket.com/event/{}", es)),
                        _ => None,
                    }
                };

                all_markets.push(crate::engine::Market {
                    id: market.id.clone(),
                    question: Some(market.question),
                    title: None,
                    subtitle: None,
                    outcome_prices: prices,
                    platform: "Polymarket".to_string(),
                    liquidity,
                    close_date,
                    url,
                });
            }

            offset += LIMIT;
            page_count += 1;
        }

        println!("[Polymarket] Total: {} markets fetched (filtered)", all_markets.len());
        Ok(all_markets)
    }
}
