use crate::engine::Market;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// A cross-platform market match
#[derive(Debug, Serialize, Clone)]
pub struct CrossMatch {
    pub platform_a: String,
    pub platform_b: String,
    pub id_a: String,
    pub id_b: String,
    pub question_a: String,
    pub question_b: String,
    pub yes_price_a: f64,
    pub yes_price_b: f64,
    pub price_diff: f64,
    pub confidence: f64,
    pub category: String,
    pub shared_entities: Vec<String>,
    pub url_a: String,
    pub url_b: String,
}

// Sports team lists
const NBA_TEAMS: &[&str] = &[
    "celtics", "nets", "knicks", "76ers", "raptors",
    "bulls", "cavaliers", "pistons", "pacers", "bucks",
    "hawks", "hornets", "heat", "magic", "wizards",
    "nuggets", "timberwolves", "thunder", "trail blazers", "jazz",
    "warriors", "clippers", "lakers", "suns", "kings",
    "mavericks", "rockets", "grizzlies", "pelicans", "spurs",
];

const NFL_TEAMS: &[&str] = &[
    "patriots", "bills", "dolphins", "jets",
    "steelers", "ravens", "bengals", "browns",
    "texans", "colts", "jaguars", "titans",
    "chiefs", "raiders", "chargers", "broncos",
    "eagles", "commanders", "giants", "cowboys",
    "packers", "bears", "vikings", "lions",
    "buccaneers", "saints", "falcons", "panthers",
    "49ers", "seahawks", "rams", "cardinals",
];

const SPORTS_CATEGORIES: &[&str] = &["nba", "nfl", "soccer", "mlb"];

struct ProcessedMarket {
    text: String,
    entities: HashSet<String>,
    teams: HashSet<String>,
    category: Option<String>,
}

pub struct CrossMatcher {
    entity_patterns: Vec<(&'static str, Regex)>,
    extra_terms: Vec<&'static str>,
    categories: Vec<(&'static str, Vec<&'static str>)>,
    year_re: Regex,
    season_re: Regex,
}

impl CrossMatcher {
    pub fn new() -> Self {
        let entity_patterns = vec![
            ("trump", Regex::new(r"(?i)\btrump\b").unwrap()),
            ("biden", Regex::new(r"(?i)\bbiden\b").unwrap()),
            ("harris", Regex::new(r"(?i)\bharris\b").unwrap()),
            ("desantis", Regex::new(r"(?i)\bdesantis\b").unwrap()),
            ("vance", Regex::new(r"(?i)\bvance\b").unwrap()),
            ("newsom", Regex::new(r"(?i)\bnewsom\b").unwrap()),
            ("haley", Regex::new(r"(?i)\bhaley\b").unwrap()),
            ("obama", Regex::new(r"(?i)\bobama\b").unwrap()),
            ("bitcoin", Regex::new(r"(?i)\b(bitcoin|btc)\b").unwrap()),
            ("ethereum", Regex::new(r"(?i)\b(ethereum|eth)\b").unwrap()),
            ("solana", Regex::new(r"(?i)\b(solana|sol)\b").unwrap()),
            ("fed", Regex::new(r"(?i)\b(fed|federal reserve|fomc)\b").unwrap()),
            ("inflation", Regex::new(r"(?i)\binflation\b").unwrap()),
            ("recession", Regex::new(r"(?i)\brecession\b").unwrap()),
            ("super_bowl", Regex::new(r"(?i)\bsuper bowl\b").unwrap()),
            ("nba_finals", Regex::new(r"(?i)\bnba finals?\b").unwrap()),
            ("world_series", Regex::new(r"(?i)\bworld series\b").unwrap()),
            ("world_cup", Regex::new(r"(?i)\bworld cup\b").unwrap()),
            ("champions_league", Regex::new(r"(?i)\bchampions league\b").unwrap()),
        ];

        let extra_terms = vec![
            "president", "election", "win", "price", "champion", "finals", "nominee",
        ];

        let categories = vec![
            ("us_politics", vec!["trump", "biden", "harris", "desantis", "president", "election", "congress", "senate", "republican", "democrat", "white house"]),
            ("crypto", vec!["bitcoin", "ethereum", "btc", "eth", "crypto", "solana"]),
            ("economics", vec!["fed", "inflation", "recession", "gdp", "interest rate", "unemployment"]),
            ("nba", vec!["nba", "basketball", "lakers", "celtics", "warriors", "finals"]),
            ("nfl", vec!["nfl", "super bowl", "football", "patriots", "chiefs"]),
            ("soccer", vec!["world cup", "fifa", "soccer", "premier league", "champions league"]),
            ("mlb", vec!["mlb", "baseball", "world series"]),
            ("ai_tech", vec!["openai", "chatgpt", "google", "apple", "tesla", "nvidia", "ai"]),
        ];

        Self {
            entity_patterns,
            extra_terms,
            categories,
            year_re: Regex::new(r"\b(202[0-9]|203[0-9])\b").unwrap(),
            season_re: Regex::new(r"\b(202[0-9])-(202[0-9])\b").unwrap(),
        }
    }

