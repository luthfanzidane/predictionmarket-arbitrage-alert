use serde::Deserialize;
use reqwest::Client;
use std::error::Error;
use chrono::{Utc, TimeZone};

#[derive(Debug, Deserialize, Default)]
struct ManifoldMarket {
    #[serde(default)]
    id: String,
    #[serde(default)]
    question: String,
    #[serde(default)]
    probability: Option<f64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    volume: Option<f64>,
    #[serde(rename = "isResolved", default)]
    is_resolved: bool,
    #[serde(rename = "closeTime", default)]
    close_time: Option<i64>,
}

pub struct ManifoldFetcher {
    client: Client,
}

impl ManifoldFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self { client }
    }

    pub async fn fetch_all_markets(&self) -> Result<Vec<crate::engine::Market>, Box<dyn Error>> {
        println!("[Manifold] Starting fetch...");
        let mut all_markets = Vec::new();
        let now_ms = Utc::now().timestamp_millis();

        let url = "https://api.manifold.markets/v0/search-markets?filter=open&contractType=BINARY&limit=500&sort=liquidity";
        
        let response = self.client
            .get(url)
            .send()
            .await?;

        if response.status() != 200 {
            println!("[Manifold] API returned {}", response.status());
            return Ok(all_markets);
        }

        let markets: Vec<ManifoldMarket> = response.json().await?;
        println!("[Manifold] Received {} markets", markets.len());

        for m in markets {
            // Skip resolved
            if m.is_resolved {
                continue;
            }
            
            // Skip expired
            if let Some(ct) = m.close_time {
                if ct < now_ms {
                    continue;
                }
            }

            let prob = m.probability.unwrap_or(0.0);
            let close_date = m.close_time.map(|ct| {
                Utc.timestamp_millis_opt(ct)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            });

            all_markets.push(crate::engine::Market {
                id: m.id,
                question: Some(m.question),
                title: None,
                subtitle: None,
                outcome_prices: vec![prob, 1.0 - prob],
                platform: "Manifold".to_string(),
                liquidity: m.volume.unwrap_or(0.0),
                close_date,
                url: m.url,
            });
        }

        println!("[Manifold] Total: {} active markets", all_markets.len());
        Ok(all_markets)
    }
}
