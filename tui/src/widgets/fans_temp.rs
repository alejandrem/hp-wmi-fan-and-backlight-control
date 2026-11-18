// ─── Fans & Temperatures Widget ─────────────────────────────────────────
// Combined fans, temperatures, and power readings.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::layout::Position;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph};
use ratatui::Frame;

use crate::sensors::{smoothing, Sensors};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FanControlButton {
    Auto,
    Max,
    RpmDown,
    RpmInput,
    RpmUp,
    ApplyRpm,
}

pub fn render(
    frame: &mut Frame,
    sensors: &Sensors,
    area: Rect,
    rpm_text: &str,
    rpm_editing: bool,
    message: &str,
) {
    let block = build_fans_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = build_fans_rows(inner);
    render_controls(frame, rows.first().copied(), rpm_text, rpm_editing);
    render_control_hint(frame, rows.get(1).copied(), message);
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
    // Keep controls, fan readings, and the separator compact at the top;
    // divide all remaining panel height among the six temperature graphs.
    let fixed_rows = 5usize;
    let temperature_rows = 6usize;
    let available = inner.height.saturating_sub(fixed_rows as u16) as usize;
    let graph_height = (available / temperature_rows).max(1) as u16;
    let mut constraints = vec![Constraint::Length(1); fixed_rows];
    constraints.extend(std::iter::repeat(Constraint::Length(graph_height)).take(temperature_rows));
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner)
        .to_vec()
}

fn control_button_areas(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(6), Constraint::Length(1),
            Constraint::Length(5), Constraint::Length(1),
            Constraint::Length(3), Constraint::Length(1),
            Constraint::Length(11), Constraint::Length(1),
            Constraint::Length(3), Constraint::Length(1),
            Constraint::Length(5),
        ])
        .split(area)
        .to_vec()
}

pub fn hit_test_control(area: Rect, x: u16, y: u16) -> Option<FanControlButton> {
    let block = build_fans_block();
    let rows = build_fans_rows(block.inner(area));
    let control_row = rows.first().copied()?;
    let buttons = control_button_areas(control_row);
    let index = buttons.iter().enumerate().find_map(|(index, button)| {
        let point = Position::new(x, y);
        button.contains(point).then_some(index)
    })?;
    match index {
        0 => Some(FanControlButton::Auto),
        2 => Some(FanControlButton::Max),
        4 => Some(FanControlButton::RpmDown),
        6 => Some(FanControlButton::RpmInput),
        8 => Some(FanControlButton::RpmUp),
        10 => Some(FanControlButton::ApplyRpm),
        _ => None,
    }
}

fn render_controls(frame: &mut Frame, area: Option<Rect>, rpm_text: &str, rpm_editing: bool) {
    let Some(area) = area else { return };
    let parts = control_button_areas(area);
    let labels = ["[AUTO]", "[MAX]", "[-]", "[RPM]", "[+]", "[SET]"];
    let indices = [0, 2, 4, 6, 8, 10];
    for (button, index) in indices.into_iter().enumerate() {
        let Some(rect) = parts.get(index).copied() else { continue };
        let (label, color) = match button {
            0 => (labels[button].to_string(), Color::Green),
            1 => (labels[button].to_string(), Color::Red),
            2 => (labels[button].to_string(), Color::Cyan),
            3 => (format!("[RPM:{}]", rpm_text), if rpm_editing { Color::Yellow } else { Color::Cyan }),
            4 => (labels[button].to_string(), Color::Cyan),
            _ => (labels[button].to_string(), Color::Yellow),
        };
        let style = Style::default().fg(Color::Black).bg(color).add_modifier(Modifier::BOLD);
        frame.render_widget(Paragraph::new(label).alignment(Alignment::Center).style(style), rect);
    }
}

fn render_control_hint(frame: &mut Frame, area: Option<Rect>, message: &str) {
    let Some(area) = area else { return };
    let text = if message.is_empty() {
        "F1 Auto  F2 Max  F3 edit RPM  Enter apply".to_string()
    } else {
        message.to_string()
    };
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(if message.is_empty() { Color::DarkGray } else { Color::Cyan }),
    ));
    frame.render_widget(widget, area);
}

fn render_fans(frame: &mut Frame, rows: &[Rect], sensors: &Sensors) {
    draw_fan_row(frame, rows, 2, "Fan 1", sensors.fan1_rpm, 4300.0);
    draw_fan_row(frame, rows, 3, "Fan 2", sensors.fan2_rpm, 4100.0);
}

fn render_separator(frame: &mut Frame, rows: &[Rect]) {
    if let Some(sep) = rows.get(4) {
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
        if let Some(row) = rows.get(5 + i) {
            draw_temp_row(frame, *row, label, *temp, *power, i);
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
    temp: Option<f64>, power: Option<f64>, history_index: usize,
) {
    let ratio = calc_temp_ratio(temp);
    let parts = split_temp_layout(area);
    render_temp_label(frame, parts[0], label);
    render_temp_gauge(frame, parts[1], ratio, history_index);
    render_temp_value(frame, parts[2], temp);
    render_temp_power(frame, parts[3], power);
}

fn calc_temp_ratio(temp: Option<f64>) -> f64 {
    temp.map(|t| (t / 100.0).clamp(0.0, 1.0)).unwrap_or(0.0)
}

fn split_temp_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(9), Constraint::Min(1),
            Constraint::Length(5), Constraint::Length(7),
        ])
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

fn render_temp_gauge(frame: &mut Frame, area: Rect, ratio: f64, history_index: usize) {
    if area.is_empty() {
        return;
    }
    let color = temp_color(ratio);
    let history = smoothing::temperature_history(history_index);
    let points = super::graph::vertical_dot_history(&history, area.width);
    if points.is_empty() {
        return;
    }
    let graph = Canvas::default()
        .x_bounds([0.0, area.width as f64])
        .y_bounds([0.0, 4.0])
        .marker(Marker::Braille)
        .paint(|context| context.draw(&Points::new(&points, color)));
    frame.render_widget(graph, area);
}

fn render_temp_value(frame: &mut Frame, area: Rect, temp: Option<f64>) {
    let value = temp.map(|t| format!("{:>3.0} C", t)).unwrap_or_else(|| " N/A".into());
    let widget = Paragraph::new(Span::styled(
        value,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
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
