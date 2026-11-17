// ─── Top Left Widget ────────────────────────────────────────────────────
// Large GPU sparkline, GPUB/GPU1 labels, and uptime.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Sparkline};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_top_left_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = build_top_left_rows(inner);
    render_title(frame, rows[0]);
    draw_gpu_sparkline(frame, sensors, rows[1]);
    render_separator(frame, rows[2]);
    render_gpu_labels(frame, sensors, rows[3]);
    render_uptime(frame, sensors, rows[4]);
}

fn build_top_left_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray))
}

fn build_top_left_rows(inner: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(inner)
        .to_vec()
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Span::styled(
        "  total-gpu-totals",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Left);
    frame.render_widget(title, area);
}

fn render_separator(frame: &mut Frame, area: Rect) {
    let sep = Paragraph::new(Span::styled(
        "------------------------------------------------------------------------",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(sep, area);
}

fn render_gpu_labels(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    draw_gpu_label(frame, parts[0], "GPUB", sensors.igpu_usage);
    draw_gpu_label(frame, parts[1], "GPU1", sensors.gpu_usage);
}

fn render_uptime(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let uptime_text = format!("up {}", sensors.uptime);
    let uptime = Paragraph::new(Span::styled(
        uptime_text,
        Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
    )).alignment(Alignment::Left);
    frame.render_widget(uptime, area);
}

fn draw_gpu_sparkline(frame: &mut Frame, _sensors: &Sensors, area: Rect) {
    let combined = combine_gpu_history();
    if combined.is_empty() {
        return;
    }
    let sparkline = Sparkline::default()
        .data(&combined)
        .style(Color::Green);
    frame.render_widget(sparkline, area);
}

fn combine_gpu_history() -> Vec<u64> {
    use crate::sensors::smoothing;
    let cpu_hist = to_u64(smoothing::cpu_usage_history());
    let igpu_hist = to_u64(smoothing::igpu_usage_history());
    let gpu_hist = to_u64(smoothing::gpu_usage_history());
    let max_len = cpu_hist.len().max(igpu_hist.len()).max(gpu_hist.len());
    (0..max_len).map(|i| {
        let sum = cpu_hist.get(i).copied().unwrap_or(0)
            + igpu_hist.get(i).copied().unwrap_or(0)
            + gpu_hist.get(i).copied().unwrap_or(0);
        sum / 3
    }).collect()
}

fn draw_gpu_label(frame: &mut Frame, area: Rect, label: &str, usage: Option<f64>) {
    let text = match usage {
        Some(u) => format!("{}: {:.0}%", label, u),
        None => format!("{}: N/A", label),
    };
    let color = match usage {
        Some(u) if u > 80.0 => Color::Red,
        Some(u) if u > 60.0 => Color::Yellow,
        Some(_) => Color::Green,
        None => Color::DarkGray,
    };
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn to_u64(data: Vec<f32>) -> Vec<u64> {
    data.iter().map(|v| (*v * 10.0) as u64).collect()
}
