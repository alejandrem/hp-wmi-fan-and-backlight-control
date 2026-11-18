// ─── Disk Widget ────────────────────────────────────────────────────────
// Shows disk partitions with usage bars.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::sensors::{smoothing, Sensors};

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_disk_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let disks = &sensors.disks;
    if disks.is_empty() {
        return;
    }
    let rows = build_disk_rows(inner, disks.len());
    render_disk_list(frame, &rows, disks);
}

fn build_disk_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  disks",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn build_disk_rows(inner: Rect, disk_count: usize) -> Vec<Rect> {
    let rows_count = disk_count * 3;
    let row_h = calc_disk_row_height(inner.height, rows_count);
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Length(row_h))
                .take(rows_count)
                .collect::<Vec<_>>(),
        )
        .split(inner)
        .to_vec()
}

fn calc_disk_row_height(height: u16, rows_count: usize) -> u16 {
    if height as usize > rows_count {
        1
    } else {
        (height as usize / rows_count).max(1) as u16
    }
}

fn render_disk_list(
    frame: &mut Frame, rows: &[Rect], disks: &[crate::sensors::DiskInfo],
) {
    for (i, disk) in disks.iter().enumerate() {
        let idx = i * 3;
        if let Some(row) = rows.get(idx) {
            draw_disk_label(frame, *row, disk);
        }
        if let (Some(first), Some(second)) = (rows.get(idx + 1), rows.get(idx + 2)) {
            let chart_area = Rect::new(first.x, first.y, first.width, first.height.saturating_add(second.height));
            draw_disk_history(frame, chart_area, disk);
        }
    }
}

fn draw_disk_label(frame: &mut Frame, area: Rect, disk: &crate::sensors::DiskInfo) {
    let mount = shorten_mount(&disk.mount);
    let text = format!("{:<12} {:6.1} GiB", mount, disk.total_gib);
    let widget = Paragraph::new(Span::styled(
        text,
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(widget, area);
}

fn draw_disk_history(frame: &mut Frame, area: Rect, disk: &crate::sensors::DiskInfo) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(area);
    let color = disk_usage_color(disk.usage_pct);
    let value = Paragraph::new(Span::styled(
        format!("{:>3.0}%", disk.usage_pct),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(value, columns[0]);

    let history = smoothing::disk_history(&disk.mount);
    let points = super::graph::vertical_dot_history(&history, columns[1].width);
    if columns[1].is_empty() || points.is_empty() {
        return;
    }
    let chart = Canvas::default()
        .x_bounds([0.0, columns[1].width as f64])
        .y_bounds([0.0, 4.0])
        .marker(Marker::Braille)
        .paint(|context| context.draw(&Points::new(&points, color)));
    frame.render_widget(chart, columns[1]);
}

fn disk_usage_color(usage_pct: f64) -> Color {
    if usage_pct > 90.0 {
        Color::Red
    } else if usage_pct > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn shorten_mount(mount: &str) -> String {
    // Shorten long mount paths
    if mount.len() > 12 {
        format!("...{}", &mount[mount.len() - 9..])
    } else {
        mount.to_string()
    }
}
