use crate::api::departures::DeparturesResponse;
use crate::view::ResultDisplay;
use colored::{Color, ColoredString, Colorize};
use tracing::info;

pub struct StdoutDisplay {}

impl ResultDisplay for StdoutDisplay {
    fn display(&self, resp: Vec<(String, DeparturesResponse)>) -> anyhow::Result<()> {
        info!("Got departures for {} stations. Display now.", resp.len());

        let grouped = crate::view::build_display_lines(&resp);
        for (name, entries) in grouped {
            println!("Station: {}", name);
            for e in entries {
                let line_colored = color_line(&e.line, e.hex);
                let delay_text = match e.delay_mins {
                    Some(d) if d != 0 => format!(" ({:+}min)", d),
                    _ => String::new(),
                };

                println!(
                    "{} {:<6}|{:<35}|{:02}min{}",
                    e.symbol, line_colored, e.dir, e.actual_mins, delay_text
                );
            }
            println!();
        }

        Ok(())
    }
}

/// Watch out: If the terminal does not support true color, the colors may look different!
/// This is the case with the RustRover internal terminal.
fn color_line(line: &str, hex: &str) -> ColoredString {
    // Use the supplied hex color and convert to colored::Color
    line.color(hex_to_color(hex)).bold()
}

fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    // Parse the 3 pairs of hex digits
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

    Color::TrueColor { r, g, b }
}