    /// Match markets across all platform pairs
    pub fn match_all(&self, all_markets: &HashMap<String, Vec<&Market>>) -> Vec<CrossMatch> {
        let platforms: Vec<&String> = all_markets.keys().collect();
        let mut all_matches = Vec::new();

        for i in 0..platforms.len() {
            for j in (i + 1)..platforms.len() {
                let pa = platforms[i];
                let pb = platforms[j];
                let markets_a = &all_markets[pa];
                let markets_b = &all_markets[pb];
                
                println!("  🔍 {} vs {} ({} x {} markets)...", pa, pb, markets_a.len(), markets_b.len());
                
                let matches = self.match_pair(markets_a, markets_b);
                println!("     → {} matches!", matches.len());
                all_matches.extend(matches);
            }
        }

        // Sort by confidence descending
        all_matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        all_matches
    }

    fn match_pair(&self, markets_a: &[&Market], markets_b: &[&Market]) -> Vec<CrossMatch> {
        // Pre-process all markets
        let processed_a: Vec<(&Market, ProcessedMarket)> = markets_a.iter()
            .map(|m| (*m, self.process(m)))
            .collect();
        let processed_b: Vec<(&Market, ProcessedMarket)> = markets_b.iter()
            .map(|m| (*m, self.process(m)))
            .collect();

        // Group B by category for faster lookup
        let mut b_by_cat: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, (_, proc)) in processed_b.iter().enumerate() {
            if let Some(ref cat) = proc.category {
                b_by_cat.entry(cat.clone()).or_default().push(idx);
            }
        }

        let mut matches = Vec::new();

