use crate::api::departures::DeparturesResponse;
use crate::view::ResultDisplay;
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

pub struct TuiDisplay {}

impl ResultDisplay for TuiDisplay {
    fn display(&self, resp: Vec<(String, DeparturesResponse)>) -> anyhow::Result<()> {
        let grouped = crate::view::build_display_lines(&resp);

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
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    (r, g, b)
}
