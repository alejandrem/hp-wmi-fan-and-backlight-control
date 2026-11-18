// ─── Top Right Widget ───────────────────────────────────────────────────
// CPU model, temp, usage, per-core bars (C0-C11), load avg, GPU usage.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::sensors::{smoothing, Sensors};

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // CPU header
            Constraint::Length(1),  // CPU gauge
            Constraint::Min(1),     // per-core bars
            Constraint::Length(1),  // load avg
            Constraint::Length(1),  // GPUB
            Constraint::Length(1),  // GPU1
        ])
        .split(inner);

    draw_cpu_header(frame, sensors, rows[0]);
    draw_cpu_gauge(frame, sensors, rows[1]);
    draw_per_core_bars(frame, sensors, rows[2]);
    draw_load_avg(frame, sensors, rows[3]);
    draw_gpu_bar(frame, rows[4], "GPU0", sensors.igpu_usage);
    draw_gpu_bar(frame, rows[5], "GPU1", sensors.gpu_usage);
}

fn draw_cpu_header(frame: &mut Frame, s: &Sensors, area: Rect) {
    let temp = s.cpu_temp.map(|t| format!("{:.0} C", t)).unwrap_or_else(|| "N/A".into());
    let usage = format!("{:.0}%", s.cpu_usage);
    let text = Line::from(vec![
        Span::styled(
            format!("{}   ", s.cpu_model),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            usage,
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            temp,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]);
    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

fn draw_cpu_gauge(frame: &mut Frame, s: &Sensors, area: Rect) {
    draw_usage_graph(frame, area, &format!("CPU {:.0}%", s.cpu_usage), "cpu", s.cpu_usage);
}

fn draw_per_core_bars(frame: &mut Frame, s: &Sensors, area: Rect) {
    let cores = &s.per_core_usage;
    if cores.is_empty() {
        return;
    }
    let (left_cores, right_cores) = split_cores(cores);
    let (left_rows, right_rows) = build_core_rows(area, &left_cores, &right_cores);
    render_core_column(frame, &left_cores, &left_rows);
    render_core_column(frame, &right_cores, &right_rows);
}

fn split_cores(cores: &[f64]) -> (Vec<(usize, f64)>, Vec<(usize, f64)>) {
    let half = (cores.len() + 1) / 2;
    let left: Vec<_> = cores.iter().take(half).enumerate()
        .map(|(i, v)| (i, *v)).collect();
    let right: Vec<_> = cores.iter().skip(half).enumerate()
        .map(|(i, v)| (i + half, *v)).collect();
    (left, right)
}

fn build_core_rows(
    area: Rect, left: &[(usize, f64)], right: &[(usize, f64)],
) -> (Vec<Rect>, Vec<Rect>) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let rows_needed = left.len().max(right.len()).max(1);
    let row_h = (area.height as usize / rows_needed).max(1) as u16;
    let constraints: Vec<_> = std::iter::repeat(Constraint::Length(row_h))
        .take(rows_needed)
        .collect();
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.clone())
        .split(cols[0])
        .to_vec();
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(cols[1])
        .to_vec();
    (left_rows, right_rows)
}

fn render_core_column(
    frame: &mut Frame, cores: &[(usize, f64)], rows: &[Rect],
) {
    for (i, (idx, usage)) in cores.iter().enumerate() {
        if let Some(row) = rows.get(i) {
            draw_core_bar(frame, *row, *idx, *usage);
        }
    }
}

fn draw_core_bar(frame: &mut Frame, area: Rect, core_idx: usize, usage: f64) {
    draw_usage_graph(
        frame,
        area,
        &format!("C{:2} {:.0}%", core_idx, usage),
        &format!("core:{core_idx}"),
        usage,
    );
}

fn draw_load_avg(frame: &mut Frame, s: &Sensors, area: Rect) {
    let text = format!(
        "Load avg: {:.2}  {:.2}  {:.2}",
        s.load_avg[0], s.load_avg[1], s.load_avg[2]
    );
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_gpu_bar(frame: &mut Frame, area: Rect, label: &str, usage: Option<f64>) {
    let pct_text = usage.map(|value| format!("{value:.0}%")).unwrap_or_else(|| "N/A".into());
    let value = usage.unwrap_or(0.0);
    let key = if label == "GPU0" { "gpu0" } else { "gpu1" };
    draw_usage_graph(frame, area, &format!("{label} {pct_text}"), key, value);
}

fn draw_usage_graph(frame: &mut Frame, area: Rect, label: &str, key: &str, usage: f64) {
    let label_width = (label.chars().count() as u16 + 1).min(area.width);
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(label_width), Constraint::Min(1)])
        .split(area);
    let label_widget = Paragraph::new(Span::styled(
        label.to_string(),
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(label_widget, parts[0]);

    let history = smoothing::usage_history(key);
    let points = super::graph::vertical_dot_history(&history, parts[1].width);
    if parts[1].is_empty() || points.is_empty() {
        return;
    }
    let chart = Canvas::default()
        .x_bounds([0.0, parts[1].width as f64])
        .y_bounds([0.0, 4.0])
        .marker(Marker::Braille)
        .paint(|context| context.draw(&Points::new(&points, usage_color(usage))));
    frame.render_widget(chart, parts[1]);
}

fn usage_color(usage: f64) -> Color {
    if usage > 80.0 { Color::Red }
    else if usage > 60.0 { Color::Yellow }
    else { Color::Green }
}
