// ─── Disk Widget ────────────────────────────────────────────────────────
// Shows disk partitions with usage bars.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, LineGauge, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

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
    let rows_count = disk_count * 2;
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
        let idx = i * 2;
        if let Some(row) = rows.get(idx) {
            draw_disk_label(frame, *row, disk);
        }
        if let Some(row) = rows.get(idx + 1) {
            draw_disk_bar(frame, *row, disk);
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

fn draw_disk_bar(frame: &mut Frame, area: Rect, disk: &crate::sensors::DiskInfo) {
    let ratio = (disk.usage_pct / 100.0).clamp(0.0, 1.0);
    let color = if disk.usage_pct > 90.0 {
        Color::Red
    } else if disk.usage_pct > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    let gauge = LineGauge::default()
        .ratio(ratio)
        .filled_symbol("█")
        .unfilled_symbol("░")
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(gauge, area);
}

fn shorten_mount(mount: &str) -> String {
    // Shorten long mount paths
    if mount.len() > 12 {
        format!("...{}", &mount[mount.len() - 9..])
    } else {
        mount.to_string()
    }
}
