// ─── Process Widget ─────────────────────────────────────────────────────
// Shows top processes table: Pid, Program, User, Mem%, Cpu%.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::sensors::Sensors;

pub fn render(frame: &mut Frame, sensors: &Sensors, area: Rect) {
    let block = build_proc_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let procs = &sensors.processes;
    if procs.is_empty() {
        return;
    }
    let rows = build_proc_rows(inner, procs.len());
    render_proc_header(frame, &rows);
    render_proc_list(frame, &rows, procs);
}

fn build_proc_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            "  proc",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
}

fn build_proc_rows(inner: Rect, proc_count: usize) -> Vec<Rect> {
    let total_rows = 1 + proc_count;
    let row_h = calc_proc_row_height(inner.height, total_rows);
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

fn calc_proc_row_height(height: u16, total_rows: usize) -> u16 {
    if height as usize > total_rows {
        1
    } else {
        (height as usize / total_rows.max(1)).max(1) as u16
    }
}

fn render_proc_header(frame: &mut Frame, rows: &[Rect]) {
    if let Some(row) = rows.get(0) {
        draw_header(frame, *row);
    }
}

fn render_proc_list(frame: &mut Frame, rows: &[Rect], procs: &[crate::sensors::ProcessInfo]) {
    for (i, proc) in procs.iter().enumerate() {
        if let Some(row) = rows.get(i + 1) {
            draw_proc_row(frame, *row, proc);
        }
    }
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let text = Line::from(vec![
        Span::styled(" Pid:   ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
        Span::styled("Program:          ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
        Span::styled("User:   ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
        Span::styled("Mem%   ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
        Span::styled("Cpu%", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
    ]);
    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

fn draw_proc_row(frame: &mut Frame, area: Rect, p: &crate::sensors::ProcessInfo) {
    let text = build_proc_line(p);
    let widget = Paragraph::new(text);
    frame.render_widget(widget, area);
}

fn build_proc_line(p: &crate::sensors::ProcessInfo) -> Line<'_> {
    let mem_str = format_mem(p.mem_mb);
    Line::from(vec![
        Span::styled(format!("{:<6}", p.pid), Style::default().fg(Color::White)),
        Span::styled(
            format!("{:<16}", truncate(&p.name, 16)),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:<7}", p.user),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(
            format!("{:<6}", mem_str),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!("{:.1}", p.cpu_pct),
            Style::default().fg(Color::Yellow),
        ),
    ])
}

fn format_mem(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.0}G", mb / 1024.0)
    } else {
        format!("{:.0}M", mb)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}..", &s[..max - 2])
    }
}
