mod engine;
mod polymarket_fetcher;
mod kalshi_fetcher;
mod manifold_fetcher;
mod telegram_notifier;
mod cross_matcher;
mod config;

use engine::ArbitrageEngine;
use polymarket_fetcher::PolymarketFetcher;
use kalshi_fetcher::KalshiFetcher;
use manifold_fetcher::ManifoldFetcher;
use telegram_notifier::TelegramNotifier;
use cross_matcher::CrossMatcher;
use config::Config;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use std::env;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    // Load config from .env
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID not set");
    let total_capital = env::var("TOTAL_CAPITAL")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<f64>()
        .unwrap_or(1000.0);

    let poly_fetcher = PolymarketFetcher::new();
    let kalshi_fetcher = KalshiFetcher::new();
    let manifold_fetcher = ManifoldFetcher::new();
    let notifier = TelegramNotifier::new(bot_token, chat_id);
    let cross_matcher = CrossMatcher::new();

    // Dedup: track already-alerted opportunity IDs (clear after 1 hour)
    let mut sent_ids: HashSet<String> = HashSet::new();
    let mut last_clear = Instant::now();
    
    println!("🚀 Rust HFT Arbitrage Engine Started!");
    println!("📡 Scanning Polymarket, Kalshi & Manifold");
    println!("💰 Capital: ${:.2}", total_capital);
    println!("🔍 Strategies: Single-Platform + Cross-Platform + Heuristic Matching\n");

    // Send startup notification
    if let Err(e) = notifier.send_startup_message().await {
        eprintln!("Failed to send startup message: {}", e);
    }

    loop {
        // Clear dedup cache every hour
        if last_clear.elapsed() > Duration::from_secs(3600) {
            sent_ids.clear();
            last_clear = Instant::now();
            println!("🔄 Cleared dedup cache");
        }

        // Reload config each cycle to pick up changes from Telegram bot
        let config = Config::load();
        
        // Check if notifications are enabled
        if !config.notifications_enabled {
            println!("⏸️ Notifications paused. Waiting...");
            sleep(Duration::from_secs(config.scan_interval_seconds)).await;
            continue;
        }

        // Create engine with config settings
        let engine = ArbitrageEngine::new(
            config.min_roi_percent / 100.0,
            config.min_profit_threshold,
            total_capital
        );

        let start = Instant::now();

        // 1. Fetch markets from ALL platforms in parallel
        let (poly_result, kalshi_result, manifold_result) = tokio::join!(
            poly_fetcher.fetch_all_markets(),
            kalshi_fetcher.fetch_all_markets(),
            manifold_fetcher.fetch_all_markets()
        );

        let mut all_markets = Vec::new();
        
        // Collect results
        let poly_markets = match poly_result {
            Ok(m) => { println!("✓ Polymarket: {} markets", m.len()); m }
            Err(e) => { eprintln!("❌ Polymarket: {}", e); Vec::new() }
        };
        let kalshi_markets = match kalshi_result {
            Ok(m) => { println!("✓ Kalshi: {} markets", m.len()); m }
            Err(e) => { eprintln!("❌ Kalshi: {}", e); Vec::new() }
        };
        let manifold_markets = match manifold_result {
            Ok(m) => { println!("✓ Manifold: {} markets", m.len()); m }
            Err(e) => { eprintln!("❌ Manifold: {}", e); Vec::new() }
        };

        all_markets.extend(poly_markets.iter().cloned());
        all_markets.extend(kalshi_markets.iter().cloned());
        all_markets.extend(manifold_markets.iter().cloned());

        let fetch_duration = start.elapsed();
        println!("⚡ Fetch: {:.1}s ({} markets)", fetch_duration.as_secs_f64(), all_markets.len());

        // 2. Single-platform arbitrage analysis
        let analysis_start = Instant::now();
        let opportunities = engine.analyze_markets(&all_markets);
        println!("🔍 Analysis: {}ms, {} opps", analysis_start.elapsed().as_millis(), opportunities.len());


        let mut new_opps = 0;
        for opp in &opportunities {
            if sent_ids.contains(&opp.id) {
                continue; // Already alerted
            }
            sent_ids.insert(opp.id.clone());
            new_opps += 1;

            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("🎯 {} | ROI: {:.2}% | ${:.4}", opp.opp_type, opp.roi_percent, opp.net_profit_after_fees);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            if let Err(e) = notifier.send_opportunity(opp).await {
                eprintln!("Failed to send alert: {}", e);
            }
        }

        // 3. Cross-platform heuristic matching
        let cross_start = Instant::now();
        let mut platform_markets: HashMap<String, Vec<&engine::Market>> = HashMap::new();
        for m in &poly_markets {
            platform_markets.entry("Polymarket".to_string()).or_default().push(m);
        }
        for m in &kalshi_markets {
            platform_markets.entry("Kalshi".to_string()).or_default().push(m);
        }
        for m in &manifold_markets {
            platform_markets.entry("Manifold".to_string()).or_default().push(m);
        }

        let cross_matches = cross_matcher.match_all(&platform_markets);
        println!("🔗 Cross-match: {}ms, {} matches", cross_start.elapsed().as_millis(), cross_matches.len());

        let mut new_cross = 0;
        for cm in &cross_matches {
            let cm_id = format!("cross_{}_{}", cm.id_a, cm.id_b);
            if sent_ids.contains(&cm_id) {
                continue; // Already alerted
            }
            sent_ids.insert(cm_id);
            new_cross += 1;

            println!("🔗 [{}] {} ↔ {} | diff: {:.1}% | conf: {:.0}%",
                cm.category, cm.platform_a, cm.platform_b, 
                cm.price_diff * 100.0, cm.confidence * 100.0);

            if let Err(e) = notifier.send_cross_match(cm).await {
                eprintln!("Failed to send cross-match: {}", e);
            }
        }

        // Summary
        let scan_time = start.elapsed().as_millis() as u64;
        println!("📊 New alerts: {} opps + {} cross (dedup cache: {})",
            new_opps, new_cross, sent_ids.len());

        if new_opps + new_cross > 0 {
            if let Err(e) = notifier.send_summary(all_markets.len(), new_opps + new_cross, scan_time).await {
                eprintln!("Failed to send summary: {}", e);
            }
        }

        println!("⏳ Next scan in {}s...\n", config.scan_interval_seconds);
        sleep(Duration::from_secs(config.scan_interval_seconds)).await;
    }
}
