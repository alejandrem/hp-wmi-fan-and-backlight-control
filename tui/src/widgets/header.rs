// ─── Header Widget ──────────────────────────────────────────────────────
// Shows time, battery percentage, time remaining, and total power draw.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_header_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let parts = split_header(inner);
    render_time(frame, parts[0]);
    render_battery(frame, parts[1], sensors);
}

fn build_header_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Reset))
        .title(Span::styled(
            "  ALETOP  ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
}

fn split_header(inner: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner)
        .to_vec()
}

fn render_time(frame: &mut Frame, area: Rect) {
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let widget = Paragraph::new(Span::styled(
        time,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Left);
    frame.render_widget(widget, area);
}

fn render_battery(frame: &mut Frame, area: Rect, sensors: &Sensors) {
    let bat_text = build_battery_text(sensors);
    let widget = Paragraph::new(Span::styled(
        bat_text,
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Right);
    frame.render_widget(widget, area);
}

fn build_battery_text(s: &Sensors) -> String {
    let pct = s.battery_pct.map(|p| format!("{}%", p)).unwrap_or_else(|| "N/A".into());
    let time = s.battery_time_remaining
        .as_deref()
        .unwrap_or("N/A");
    let power = s.total_power
        .map(|p| format!("{:.2}W", p))
        .unwrap_or_else(|| "N/A".into());
    format!("BAT {}  {}  {}", pct, time, power)
}
