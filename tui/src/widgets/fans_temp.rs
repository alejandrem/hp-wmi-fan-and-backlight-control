// ─── Fans & Temperatures Widget ─────────────────────────────────────────
// Combined fans, temperatures, and power readings.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_fans_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = build_fans_rows(inner);
    render_fans(frame, &rows, sensors);
    render_separator(frame, &rows);
    render_temps(frame, &rows, sensors);
}

fn build_fans_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  fans & temperatures",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn build_fans_rows(inner: Rect) -> Vec<Rect> {
    let total_rows = 9;
    let row_h = calc_row_height(inner.height, total_rows);
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Length(row_h))
                .take(total_rows)
                .collect::<Vec<_>>(),
        )
        .split(inner)
        .to_vec()
}

fn calc_row_height(height: u16, total_rows: usize) -> u16 {
    if height as usize >= total_rows {
        1
    } else {
        (height as usize / total_rows).max(1) as u16
    }
}

fn render_fans(frame: &mut Frame, rows: &[Rect], sensors: &Sensors) {
    draw_fan_row(frame, rows, 0, "Fan 1", sensors.fan1_rpm, 4300.0);
    draw_fan_row(frame, rows, 1, "Fan 2", sensors.fan2_rpm, 4100.0);
}

fn render_separator(frame: &mut Frame, rows: &[Rect]) {
    if let Some(sep) = rows.get(2) {
        let widget = Paragraph::new(Span::styled(
            "--------------------------------------------------------",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(widget, *sep);
    }
}

fn render_temps(frame: &mut Frame, rows: &[Rect], sensors: &Sensors) {
    let temps = build_temp_list(sensors);
    for (i, (label, temp, power)) in temps.iter().enumerate() {
        if let Some(row) = rows.get(3 + i) {
            draw_temp_row(frame, *row, label, *temp, *power);
        }
    }
}

fn build_temp_list(s: &Sensors) -> [(&str, Option<f64>, Option<f64>); 6] {
    [
        ("CPU",     s.cpu_temp,     s.cpu_power),
        ("iGPU",    s.igpu_temp,    s.igpu_power),
        ("GPU",     s.gpu_temp,     s.gpu_power),
        ("NVMe",    s.nvme_temp,    None),
        ("ACPI",    s.acpi_temp,    None),
        ("Chipset", s.chipset_temp, None),
    ]
}

fn draw_fan_row(
    frame: &mut Frame, rows: &[Rect], idx: usize,
    label: &str, rpm: Option<i64>, max: f64,
) {
    let Some(row) = rows.get(idx) else { return };
    let (text, ratio) = build_fan_data(label, rpm, max);
    let parts = split_fan_layout(*row);
    render_fan_label(frame, parts[0], &text);
    render_fan_gauge(frame, parts[1], ratio);
}

fn build_fan_data(
    label: &str, rpm: Option<i64>, max: f64,
) -> (String, f64) {
    let rpm_text = rpm.map(|r| format!("{} RPM", r))
        .unwrap_or_else(|| "N/A".into());
    let ratio = rpm.map(|r| (r as f64 / max).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    (format!("{}: {}", label, rpm_text), ratio)
}

fn split_fan_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(1)])
        .split(area)
        .to_vec()
}

fn render_fan_label(frame: &mut Frame, area: Rect, text: &str) {
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn render_fan_gauge(frame: &mut Frame, area: Rect, ratio: f64) {
    let gauge = LineGauge::default()
        .ratio(ratio)
        .filled_symbol("=")
        .unfilled_symbol("-")
        .filled_style(Style::default().fg(Color::Cyan))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(gauge, area);
}

fn draw_temp_row(
    frame: &mut Frame, area: Rect, label: &str,
    temp: Option<f64>, power: Option<f64>,
) {
    let ratio = calc_temp_ratio(temp);
    let parts = split_temp_layout(area);
    render_temp_label(frame, parts[0], label);
    render_temp_gauge(frame, parts[1], ratio, temp);
    render_temp_power(frame, parts[2], power);
}

fn calc_temp_ratio(temp: Option<f64>) -> f64 {
    temp.map(|t| (t / 100.0).clamp(0.0, 1.0)).unwrap_or(0.0)
}

fn split_temp_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(9), Constraint::Min(1), Constraint::Length(7)])
        .split(area)
        .to_vec()
}

fn render_temp_label(frame: &mut Frame, area: Rect, label: &str) {
    let widget = Paragraph::new(Span::styled(
        format!("{}:    ", label),
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn render_temp_gauge(frame: &mut Frame, area: Rect, ratio: f64, temp: Option<f64>) {
    let temp_text = temp.map(|t| format!("{:.0} C", t))
        .unwrap_or_else(|| "N/A".into());
    let color = temp_color(ratio);
    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(Span::styled(
            temp_text,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ))
        .filled_symbol("-")
        .unfilled_symbol(".")
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(gauge, area);
}

fn render_temp_power(frame: &mut Frame, area: Rect, power: Option<f64>) {
    let power_text = power.map(|p| format!("{:.1}W", p))
        .unwrap_or_else(|| "   ".into());
    let widget = Paragraph::new(Span::styled(
        power_text,
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn temp_color(ratio: f64) -> Color {
    if ratio > 0.8 { Color::Red }
    else if ratio > 0.6 { Color::Yellow }
    else { Color::Green }
}
