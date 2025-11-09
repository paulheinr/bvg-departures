use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as};
use crate::api::BvgClient;

/// Query parameters for GET /stops/:id/departures
///
/// Mirrors https://v6.bvg.transport.rest/api.html#stops-id-departures
#[serde_as]
#[derive(Debug, Clone, Serialize, Default)]
pub struct DeparturesParams {
    /// Date & time to get departures for, e.g. "now" or RFC3339. If None, server uses "now".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    /// Filter departures by direction (stop id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,

    /// Show departures for how many minutes? (default 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,

    /// Max number of departures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<u32>,

    /// Parse & return lines of each stop/station?
    #[serde(rename = "linesOfStops", skip_serializing_if = "Option::is_none")]
    pub lines_of_stops: Option<bool>,

    /// Parse & return hints & warnings?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remarks: Option<bool>,

    /// Response language ("en" default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    // Product filters:
    #[serde(skip_serializing_if = "Option::is_none")] pub suburban: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub subway:   Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub tram:     Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub bus:      Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub ferry:    Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub express:  Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub regional: Option<bool>,

    /// Pretty-print JSON? (server-side)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretty: Option<bool>,
}

/// Typed response. The docs show an envelope with `departures` and an optional timestamp.
/// See example payload in the docs. Fields we don’t strictly need are `Option`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeparturesResponse {
    pub departures: Vec<Departure>,
    #[serde(default)]
    pub realtime_data_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Departure {
    pub trip_id: Option<String>,
    pub direction: Option<String>,

    pub line: Option<Line>,

    /// realtime departure time (RFC3339 with offset), if available
    #[serde(default)]
    pub when: Option<DateTime<Utc>>,
    /// scheduled departure time
    #[serde(default)]
    pub planned_when: Option<DateTime<Utc>>,

    /// delay in seconds
    #[serde(default)]
    pub delay: Option<i64>,

    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub planned_platform: Option<String>,

    #[serde(default)]
    pub stop: Option<Stop>,

    #[serde(default)]
    pub remarks: Option<Vec<Remark>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Line {
    #[serde(default)]
    pub r#type: Option<String>, // "line"
    #[serde(default)]
    pub id: Option<String>,     // e.g. "u6"
    #[serde(default)]
    pub name: Option<String>,   // e.g. "U6"
    #[serde(default)]
    pub mode: Option<String>,   // e.g. "train" | "bus" ...
    #[serde(default)]
    pub product: Option<String>,// e.g. "subway" | "bus"
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stop {
    #[serde(default)]
    pub r#type: Option<String>, // "stop"
    #[serde(default)]
    pub id: Option<String>,     // stop id
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Remark {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,     // e.g. "warning"
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Error type for this module.
#[derive(thiserror::Error, Debug)]
pub enum DeparturesError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL build error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Server returned {status}: {body}")]
    Status { status: reqwest::StatusCode, body: String },
}

impl BvgClient {
    /// GET /stops/:id/departures
    ///
    /// Example equivalent to:
    /// `curl 'https://v6.bvg.transport.rest/stops/900055151/departures?duration=10&linesOfStops=false&remarks=true&language=en'`
    pub async fn get_departures(
        &self,
        stop_id: &str,
        params: &DeparturesParams,
    ) -> Result<DeparturesResponse, DeparturesError> {
        let mut url = self.base.join("stops/")?;
        url.path_segments_mut().expect("url base")
            .pop_if_empty()
            .push(stop_id)
            .push("departures");

        let res = self.http.get(url).query(params).send().await?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(DeparturesError::Status { status, body });
        }
        Ok(res.json::<DeparturesResponse>().await?)
    }
}