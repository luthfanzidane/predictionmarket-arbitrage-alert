use reqwest::Client;
use serde_json::json;
use std::error::Error;
use crate::cross_matcher::CrossMatch;

pub struct TelegramNotifier {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
            chat_id,
        }
    }

    pub async fn send_opportunity(&self, opp: &crate::engine::Opportunity) -> Result<(), Box<dyn Error>> {
        let url_section = if !opp.url_a.is_empty() && opp.url_a == opp.url_b {
            format!("\n🔗 [View Market]({})", opp.url_a)
        } else if !opp.url_a.is_empty() && !opp.url_b.is_empty() {
            format!("\n🔗 [Market A]({})|[Market B]({})", opp.url_a, opp.url_b)
        } else if !opp.url_a.is_empty() {
            format!("\n🔗 [View Market]({})", opp.url_a)
        } else {
            String::new()
        };

        let message = format!(
            "🎯 *{} ARBITRAGE ALERT*\n\n\
            ━━━━━━━━━━━━━━━━━━━━\n\
            📈 *ACTION*:\n{}\n\n\
            💰 *FINANCIALS*:\n\
            ├ Buy YES: ${:.4}\n\
            ├ Buy NO: ${:.4}\n\
            ├ Total Cost: ${:.4}\n\
            ├ Gross Profit: ${:.4}\n\
            ├ Net After Fees: ${:.4}\n\
            └ *ROI: {:.2}%*\n\n\
            💵 *Position Size*: ${:.2}\n\
            (25% Kelly Criterion)\n\n\
            📝 *Market*:\n{}\n\n\
            🏦 *Platforms*: {} ↔ {}{}\n\
            ━━━━━━━━━━━━━━━━━━━━\n\
            ⚡ _Rust HFT Engine | {}ms latency_",
            opp.opp_type.to_uppercase(),
            opp.action,
            opp.buy_yes_price,
            opp.buy_no_price,
            opp.total_cost,
            opp.gross_profit,
            opp.net_profit_after_fees,
            opp.roi_percent,
            opp.suggested_position,
            opp.description,
            opp.platform_a,
            opp.platform_b,
            url_section,
            chrono::Utc::now().timestamp_millis() % 1000
        );

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    pub async fn send_startup_message(&self) -> Result<(), Box<dyn Error>> {
        let message = "🚀 *RUST HFT ARBITRAGE BOT ONLINE*\n\n\
            ━━━━━━━━━━━━━━━━━━━━\n\
            ⚡ *Engine*: Rust (Low-Latency)\n\
            📡 *Sources*: Polymarket + Kalshi + Manifold\n\
            🔍 *Strategies*:\n\
            ├ Single-Platform (YES+NO<1)\n\
            ├ Cross-Platform (Roan's Method)\n\
            └ Heuristic Matching (Entity+Category+Team+Year)\n\n\
            💰 *Fee Calculation*: Enabled\n\
            📊 *Position Sizing*: 25% Kelly\n\
            ━━━━━━━━━━━━━━━━━━━━\n\
            _Scanning 7000+ markets every cycle..._";

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    pub async fn send_summary(&self, total_markets: usize, opportunities: usize, scan_time_ms: u64) -> Result<(), Box<dyn Error>> {
        // Only send if opportunities found
        if opportunities == 0 {
            return Ok(());
        }

        let message = format!(
            "📊 *Scan Summary*\n\
            Markets: {} | Opps: {} | Time: {}ms",
            total_markets, opportunities, scan_time_ms
        );

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    pub async fn send_cross_match(&self, m: &CrossMatch) -> Result<(), Box<dyn Error>> {
        let url_section = if !m.url_a.is_empty() && !m.url_b.is_empty() {
            format!("\n🔗 [Market A]({})|[Market B]({})", m.url_a, m.url_b)
        } else {
            String::new()
        };

        let message = format!(
            "🔗 *CROSS-PLATFORM MATCH*\n\n\
            ━━━━━━━━━━━━━━━━━━━━\n\
            📊 *Category*: {}\n\
            📈 *Confidence*: {:.0}%\n\n\
            🅰️ *{}*:\n\
            {} (YES: ${:.3})\n\n\
            🅱️ *{}*:\n\
            {} (YES: ${:.3})\n\n\
            💰 *Price Diff*: {:.1}%\n\
            🏷️ *Entities*: {}{}\n\
            ━━━━━━━━━━━━━━━━━━━━",
            m.category.to_uppercase(),
            m.confidence * 100.0,
            m.platform_a,
            m.question_a,
            m.yes_price_a,
            m.platform_b,
            m.question_b,
            m.yes_price_b,
            m.price_diff * 100.0,
            m.shared_entities.join(", "),
            url_section,
        );

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "Markdown"
        });

        self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }
}
