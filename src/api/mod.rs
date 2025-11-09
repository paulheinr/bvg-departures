pub mod departures;

use reqwest::Url;

/// Minimal API client. Reuse across calls.
#[derive(Clone)]
pub struct BvgClient {
    http: reqwest::Client,
    base: Url,
}

impl Default for BvgClient {
    fn default() -> Self {
        Self::new(Url::parse("https://v6.bvg.transport.rest/").unwrap())
    }
}

impl BvgClient {
    pub fn new(base: Url) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("bvg-api/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client");
        Self { http, base }
    }
}