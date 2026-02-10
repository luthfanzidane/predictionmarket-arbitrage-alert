use serde::{Deserialize, Serialize};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

// Platform fee constants (percentage)
const POLYMARKET_FEE: f64 = 0.02; // 2%
const KALSHI_FEE: f64 = 0.01;     // 1%
const MANIFOLD_FEE: f64 = 0.02;   // 2%

// Minimum profit threshold from Roan's research ($0.05)
const MIN_PROFIT_THRESHOLD: f64 = 0.05;

// Implication patterns for dependency detection
// Format: (keyword_a, keyword_b) means if market contains A, it implies market B
const IMPLICATION_PATTERNS: &[(&str, &str)] = &[
    // === POLITICS ===
    ("trump win", "republican win"),
    ("trump wins", "republicans win"),
    ("biden win", "democrat win"),
    ("harris win", "democrat win"),
    ("landslide", "win"),
    ("win by 5+", "win"),
    ("win by 10+", "win by 5+"),
    // === CRYPTO ===
    ("bitcoin 200k", "bitcoin 150k"),
    ("bitcoin 150k", "bitcoin 100k"),
    ("bitcoin 100k", "bitcoin 75k"),
    ("ethereum 10k", "ethereum 5k"),
    ("btc 200k", "btc 100k"),
    ("btc 100k", "btc 75k"),
    ("eth 10k", "eth 5k"),
    ("solana 500", "solana 300"),
    ("solana 300", "solana 200"),
    // === ECONOMICS ===
    ("recession 2025", "gdp negative"),
    ("fed cut", "rate decrease"),
    ("fed cuts 3", "fed cuts 2"),
    ("fed cuts 2", "fed cut"),
    ("inflation below 2", "inflation below 3"),
    ("unemployment above 5", "unemployment above 4"),
    // === SPORTS ===
    ("sweep", "win series"),
    ("win in 4", "win series"),
    ("win in 5", "win series"),
    ("win finals", "reach finals"),
    ("win championship", "reach playoffs"),
    ("super bowl win", "reach super bowl"),
    ("win mvp", "reach playoffs"),
    // === AI & TECH ===
    ("agi by 2026", "agi by 2030"),
    ("agi by 2027", "agi by 2030"),
    ("tesla 500", "tesla 400"),
    ("tesla 400", "tesla 300"),
    ("nvidia 200", "nvidia 150"),
    ("apple 250", "apple 200"),
    // === GENERAL ===
    ("before march", "in 2025"),
    ("before june", "in 2025"),
    ("by q1", "in 2025"),
    ("by q2", "by end of year"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub question: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub outcome_prices: Vec<f64>,
    pub platform: String,
    pub liquidity: f64,
    pub close_date: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Opportunity {
    pub id: String,
    pub opp_type: String,
    pub description: String,
    pub market_a: String,
    pub market_b: String,
    pub platform_a: String,
    pub platform_b: String,
    pub url_a: String,
    pub url_b: String,
    pub buy_yes_price: f64,
    pub buy_no_price: f64,
    pub total_cost: f64,
    pub gross_profit: f64,
    pub net_profit_after_fees: f64,
    pub roi_percent: f64,
    pub suggested_position: f64,
    pub action: String,
}

// Logical dependency between markets
#[derive(Debug, Clone)]
struct MarketDependency {
    implied_market: usize,   // Index of market that is implied
    implying_market: usize,  // Index of market that implies
    dependency_type: String, // "implies" or "mutually_exclusive"
}

pub struct ArbitrageEngine {
    pub min_roi: f64,
    pub min_profit_threshold: f64,
    pub total_capital: f64,
}

impl ArbitrageEngine {
    pub fn new(min_roi: f64, min_profit_threshold: f64, total_capital: f64) -> Self {
        Self {
            min_roi,
            min_profit_threshold: min_profit_threshold.max(MIN_PROFIT_THRESHOLD), // At least $0.05
            total_capital,
        }
    }

    fn get_platform_fee(&self, platform: &str) -> f64 {
        match platform.to_lowercase().as_str() {
            "polymarket" => POLYMARKET_FEE,
            "kalshi" => KALSHI_FEE,
            "manifold" => MANIFOLD_FEE,
            _ => 0.02,
        }
    }

    pub fn analyze_markets(&self, markets: &[Market]) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // 1. Single-platform arbitrage (YES + NO < 1.0)
        let single_opps: Vec<Opportunity> = markets.par_iter()
            .filter_map(|m| self.check_single_platform(m))
            .collect();
        opportunities.extend(single_opps);

        // 2. Cross-platform arbitrage
        let cross_opps = self.check_cross_platform(markets);
        opportunities.extend(cross_opps);

        // 3. COMBINATORIAL ARBITRAGE (Roan's method)
        // Detect logical dependencies and exploit price inconsistencies
        let combinatorial_opps = self.check_combinatorial_arbitrage(markets);
        opportunities.extend(combinatorial_opps);

        // 4. Multi-condition market rebalancing
        let rebalance_opps = self.check_multi_condition_rebalancing(markets);


        opportunities.extend(rebalance_opps);

        // Sort by profit (highest first)
        opportunities.sort_by(|a, b| {
            b.net_profit_after_fees.partial_cmp(&a.net_profit_after_fees)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        opportunities
    }

    fn check_single_platform(&self, market: &Market) -> Option<Opportunity> {
        if market.outcome_prices.len() < 2 {
            return None;
        }

        let yes_price = market.outcome_prices.get(0).copied().unwrap_or(0.0);
        let no_price = market.outcome_prices.get(1).copied().unwrap_or(0.0);

        // Skip markets with unreliable prices
        if yes_price < 0.01 || no_price < 0.01 {
            return None;
        }

        let total_cost = yes_price + no_price;

        // Core invariant: YES + NO = 1.0
        if total_cost < 1.0 && total_cost > 0.0 {
            let gross_profit = 1.0 - total_cost;
            let fee = self.get_platform_fee(&market.platform);
            let total_fees = total_cost * fee * 2.0;
            let net_profit = gross_profit - total_fees;

            if net_profit >= self.min_profit_threshold {
                let roi = (net_profit / total_cost) * 100.0;
                if roi >= self.min_roi * 100.0 {
                    let position_size = self.calculate_position_size(net_profit, total_cost);
                    
                    return Some(Opportunity {
                        id: format!("single_{}", market.id),
                        opp_type: "Single-Platform".into(),
                        description: market.question.clone()
                            .or(market.title.clone())
                            .unwrap_or_default(),
                        market_a: market.id.clone(),
                        market_b: market.id.clone(),
                        platform_a: market.platform.clone(),
                        platform_b: market.platform.clone(),
                        url_a: market.url.clone().unwrap_or_default(),
                        url_b: market.url.clone().unwrap_or_default(),
                        buy_yes_price: yes_price,
                        buy_no_price: no_price,
                        total_cost,
                        gross_profit,
                        net_profit_after_fees: net_profit,
                        roi_percent: roi,
                        suggested_position: position_size,
                        action: format!("Buy YES @${:.2} + NO @${:.2} on {}", 
                            yes_price, no_price, market.platform),
                    });
                }
            }
        }
        None
    }

    fn check_cross_platform(&self, markets: &[Market]) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        let stop_words: HashSet<&str> = ["the", "a", "an", "is", "will", "be", "to", "of", "in", "for", "on", "at", "by"].iter().cloned().collect();

        let polymarket: Vec<&Market> = markets.iter()
            .filter(|m| m.platform == "Polymarket")
            .collect();
        let kalshi: Vec<&Market> = markets.iter()
            .filter(|m| m.platform == "Kalshi")
            .collect();

        // optimization: Pre-compute lowercase text AND word sets to avoid re-allocation in O(N*M) loop
        let poly_data: Vec<(&Market, String, HashSet<String>)> = polymarket.par_iter()
            .map(|m| {
                let text = self.get_market_text(m).to_lowercase();
                let words: HashSet<String> = text.split_whitespace()
                    .filter(|w| !stop_words.contains(w) && w.len() > 2)
                    .map(|w| w.to_string())
                    .collect();
                (*m, text, words)
            })
            .collect();

        let kalshi_data: Vec<(&Market, String, HashSet<String>)> = kalshi.par_iter()
            .map(|m| {
                let text = self.get_market_text(m).to_lowercase();
                let words: HashSet<String> = text.split_whitespace()
                    .filter(|w| !stop_words.contains(w) && w.len() > 2)
                    .map(|w| w.to_string())
                    .collect();
                (*m, text, words)
            })
            .collect();

        // Parallelize the N*M comparison
        let cross_opps: Vec<Opportunity> = poly_data.par_iter()
            .flat_map(|(poly_market, poly_text, poly_words)| {
                let mut local_opps = Vec::new();
                if poly_words.is_empty() { return local_opps; }

                for (kalshi_market, kalshi_text, kalshi_words) in &kalshi_data {
                    if kalshi_words.is_empty() { continue; }

                    // Optimization: Check if length difference is too big (strings can't be similar)
                    if (poly_text.len() as i32 - kalshi_text.len() as i32).abs() > 60 {
                        continue;
                    }

                    let similarity = self.calculate_similarity_sets(poly_words, kalshi_words);
                    
                    if similarity > 0.4 {
                        if let Some(opp) = self.calculate_cross_platform_spread(poly_market, kalshi_market) {
                            local_opps.push(opp);
                        }
                    }
                }
                local_opps
            })
            .collect();

        opportunities.extend(cross_opps);
        opportunities
    }

    fn calculate_similarity_sets(&self, words_a: &HashSet<String>, words_b: &HashSet<String>) -> f64 {
        let intersection: HashSet<_> = words_a.intersection(words_b).collect();
        let union_size = words_a.len() + words_b.len() - intersection.len();

        if union_size == 0 { return 0.0; }
        intersection.len() as f64 / union_size as f64
    }



    /// COMBINATORIAL ARBITRAGE (From Roan's Article)
    /// Detects logical dependencies between markets and exploits price inconsistencies
    /// Key insight: If market A implies market B, then P(A) <= P(B)
    /// If P(A) > P(B), there's arbitrage: sell A, buy B
    fn check_combinatorial_arbitrage(&self, markets: &[Market]) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Build dependency graph
        let dependencies = self.detect_dependencies(markets);

        for dep in &dependencies {
            let implying = &markets[dep.implying_market];
            let implied = &markets[dep.implied_market];

            let implying_yes = implying.outcome_prices.get(0).copied().unwrap_or(0.0);
            let implied_yes = implied.outcome_prices.get(0).copied().unwrap_or(0.0);

            // Skip unreliable prices
            if implying_yes < 0.01 || implied_yes < 0.01 {
                continue;
            }

            // If A implies B, then P(A) must be <= P(B)
            // Violation: P(A) > P(B) creates arbitrage
            if implying_yes > implied_yes + 0.02 { // 2% threshold
                let price_gap = implying_yes - implied_yes;
                
                // Arbitrage: Sell YES on implying (expensive), Buy YES on implied (cheap)
                // But since we can't short easily, we do:
                // Buy NO on implying + Buy YES on implied
                let implying_no = implying.outcome_prices.get(1).copied().unwrap_or(0.0);
                
                let total_cost = implying_no + implied_yes;
                let fee_implying = self.get_platform_fee(&implying.platform);
                let fee_implied = self.get_platform_fee(&implied.platform);
                let total_fees = (implying_no * fee_implying) + (implied_yes * fee_implied);

                // If implying is TRUE → implied is TRUE (we win implied YES)
                // If implying is FALSE → we win implying NO
                // Minimum payout is max(implying_no paid, implied_yes paid) = we cover one side
                // This is a hedge, not pure arbitrage, but captures the mispricing

                let gross_profit = price_gap;
                let net_profit = gross_profit - total_fees;

                if net_profit >= self.min_profit_threshold {
                    let roi = (net_profit / total_cost) * 100.0;
                    
                    let implying_text = self.get_market_text(implying);
                    let implied_text = self.get_market_text(implied);

                    opportunities.push(Opportunity {
                        id: format!("comb_{}_{}", implying.id, implied.id),
                        opp_type: "Combinatorial".into(),
                        description: format!(
                            "LOGICAL: '{}' implies '{}' but priced higher",
                            self.truncate_text(&implying_text, 25),
                            self.truncate_text(&implied_text, 25)
                        ),
                        market_a: implying.id.clone(),
                        market_b: implied.id.clone(),
                        platform_a: implying.platform.clone(),
                        platform_b: implied.platform.clone(),
                        url_a: implying.url.clone().unwrap_or_default(),
                        url_b: implied.url.clone().unwrap_or_default(),
                        buy_yes_price: implied_yes,
                        buy_no_price: implying_no,
                        total_cost,
                        gross_profit,
                        net_profit_after_fees: net_profit,
                        roi_percent: roi,
                        suggested_position: self.calculate_position_size(net_profit, total_cost),
                        action: format!(
                            "Buy NO on '{}' @${:.2} + Buy YES on '{}' @${:.2}",
                            self.truncate_text(&implying_text, 15), implying_no,
                            self.truncate_text(&implied_text, 15), implied_yes
                        ),
                    });
                }
            }
        }

        opportunities
    }

    /// Detect logical dependencies between markets
    fn detect_dependencies(&self, markets: &[Market]) -> Vec<MarketDependency> {
        let mut dependencies = Vec::new();

        // Pre-compute lowercase texts once
        let texts: Vec<String> = markets.iter()
            .map(|m| self.get_market_text(m).to_lowercase())
            .collect();

        // Build keyword index: pattern -> Vec<market_idx>
        let mut keyword_idx_a: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut keyword_idx_b: HashMap<usize, Vec<usize>> = HashMap::new();

        for (i, text) in texts.iter().enumerate() {
            for (p_idx, (pattern_a, pattern_b)) in IMPLICATION_PATTERNS.iter().enumerate() {
                if text.contains(pattern_a) {
                    keyword_idx_a.entry(p_idx).or_default().push(i);
                }
                if text.contains(pattern_b) {
                    keyword_idx_b.entry(p_idx).or_default().push(i);
                }
            }
        }

        // Only check pairs that actually match patterns (avoids O(n²) full scan)
        for (p_idx, _) in IMPLICATION_PATTERNS.iter().enumerate() {
            if let (Some(a_markets), Some(b_markets)) = (keyword_idx_a.get(&p_idx), keyword_idx_b.get(&p_idx)) {
                for &i in a_markets {
                    for &j in b_markets {
                        if i != j {
                            dependencies.push(MarketDependency {
                                implying_market: i,
                                implied_market: j,
                                dependency_type: "implies".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Subset check (only for markets sharing subjects)
        let subjects = [
            // Politics
            "trump", "biden", "harris", "republican", "democrat",
            // Crypto
            "bitcoin", "btc", "ethereum", "eth", "solana", "sol", "xrp", "doge",
            // Sports
            "lakers", "celtics", "warriors", "chiefs", "eagles", "yankees",
            "lebron", "curry", "mahomes", "messi", "ronaldo",
            // Tech/AI
            "tesla", "nvidia", "apple", "google", "openai", "agi",
            // Economics
            "fed", "inflation", "recession", "gdp", "unemployment",
        ];

        // Group markets by subject
        let mut subject_groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, text) in texts.iter().enumerate() {
            for &subj in &subjects {
                if text.contains(subj) {
                    subject_groups.entry(subj).or_default().push(i);
                }
            }
        }

        // Only check subset within same subject group
        for (_, group) in &subject_groups {
            for &i in group {
                for &j in group {
                    if i != j && self.is_subset_market(&texts[i], &texts[j]) {
                        dependencies.push(MarketDependency {
                            implying_market: i,
                            implied_market: j,
                            dependency_type: "subset".to_string(),
                        });
                    }
                }
            }
        }

        dependencies
    }

    fn is_subset_market(&self, text_a: &str, text_b: &str) -> bool {
        let subset_indicators = [
            ("by 5+", "win"),
            ("by 10+", "win"),
            ("landslide", "win"),
            ("sweep", "win"),
            ("before march", "in 2025"),
            ("by june", "in 2025"),
            // Crypto price thresholds
            ("200k", "100k"),
            ("150k", "100k"),
            ("10k", "5k"),
            ("500", "300"),
            // Sports
            ("win in 4", "win series"),
            ("win in 5", "win series"),
            ("win finals", "reach finals"),
            // Fed
            ("cuts 3", "cut"),
            ("cuts 4", "cuts 2"),
        ];

        for (specific, general) in subset_indicators {
            if text_a.contains(specific) && text_b.contains(general) 
                && !text_b.contains(specific) {
                return true;
            }
        }

        false
    }

    /// Multi-condition market rebalancing
    /// If a market has multiple outcomes (A, B, C, D) that sum != 1, there's arbitrage
    fn check_multi_condition_rebalancing(&self, markets: &[Market]) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        for market in markets {
            // Check markets with more than 2 outcomes
            if market.outcome_prices.len() > 2 {
                let total: f64 = market.outcome_prices.iter().sum();
                
                // If sum of all outcomes < 1, buy all (guaranteed $1 payout)
                if total < 1.0 && total > 0.0 {
                    let gross_profit = 1.0 - total;
                    let fee = self.get_platform_fee(&market.platform);
                    let total_fees = total * fee;
                    let net_profit = gross_profit - total_fees;

                    if net_profit >= self.min_profit_threshold {
                        let roi = (net_profit / total) * 100.0;

                        opportunities.push(Opportunity {
                            id: format!("multi_{}", market.id),
                            opp_type: "Multi-Condition".into(),
                            description: format!(
                                "{} outcomes sum to ${:.2} (should be $1.00)",
                                market.outcome_prices.len(),
                                total
                            ),
                            market_a: market.id.clone(),
                            market_b: market.id.clone(),
                            platform_a: market.platform.clone(),
                            platform_b: market.platform.clone(),
                            url_a: market.url.clone().unwrap_or_default(),
                            url_b: market.url.clone().unwrap_or_default(),
                            buy_yes_price: total,
                            buy_no_price: 0.0,
                            total_cost: total,
                            gross_profit,
                            net_profit_after_fees: net_profit,
                            roi_percent: roi,
                            suggested_position: self.calculate_position_size(net_profit, total),
                            action: format!(
                                "Buy ALL {} outcomes on {} for ${:.2}",
                                market.outcome_prices.len(),
                                market.platform,
                                total
                            ),
                        });
                    }
                }
            }
        }

        opportunities
    }

    fn calculate_cross_platform_spread(&self, market_a: &Market, market_b: &Market) -> Option<Opportunity> {
        let yes_a = market_a.outcome_prices.get(0).copied().unwrap_or(0.0);
        let no_a = market_a.outcome_prices.get(1).copied().unwrap_or(0.0);
        let yes_b = market_b.outcome_prices.get(0).copied().unwrap_or(0.0);
        let no_b = market_b.outcome_prices.get(1).copied().unwrap_or(0.0);

        if yes_a == 0.0 || no_a == 0.0 || yes_b == 0.0 || no_b == 0.0 {
            return None;
        }

        // Strategy 1: Buy YES on A + Buy NO on B
        let cost_1 = yes_a + no_b;
        let fee_a = self.get_platform_fee(&market_a.platform);
        let fee_b = self.get_platform_fee(&market_b.platform);
        let fees_1 = (yes_a * fee_a) + (no_b * fee_b);
        let net_profit_1 = 1.0 - cost_1 - fees_1;

        // Strategy 2: Buy YES on B + Buy NO on A
        let cost_2 = yes_b + no_a;
        let fees_2 = (yes_b * fee_b) + (no_a * fee_a);
        let net_profit_2 = 1.0 - cost_2 - fees_2;

        let (best_cost, best_net_profit, buy_yes_market, buy_no_market, buy_yes_price, buy_no_price) = 
            if net_profit_1 > net_profit_2 {
                (cost_1, net_profit_1, market_a, market_b, yes_a, no_b)
            } else {
                (cost_2, net_profit_2, market_b, market_a, yes_b, no_a)
            };

        if best_net_profit >= self.min_profit_threshold && best_cost > 0.0 {
            let roi = (best_net_profit / best_cost) * 100.0;
            
            if roi >= self.min_roi * 100.0 {
                let gross_profit = 1.0 - best_cost;
                let position_size = self.calculate_position_size(best_net_profit, best_cost);
                
                let description = format!(
                    "{}",
                    buy_yes_market.question.clone()
                        .or(buy_yes_market.title.clone())
                        .unwrap_or_default()
                );

                return Some(Opportunity {
                    id: format!("cross_{}_{}", buy_yes_market.id, buy_no_market.id),
                    opp_type: "Cross-Platform".into(),
                    description: self.truncate_text(&description, 50),
                    market_a: buy_yes_market.id.clone(),
                    market_b: buy_no_market.id.clone(),
                    platform_a: buy_yes_market.platform.clone(),
                    platform_b: buy_no_market.platform.clone(),
                    url_a: buy_yes_market.url.clone().unwrap_or_default(),
                    url_b: buy_no_market.url.clone().unwrap_or_default(),
                    buy_yes_price,
                    buy_no_price,
                    total_cost: best_cost,
                    gross_profit,
                    net_profit_after_fees: best_net_profit,
                    roi_percent: roi,
                    suggested_position: position_size,
                    action: format!(
                        "Buy YES @${:.2} on {} + Buy NO @${:.2} on {}",
                        buy_yes_price, buy_yes_market.platform,
                        buy_no_price, buy_no_market.platform
                    ),
                });
            }
        }

        None
    }

    fn get_market_text(&self, market: &Market) -> String {
        format!(
            "{} {} {}",
            market.question.clone().unwrap_or_default(),
            market.title.clone().unwrap_or_default(),
            market.subtitle.clone().unwrap_or_default()
        )
    }

    fn calculate_similarity(&self, text_a: &str, text_b: &str) -> f64 {
        let stop_words = ["the", "a", "an", "is", "will", "be", "to", "of", "in", "for", "on", "at", "by"];
        
        let filter_words = |text: &str| -> HashSet<String> {
            text.split_whitespace()
                .map(|w| w.to_lowercase())
                .filter(|w| !stop_words.contains(&w.as_str()) && w.len() > 2)
                .collect()
        };

        let words_a = filter_words(text_a);
        let words_b = filter_words(text_b);

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection: HashSet<_> = words_a.intersection(&words_b).collect();
        let union_size = words_a.len() + words_b.len() - intersection.len();

        if union_size == 0 { return 0.0; }

        intersection.len() as f64 / union_size as f64
    }

    fn calculate_position_size(&self, net_profit: f64, cost: f64) -> f64 {
        // Kelly Criterion with 25% fraction (conservative)
        let edge = net_profit / cost;
        let win_prob = 0.95;
        let loss_prob = 1.0 - win_prob;
        
        let kelly_fraction = (edge * win_prob) - loss_prob;
        let conservative_kelly = kelly_fraction * 0.25;
        
        if conservative_kelly > 0.0 {
            let max_position = self.total_capital * 0.1;
            (conservative_kelly * self.total_capital).min(max_position).max(0.0)
        } else {
            0.0
        }
    }

    fn truncate_text(&self, text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}...", &text[..max_len])
        } else {
            text.to_string()
        }
    }
}
