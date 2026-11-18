// ─── Memory Widget ──────────────────────────────────────────────────────
// Btop-style memory values with a small vertical dot meter per category.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::sensors::{smoothing, MemoryInfo, Sensors};

type MemoryItem<'a> = (&'a str, f64, Color);

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_mem_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let memory = &sensors.memory;
    if memory.total_gib <= 0.0 || inner.height == 0 {
        return;
    }
    let items = build_mem_items(memory);
    render_memory_list(frame, inner, &items, memory.total_gib);
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

fn build_mem_items(memory: &MemoryInfo) -> [MemoryItem<'_>; 5] {
    [
        ("Total", memory.total_gib, Color::White),
        ("Used", memory.used_gib, Color::Green),
        ("Available", memory.available_gib, Color::Cyan),
        ("Cached", memory.cached_gib, Color::Yellow),
        ("Free", memory.free_gib, Color::White),
    ]
}

fn render_memory_list(
    frame: &mut Frame,
    area: Rect,
    items: &[MemoryItem<'_>],
    total: f64,
) {
    let section_height = area.height.saturating_sub(1) as usize / 4;
    if section_height == 0 {
        return;
    }
    let mut constraints = vec![Constraint::Length(1)];
    constraints.extend(std::iter::repeat(Constraint::Length(section_height as u16)).take(4));
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    if let Some(total_area) = sections.first() {
        draw_total(frame, *total_area, items[0]);
    }
    for (index, item) in items.iter().enumerate().skip(1) {
        if let Some(section) = sections.get(index) {
            draw_memory_item(frame, *section, *item, total, index - 1);
        }
    }
}

fn draw_total(frame: &mut Frame, area: Rect, item: MemoryItem<'_>) {
    let value = format!("{:.1} GiB", item.1);
    let line = Line::from(vec![
        Span::styled(
            format!("{}: ", item.0),
            Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, Style::default().fg(item.2).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_memory_item(frame: &mut Frame, area: Rect, item: MemoryItem<'_>, total: f64, history_index: usize) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    let percent = (item.1 / total * 100.0).clamp(0.0, 100.0);
    let percent_text = format!("{:>3.0}%", percent);
    let value_text = format!("{:.2} GiB", item.1);
    let value = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}: ", item.0),
                Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
            ),
            Span::styled(value_text, Style::default().fg(item.2).add_modifier(Modifier::BOLD)),
        ]));
    frame.render_widget(value, rows[0]);

    let chart_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(rows[1]);
    let percent = Paragraph::new(Span::styled(
        percent_text,
        Style::default().fg(item.2).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(percent, chart_row[0]);
    draw_memory_history(frame, chart_row[1], history_index, item.2);
}

fn draw_memory_history(frame: &mut Frame, area: Rect, history_index: usize, color: Color) {
    if area.is_empty() {
        return;
    }
    let history = smoothing::memory_history(history_index);
    if history.is_empty() {
        return;
    }
    let points = super::graph::vertical_dot_history(&history, area.width);
    let meter = Canvas::default()
        .x_bounds([0.0, area.width as f64])
        .y_bounds([0.0, 4.0])
        .marker(Marker::Braille)
        .paint(|context| context.draw(&Points::new(&points, color)));
    frame.render_widget(meter, area);
}
