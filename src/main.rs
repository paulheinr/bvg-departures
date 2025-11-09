use std::{fs};
use crate::api::BvgClient;
use crate::api::departures::{DeparturesResponse};

mod api;
use serde::Deserialize;
use clap::Parser;
use log::info;

#[derive(Debug, Deserialize)]
pub struct InputStops {
    pub stops: Vec<InputStop>,
}

#[derive(Debug, Deserialize)]
pub struct InputStop {
    pub id: String,
    pub name: String,
    #[serde(default = "u32_value_15")]
    look_ahead: u32,
    // directions can be missing or empty, so Option<Vec<String>> is safe
    /// Directions do not need to match the BVG-API response. It is used for filtering during post-processing.
    #[serde(default)]
    pub directions: Vec<String>,
}

fn u32_value_15() -> u32 {
    15
}

#[derive(Parser, Debug)]
struct Cli {
    /// The path to the file to read
    path: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    info!("Starting with {}", args.path.display());

    let stops: InputStops = serde_yaml::from_str(&fs::read_to_string(args.path)?)?;

    let client = BvgClient::default();
    let result = client.get_departures(stops).await?;

    display_result(result);

    Ok(())
}

fn display_result(resp: Vec<(String, DeparturesResponse)>) {
    info!("Got {} departures. Display now.", resp.len());

    for (name, departures) in resp {
        println!("Station: {}", name);
        // println!("line  |direction                          |actual");
        for d in &departures.departures {
            let line = d.line.as_ref().and_then(|l| l.name.as_ref()).map(String::as_str).unwrap_or("?");
            let dir = d.direction.as_deref().unwrap_or("");
            let actual_mins = d.when.map(|w| (w - chrono::Utc::now()).num_seconds() / 60);
            let delay = d.delay.map(|d| d / 60);
            let delay_text = match delay {
                Some(d) if d != 0 => format!(" ({:+}min)", d), // note the `+` for explicit sign
                _ => String::new(),
            };
            println!("{:<6}|{:<35}|{:02}min{}",
                     line,
                     dir,
                     actual_mins.unwrap_or_default().max(0),
                     delay_text
            );
        }
        println!();
    }
}
