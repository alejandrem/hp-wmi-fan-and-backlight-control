// ─── Top Left Widget ────────────────────────────────────────────────────
// CPU history graph, GPU labels, and uptime.
// Activity history is drawn with Braille dot markers for a btop-like graph.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine, Points};
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_top_left_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = build_top_left_rows(inner);
    draw_combined_sparkline(frame, sensors, rows[0]);
    render_separator(frame, rows[1]);
    render_gpu_labels(frame, sensors, rows[2]);
    render_uptime(frame, sensors, rows[3]);
}

fn build_top_left_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " cpu ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
}

fn build_top_left_rows(inner: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(inner)
        .to_vec()
}

fn render_separator(frame: &mut Frame, area: Rect) {
    let sep_str = build_separator_str(area.width);
    let sep = Paragraph::new(Span::styled(
        sep_str,
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(sep, area);
}

fn build_separator_str(width: u16) -> String {
    let len = width as usize;
    "-".repeat(len)
}

fn render_gpu_labels(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let rows = gpu_label_rows(parts[0]);
    draw_gpu_label_with_bar(
        frame, rows[0], "GPUB", sensors.igpu_usage,
    );
    draw_gpu_bar(frame, rows[1], sensors.igpu_usage);
    let rows = gpu_label_rows(parts[1]);
    draw_gpu_label_with_bar(
        frame, rows[0], "GPU1", sensors.gpu_usage,
    );
    draw_gpu_bar(frame, rows[1], sensors.gpu_usage);
}

fn gpu_label_rows(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area)
        .to_vec()
}

fn render_uptime(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let uptime_text = format!("up {}", sensors.uptime);
    let uptime = Paragraph::new(Span::styled(
        uptime_text,
        Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Left);
    frame.render_widget(uptime, area);
}

fn draw_combined_sparkline(
    frame: &mut Frame, _sensors: &Sensors, area: Rect,
) {
    let history = build_cpu_history();
    if history.is_empty() {
        return;
    }
    let values = super::graph::resample_history(&history, area.width);
    let points: Vec<(f64, f64)> = values.iter().enumerate()
        .map(|(i, value)| (i as f64, *value))
        .collect();
    let x_max = points.len().saturating_sub(1).max(1) as f64;
    // Laptop CPU/GPU usage is often far below 100%; a fixed 0..100 scale
    // compresses normal activity into an almost invisible baseline.
    let peak = history.iter().copied().max().unwrap_or(0).min(100) as f64;
    let y_max = (((peak * 1.15 / 10.0).ceil() * 10.0).max(10.0)).min(100.0);
    let graph = Canvas::default()
        .x_bounds([0.0, x_max])
        .y_bounds([0.0, y_max])
        .marker(Marker::Braille)
        .paint(|ctx| {
            for &(x, value) in &points {
                ctx.draw(&CanvasLine::new(
                    x, 0.0, x, value, Color::Cyan,
                ));
            }
            ctx.draw(&Points::new(&points, Color::Cyan));
        });
    frame.render_widget(graph, area);
}

fn build_cpu_history() -> Vec<u64> {
    use crate::sensors::smoothing;
    smoothing::cpu_usage_history()
        .into_iter()
        .map(|value| value.max(0.0) as u64)
        .collect()
}

fn draw_gpu_label_with_bar(
    frame: &mut Frame, area: Rect, label: &str, usage: Option<f64>,
) {
    let text = gpu_label_text(label, usage);
    let color = gpu_label_color(usage);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_gpu_bar(frame: &mut Frame, area: Rect, usage: Option<f64>) {
    let pct = usage.unwrap_or(0.0) as f64;
    let bar = LineGauge::default()
        .filled_symbol("█")
        .unfilled_symbol("░")
        .ratio(pct / 100.0)
        .style(gpu_label_color(usage));
    frame.render_widget(bar, area);
}

fn gpu_label_text(label: &str, usage: Option<f64>) -> String {
    match usage {
        Some(u) => format!("{}: {:.0}%", label, u),
        None => format!("{}: N/A", label),
    }
}

fn gpu_label_color(usage: Option<f64>) -> Color {
    match usage {
        Some(u) if u > 80.0 => Color::Red,
        Some(u) if u > 60.0 => Color::Yellow,
        Some(_) => Color::Green,
        None => Color::DarkGray,
    }
}

