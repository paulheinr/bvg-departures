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
}

pub(super) fn build_display_lines(
    resp: &Vec<(String, crate::api::departures::DeparturesResponse)>,
) -> Vec<(String, Vec<DisplayEntry>)> {
    use chrono::Utc;
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

            entries.push(DisplayEntry {
                line,
                dir,
                actual_mins,
                delay_mins,
                symbol,
                hex,
            });
        }
        out.push((station_name.clone(), entries));
    }
    out
}

pub(super) mod std_out {
    use crate::api::departures::DeparturesResponse;
    use colored::{Color, ColoredString, Colorize};
    use tracing::info;

    use super::build_display_lines;

    pub fn display_plain(resp: Vec<(String, DeparturesResponse)>) {
        info!("Got departures for {} stations. Display now.", resp.len());

        let grouped = build_display_lines(&resp);
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
}

pub(super) mod tui {
    use crate::api::departures::DeparturesResponse;

    // Simple TUI renderer: build a text representation and render it inside a Paragraph composed of styled spans.
    pub fn display_tui(resp: Vec<(String, DeparturesResponse)>) -> anyhow::Result<()> {
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::execute;
        use crossterm::terminal::{
            disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        };
        use std::io::stdout;
        use tui::layout::Alignment;
        use tui::style::{Color as TuiColor, Modifier, Style};
        use tui::text::{Span, Spans, Text};
        use tui::widgets::{Block, Borders, Paragraph};
        use tui::{backend::CrosstermBackend, Terminal};

        use super::build_display_lines;

        let grouped = build_display_lines(&resp);

        // Build lines as Vec<Spans> so we can style the line token separately
        let mut lines: Vec<Spans> = Vec::new();

        lines.push(Spans::from(Span::raw(format!(
            "Got departures for {} stations",
            grouped.len()
        ))));
        lines.push(Spans::from(Span::raw("")));

        for (name, entries) in grouped {
            lines.push(Spans::from(Span::raw(format!("Station: {}", name))));
            for e in entries {
                let (r, g, b) = hex_to_rgb(e.hex);
                let tui_color = TuiColor::Rgb(r, g, b);
                let delay_text = match e.delay_mins {
                    Some(d) if d != 0 => format!(" ({:+}min)", d),
                    _ => String::new(),
                };

                // Compose spans: symbol, styled line, and the rest as raw text
                let span_vec = vec![
                    Span::raw(format!("{} ", e.symbol)),
                    Span::styled(
                        format!("{:<6}", e.line),
                        Style::default().fg(tui_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(
                        "|{:<35}|{:02}min{}",
                        e.dir, e.actual_mins, delay_text
                    )),
                ];

                lines.push(Spans::from(span_vec));
            }
            lines.push(Spans::from(Span::raw("")));
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Render once and wait for 'q' to quit
        terminal.draw(|f| {
            let size = f.size();
            let paragraph = Paragraph::new(Text::from(lines.clone()))
                .block(Block::default().borders(Borders::ALL).title("Departures"))
                .alignment(Alignment::Left);
            f.render_widget(paragraph, size);
        })?;

        // Wait for user to press 'q' to quit. Timeout every 250ms to keep responsive (no refresh behavior implemented).
        loop {
            if event::poll(std::time::Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }

        // restore terminal
        disable_raw_mode()?;
        // Leave alternate screen and restore terminal state
        execute!(std::io::stdout(), LeaveAlternateScreen)?;

        Ok(())
    }

    fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        (r, g, b)
    }
}
