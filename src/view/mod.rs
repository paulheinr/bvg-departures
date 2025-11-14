use crate::api::departures::DeparturesResponse;
use async_trait::async_trait;

pub(crate) mod std_out;
pub(crate) mod tui;

#[async_trait]
pub(super) trait ResultDisplay {
    async fn display(&self) -> anyhow::Result<()>;
}

pub(super) fn product_symbol(product: &str) -> &'static str {
    match product {
        p if p.contains("subway") => "🚇",
        p if p.contains("suburban") => "🚆",
        p if p.contains("bus") => "🚌",
        p if p.contains("tram") => "🚃",
        _ => "🚀",
    }
}

pub(super) fn product_hex(product: &str) -> &'static str {
    match product {
        p if p.contains("subway") => "#00539F",
        p if p.contains("suburban") => "#00854A",
        p if p.contains("bus") => "#95276E",
        p if p.contains("tram") => "#BE1414",
        _ => "#00FFFF",
    }
}

// Shared display entry and builder to avoid duplicated formatting logic between std_out and tui
pub(super) struct DisplayEntry {
    pub line: String,
    pub dir: String,
    pub actual_mins: i64,
    pub delay_mins: Option<i64>,
    pub symbol: &'static str,
    pub hex: &'static str,
    // Absolute departure time formatted as HH:MM in local time (None if unknown)
    pub abs_time: Option<String>,
}

pub(super) fn build_display_lines(
    resp: &Vec<(String, DeparturesResponse)>,
) -> Vec<(String, Vec<DisplayEntry>)> {
    use chrono::{Local, Utc};
    let mut out: Vec<(String, Vec<DisplayEntry>)> = Vec::new();
    for (station_name, departures) in resp.iter() {
        let mut entries: Vec<DisplayEntry> = Vec::new();
        for d in &departures.departures {
            let line = d
                .line
                .as_ref()
                .and_then(|l| l.name.as_ref())
                .map(|s| s.clone())
                .unwrap_or_else(|| "?".to_string());

            let product = d
                .line
                .as_ref()
                .and_then(|l| l.product.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("");

            let symbol = product_symbol(product);
            let hex = product_hex(product);

            let dir = d.direction.as_deref().unwrap_or("").to_string();
            let actual_mins = d
                .when
                .map(|w| (w - Utc::now()).num_seconds() / 60)
                .unwrap_or_default()
                .max(0);
            let delay_mins = d.delay.map(|d| d / 60);

            let abs_time = d
                .when
                .map(|w| w.with_timezone(&Local).format("%H:%M").to_string());

            entries.push(DisplayEntry {
                line,
                dir,
                actual_mins,
                delay_mins,
                symbol,
                hex,
                abs_time,
            });
        }
        out.push((station_name.clone(), entries));
    }
    out
}
