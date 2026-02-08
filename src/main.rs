use crate::api::BvgClient;
use std::fs;

mod api;
mod view;

use crate::view::std_out::StdoutDisplayBuilder;
use crate::view::tui::{LogBuffer, TuiDisplayBuilder};
use crate::view::ResultDisplay;
use clap::Parser;
use serde::Deserialize;
use tracing::info;

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

    /// Use a simple TUI for display
    #[clap(long, action)]
    #[clap(default_value = "true")]
    tui: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let log_buffer = if args.tui {
        let log_buffer = LogBuffer::new(8);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(log_buffer.make_writer())
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        Some(log_buffer)
    } else {
        // construct a subscriber that prints formatted traces to stdout
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        // use that subscriber to process traces emitted after this point
        tracing::subscriber::set_global_default(subscriber)?;
        None
    };

    info!("Starting with {}", args.path.display());

    let stops: InputStops = serde_yaml::from_str(&fs::read_to_string(args.path)?)?;

    let display: Box<dyn ResultDisplay> = if args.tui {
        let log_buffer = log_buffer.expect("log buffer for tui");
        Box::new(
            TuiDisplayBuilder::<BvgClient>::default()
                .stops(stops)
                .api_client(BvgClient::default())
                .log_buffer(log_buffer)
                .build()?,
        )
    } else {
        Box::new(
            StdoutDisplayBuilder::<BvgClient>::default()
                .stops(stops)
                .api_client(BvgClient::default())
                .build()?,
        )
    };

    display.display().await?;

    Ok(())
}
