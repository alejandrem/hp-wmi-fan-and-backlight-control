// ─── Memory Widget ──────────────────────────────────────────────────────
// Shows RAM usage: total, used, available, cached, free with bars.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_mem_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let m = &sensors.memory;
    let total = m.total_gib;
    if total <= 0.0 {
        return;
    }
    let items = build_mem_items(m);
    let rows = build_mem_rows(inner, items.len());
    render_mem_list(frame, &rows, &items, total);
}

fn build_mem_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  mem",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn build_mem_items(m: &crate::sensors::MemoryInfo) -> [(&str, f64, Color); 5] {
    [
        ("Total",   m.total_gib,   Color::White),
        ("Used",    m.used_gib,    Color::Red),
        ("Avail",   m.available_gib, Color::Green),
        ("Cache",   m.cached_gib,  Color::Yellow),
        ("Free",    m.free_gib,    Color::Cyan),
    ]
}

fn build_mem_rows(inner: Rect, count: usize) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Length(1))
                .take(count)
                .collect::<Vec<_>>(),
        )
        .split(inner)
        .to_vec()
}

fn render_mem_list(
    frame: &mut Frame, rows: &[Rect],
    items: &[(&str, f64, Color)], total: f64,
) {
    for (i, (label, value, color)) in items.iter().enumerate() {
        if let Some(row) = rows.get(i) {
            draw_mem_row(frame, *row, label, *value, total, *color);
        }
    }
}

fn draw_mem_row(
    frame: &mut Frame, area: Rect, label: &str,
    value: f64, total: f64, color: Color,
) {
    let ratio = (value / total).clamp(0.0, 1.0);
    let parts = split_mem_layout(area);
    render_mem_label(frame, parts[0], label, value, total, color);
    render_mem_gauge(frame, parts[1], ratio, color);
}

fn split_mem_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Min(1)])
        .split(area)
        .to_vec()
}

fn render_mem_label(
    frame: &mut Frame, area: Rect,
    label: &str, value: f64, total: f64, color: Color,
) {
    let pct = (value / total * 100.0).round() as u64;
    let text = format!("{:>5} {:5.1}G {:>3}%", label, value, pct);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn render_mem_gauge(frame: &mut Frame, area: Rect, ratio: f64, color: Color) {
    let gauge = LineGauge::default()
        .ratio(ratio)
        .filled_symbol("█")
        .unfilled_symbol("░")
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(gauge, area);
}