        for (raw_a, proc_a) in &processed_a {
            let cat_a = match &proc_a.category {
                Some(c) => c,
                None => continue,
            };

            let b_indices = match b_by_cat.get(cat_a) {
                Some(indices) => indices,
                None => continue,
            };

            for &b_idx in b_indices {
                let (raw_b, proc_b) = &processed_b[b_idx];

                // Entity overlap check
                let shared: HashSet<&String> = proc_a.entities.intersection(&proc_b.entities).collect();
                if shared.len() < 2 {
                    continue;
                }

                // STRICT YEAR CHECK
                let years_a = self.extract_years(&proc_a.text);
                let years_b = self.extract_years(&proc_b.text);
                if !years_a.is_empty() && !years_b.is_empty() {
                    if years_a.intersection(&years_b).count() == 0 {
                        continue; // Different years
                    }
                }

                // CLOSE DATE CHECK: within 90 days
                if let (Some(ref cd_a), Some(ref cd_b)) = (&raw_a.close_date, &raw_b.close_date) {
                    if let (Ok(da), Ok(db)) = (
                        cd_a.parse::<DateTime<Utc>>(),
                        cd_b.parse::<DateTime<Utc>>(),
                    ) {
                        let gap_days = (da - db).num_days().unsigned_abs();
                        if gap_days > 90 {
                            continue; // Too far apart
                        }
                    }
                }

                // STRICT SPORTS CHECK
                let is_sports = SPORTS_CATEGORIES.contains(&cat_a.as_str());
                if is_sports {
                    if proc_a.teams.is_empty() || proc_b.teams.is_empty() {
                        continue;
                    }
                    if proc_a.teams.intersection(&proc_b.teams).count() == 0 {
                        continue;
                    }
                }

                // Calculate confidence
                let mut confidence = shared.len() as f64 * 0.2;
                if !years_a.is_empty() && !years_b.is_empty() 
                    && years_a.intersection(&years_b).count() > 0 {
                    confidence += 0.3;
                }
                if is_sports {
                    let team_overlap = proc_a.teams.intersection(&proc_b.teams).count();
                    confidence += team_overlap as f64 * 0.3;
                }
                confidence = confidence.min(1.0);

                if confidence < 0.5 {
                    continue;
                }

                let yes_a = raw_a.outcome_prices.first().copied().unwrap_or(0.0);
                let yes_b = raw_b.outcome_prices.first().copied().unwrap_or(0.0);
                let price_diff = (yes_a - yes_b).abs();

                let q_a = self.get_question(raw_a);
                let q_b = self.get_question(raw_b);

                matches.push(CrossMatch {
                    platform_a: raw_a.platform.clone(),
                    platform_b: raw_b.platform.clone(),
                    id_a: raw_a.id.clone(),
                    id_b: raw_b.id.clone(),
                    question_a: if q_a.len() > 100 { q_a[..100].to_string() } else { q_a },
                    question_b: if q_b.len() > 100 { q_b[..100].to_string() } else { q_b },
                    yes_price_a: yes_a,
                    yes_price_b: yes_b,
                    price_diff: (price_diff * 10000.0).round() / 10000.0,
                    confidence,
                    category: cat_a.clone(),
                    shared_entities: shared.into_iter().cloned().collect(),
                    url_a: raw_a.url.clone().unwrap_or_default(),
                    url_b: raw_b.url.clone().unwrap_or_default(),
                });
            }
        }

        // Deduplicate
        let mut seen: HashSet<(String, String)> = HashSet::new();
        matches.retain(|m| {
            let key = (m.id_a.clone(), m.id_b.clone());
            seen.insert(key)
        });

        matches
    }

    fn process(&self, market: &Market) -> ProcessedMarket {
        let text = self.get_question(market).to_lowercase();
        
        let mut entities = HashSet::new();
        for (name, re) in &self.entity_patterns {
            if re.is_match(&text) {
                entities.insert(name.to_string());
            }
        }
        for term in &self.extra_terms {
            if text.contains(term) {
                entities.insert(term.to_string());
            }
        }

        let mut teams = HashSet::new();
        for team in NBA_TEAMS.iter().chain(NFL_TEAMS.iter()) {
            if text.contains(team) {
                teams.insert(team.to_string());
            }
        }

        let category = self.classify(&text);

        ProcessedMarket { text, entities, teams, category }
    }

    fn classify(&self, text: &str) -> Option<String> {
        for (cat_name, keywords) in &self.categories {
            let count = keywords.iter()
                .filter(|kw| text.contains(**kw))
                .count();
            if count >= 2 {
                return Some(cat_name.to_string());
            }
        }
        None
    }

    fn extract_years(&self, text: &str) -> HashSet<String> {
        let mut years = HashSet::new();
        for cap in self.year_re.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                years.insert(m.as_str().to_string());
            }
        }
        for cap in self.season_re.captures_iter(text) {
            if let Some(y1) = cap.get(1) {
                years.insert(y1.as_str().to_string());
            }
            if let Some(y2) = cap.get(2) {
                years.insert(y2.as_str().to_string());
            }
        }
        years
    }

    fn get_question(&self, market: &Market) -> String {
        if let Some(ref q) = market.question {
            if !q.is_empty() {
                return q.clone();
            }
        }
        if let Some(ref t) = market.title {
            if let Some(ref s) = market.subtitle {
                return format!("{} - {}", t, s);
            }
            return t.clone();
        }
        String::new()
    }
}
