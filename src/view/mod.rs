pub(super) mod std_out {
    use crate::api::departures::DeparturesResponse;
    use chrono::Utc;
    use colored::{Color, ColoredString, Colorize};
    use tracing::info;

    pub fn display_plain(resp: Vec<(String, DeparturesResponse)>) {
        info!("Got departures for {} stations. Display now.", resp.len());
        for (name, departures) in resp {
            println!("Station: {}", name);
            // println!("line  |direction                          |actual");
            for d in &departures.departures {
                let line = d
                    .line
                    .as_ref()
                    .and_then(|l| l.name.as_ref())
                    .map(String::as_str)
                    .unwrap_or("?");

                let product = d
                    .line
                    .as_ref()
                    .and_then(|l| l.product.as_ref())
                    .map(String::as_str)
                    .unwrap_or_default();

                let line_colored = color_line(line, product);
                let dir = d.direction.as_deref().unwrap_or("");
                let actual_mins = d.when.map(|w| (w - Utc::now()).num_seconds() / 60);
                let delay_text = match d.delay.map(|d| d / 60) {
                    Some(d) if d != 0 => format!(" ({:+}min)", d), // note the `+` for explicit sign
                    _ => String::new(),
                };
                let symbol = line_symbole(product);

                println!(
                    "{} {:<6}|{:<35}|{:02}min{}",
                    symbol,
                    line_colored,
                    dir,
                    actual_mins.unwrap_or_default().max(0),
                    delay_text
                );
            }
            println!();
        }
    }

    /// Watch out: If the terminal does not support true color, the colors may look different!
    /// This is the case with the RustRover internal terminal.
    fn color_line(line: &str, product: &str) -> ColoredString {
        match product {
            p if p.contains("subway") => line.color(hex_to_color("#00539F")).bold(),
            p if p.contains("suburban") => line.color(hex_to_color("#00854A")).bold(),
            p if p.contains("bus") => line.color(hex_to_color("#95276E")).bold(),
            p if p.contains("tram") => line.color(hex_to_color("#BE1414")).bold(),
            p => p.cyan().bold(),
        }
    }

    fn line_symbole(product: &str) -> String {
        match product {
            p if p.contains("subway") => "🚇".to_string(),
            p if p.contains("suburban") => "🚆".to_string(),
            p if p.contains("bus") => "🚌".to_string(),
            p if p.contains("tram") => "🚃".to_string(),
            _ => "🚀".to_string(),
        }
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
    use chrono::Utc;

    // Simple TUI renderer: build a text representation and render it inside a single Paragraph widget.
    pub fn display_tui(resp: Vec<(String, DeparturesResponse)>) -> anyhow::Result<()> {
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::execute;
        use crossterm::terminal::{
            disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        };
        use std::io::stdout;
        use tui::layout::Alignment;
        use tui::widgets::{Block, Borders, Paragraph};
        use tui::{backend::CrosstermBackend, Terminal};

        // Assemble the textual content
        let mut buf = String::new();
        buf.push_str(&format!("Got departures for {} stations\n\n", resp.len()));
        for (name, departures) in resp {
            buf.push_str(&format!("Station: {}\n", name));
            for d in &departures.departures {
                let line = d
                    .line
                    .as_ref()
                    .and_then(|l| l.name.as_ref())
                    .map(String::as_str)
                    .unwrap_or("?");
                let dir = d.direction.as_deref().unwrap_or("");
                let actual_mins = d.when.map(|w| (w - Utc::now()).num_seconds() / 60);
                let delay = d.delay.map(|d| d / 60);
                let delay_text = match delay {
                    Some(d) if d != 0 => format!(" ({:+}min)", d),
                    _ => String::new(),
                };
                buf.push_str(&format!(
                    "{:<6}|{:<35}|{:02}min{}\n",
                    line,
                    dir,
                    actual_mins.unwrap_or_default().max(0),
                    delay_text
                ));
            }
            buf.push_str("\n");
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
            let paragraph = Paragraph::new(buf.as_str())
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
}
