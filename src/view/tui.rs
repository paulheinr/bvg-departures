use crate::api::departures::DeparturesApi;
use crate::view::{DisplayEntry, ResultDisplay};
use crate::InputStops;
use async_trait::async_trait;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use derive_builder::Builder;
use std::collections::VecDeque;
use std::io;
use std::io::{stdout, Stdout};
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::fmt::writer::MakeWriter;
use tui::layout::Alignment;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color as TuiColor, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Borders, Paragraph};
use tui::{backend::CrosstermBackend, Terminal};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone)]
pub struct LogBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
    max_lines: usize,
}

impl LogBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::with_capacity(max_lines))),
            max_lines,
        }
    }

    pub fn make_writer(&self) -> LogBufferWriterFactory {
        LogBufferWriterFactory {
            buffer: self.clone(),
        }
    }

    fn push_line(&self, line: String) {
        let mut lines = self.lines.lock().expect("log buffer lock");
        lines.push_back(line);
        while lines.len() > self.max_lines {
            lines.pop_front();
        }
    }

    fn snapshot(&self) -> Vec<String> {
        let lines = self.lines.lock().expect("log buffer lock");
        lines.iter().cloned().collect()
    }
}

pub struct LogBufferWriterFactory {
    buffer: LogBuffer,
}

impl<'a> MakeWriter<'a> for LogBufferWriterFactory {
    type Writer = LogBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogBufferWriter {
            buffer: self.buffer.clone(),
            pending: String::new(),
        }
    }
}

pub struct LogBufferWriter {
    buffer: LogBuffer,
    pending: String,
}

impl io::Write for LogBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let chunk = String::from_utf8_lossy(buf);
        self.pending.push_str(&chunk);

        while let Some(pos) = self.pending.find('\n') {
            let line = self.pending[..pos].to_string();
            self.buffer.push_line(line);
            self.pending = self.pending[pos + 1..].to_string();
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.pending.is_empty() {
            let line = std::mem::take(&mut self.pending);
            self.buffer.push_line(line);
        }
        Ok(())
    }
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct TuiDisplay<D: DeparturesApi> {
    api_client: D,
    stops: InputStops,
    log_buffer: LogBuffer,
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

        Self::render(&display_lines, &self.log_buffer, &mut terminal)?;

        // Wait for user to press 'q' to quit. Timeout every 250ms to keep responsive (no refresh behavior implemented).
        loop {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('l') => {
                        info!("This is a sample log.");
                        Self::render(&display_lines, &self.log_buffer, &mut terminal)?;
                    }
                    KeyCode::Char('r') => {
                        // Refresh: re-fetch departures and re-render
                        let resp = self.api_client.get_departures(&self.stops).await?;
                        display_lines = crate::view::build_display_lines(&resp);
                        Self::render(&display_lines, &self.log_buffer, &mut terminal)?;
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    // Re-render using the current terminal size
                    Self::render(&display_lines, &self.log_buffer, &mut terminal)?;
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
        log_buffer: &LogBuffer,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), anyhow::Error> {
        // Render once and wait for 'q' to quit
        terminal.draw(|f| {
            let size = f.size();
            let log_height = if size.height > 10 { 5 } else { 3 };
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(log_height)].as_ref())
                .split(size);

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

            let (line_width, dir_width) = max_column_widths(display_lines);
            let line_width = line_width.max(5);
            let dir_width = dir_width.max(1);

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
                            format!("{:<width$}", e.line, width = line_width),
                            Style::default().bg(tui_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!(
                            "| {:<width$} | {:>5} | {:2}min{}",
                            e.dir,
                            abs_text,
                            e.actual_mins,
                            delay_text,
                            width = dir_width
                        )),
                    ];

                    spans.push(Spans::from(span_vec));
                }
                spans.push(Spans::from(Span::raw("")));
            }

            let paragraph = Paragraph::new(Text::from(spans))
                .block(Block::default().borders(Borders::ALL).title("Departures"))
                .alignment(Alignment::Left);

            let log_lines = log_buffer.snapshot();
            let log_inner_height = chunks[1].height.saturating_sub(2) as usize;
            let log_inner_width = chunks[1].width.saturating_sub(2) as usize;
            let mut visible_logs: Vec<String> = if log_lines.is_empty() {
                vec!["No logs yet".to_string()]
            } else if log_inner_height == 0 {
                Vec::new()
            } else {
                log_lines
                    .into_iter()
                    .rev()
                    .take(log_inner_height)
                    .collect::<Vec<String>>()
                    .into_iter()
                    .rev()
                    .collect()
            };
            for line in &mut visible_logs {
                *line = truncate_line(line, log_inner_width);
            }
            let log_spans: Vec<Spans> = visible_logs
                .into_iter()
                .map(|line| Spans::from(Span::raw(line)))
                .collect();
            let log_paragraph = Paragraph::new(Text::from(log_spans))
                .block(Block::default().borders(Borders::ALL).title("Logs"))
                .alignment(Alignment::Left);

            f.render_widget(paragraph, chunks[0]);
            f.render_widget(log_paragraph, chunks[1]);
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

fn truncate_line(line: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if line.width() <= max_width {
        return line.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for ch in line.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out
}

fn max_column_widths(display_lines: &Vec<(String, Vec<DisplayEntry>)>) -> (usize, usize) {
    let mut max_line = 0usize;
    let mut max_dir = 0usize;
    for (_, entries) in display_lines {
        for entry in entries {
            max_line = max_line.max(UnicodeWidthStr::width(entry.line.as_str()));
            max_dir = max_dir.max(UnicodeWidthStr::width(entry.dir.as_str()));
        }
    }
    (max_line, max_dir)
}
