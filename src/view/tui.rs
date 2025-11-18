use crate::api::departures::DeparturesApi;
use crate::view::{DisplayEntry, ResultDisplay};
use crate::InputStops;
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use derive_builder::Builder;
use std::io::{stdout, Stdout};
use tui::layout::Alignment;
use tui::style::{Color as TuiColor, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, Paragraph};
use tui::{backend::CrosstermBackend, Terminal};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct TuiDisplay<D: DeparturesApi> {
    api_client: D,
    stops: InputStops,
}

#[async_trait]
impl<D: DeparturesApi + Sync> ResultDisplay for TuiDisplay<D> {
    async fn display(&self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let resp = self.api_client.get_departures(&self.stops).await?;
        let mut display_lines = crate::view::build_display_lines(&resp);

        Self::render(&display_lines, &mut terminal)?;

        // Wait for user to press 'q' to quit. Timeout every 250ms to keep responsive (no refresh behavior implemented).
        loop {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        // Refresh: re-fetch departures and re-render
                        let resp = self.api_client.get_departures(&self.stops).await?;
                        display_lines = crate::view::build_display_lines(&resp);
                        Self::render(&display_lines, &mut terminal)?;
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    // Re-render using the current terminal size
                    Self::render(&display_lines, &mut terminal)?;
                }
                _ => {}
            }
        }

        // restore terminal
        disable_raw_mode()?;
        // Leave alternate screen and restore terminal state
        execute!(std::io::stdout(), LeaveAlternateScreen)?;

        Ok(())
    }
}

impl<D: DeparturesApi> TuiDisplay<D> {
    fn render(
        display_lines: &Vec<(String, Vec<DisplayEntry>)>,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), anyhow::Error> {
        // Render once and wait for 'q' to quit
        terminal.draw(|f| {
            let size = f.size();

            // Build header with current time right-aligned within the content area
            let now = Local::now();
            let now_str = now.format("%H:%M:%S").to_string();
            let header_line = format!("Request time: {}", now_str);

            // Build the lines for the entries
            let mut spans: Vec<Spans> = Vec::new();
            spans.push(Spans::from(Span::styled(
                header_line,
                Style::default().add_modifier(Modifier::BOLD),
            )));

            spans.push(Spans::from(Span::raw("")));

            for (name, entries) in display_lines {
                spans.push(Spans::from(Span::styled(
                    format!("Station: {}", name),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                )));

                for e in entries {
                    let (r, g, b) = hex_to_rgb(e.hex);
                    let tui_color = TuiColor::Rgb(r, g, b);
                    let delay_text = match e.delay_mins {
                        Some(d) if d != 0 => format!(" ({:+}min)", d),
                        _ => String::new(),
                    };

                    let abs_text = e
                        .abs_time
                        .as_ref()
                        .map(|t| format!("{}", t))
                        .unwrap_or_else(|| String::from("--"));

                    // Compose spans: symbol, styled line, absolute time, and the rest as raw text
                    let span_vec = vec![
                        Span::raw(format!("{} ", e.symbol)),
                        Span::styled(
                            format!("{:<5}", e.line),
                            Style::default().bg(tui_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!(
                            "| {:<30} | {:>5} | {:2}min{}",
                            e.dir, abs_text, e.actual_mins, delay_text
                        )),
                    ];

                    spans.push(Spans::from(span_vec));
                }
                spans.push(Spans::from(Span::raw("")));
            }

            let paragraph = Paragraph::new(Text::from(spans))
                .block(Block::default().borders(Borders::ALL).title("Departures"))
                .alignment(Alignment::Left);
            f.render_widget(paragraph, size);
        })?;
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
