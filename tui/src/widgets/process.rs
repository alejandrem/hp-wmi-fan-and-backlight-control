// ─── Process Widget ─────────────────────────────────────────────────────
// Shows top processes table using Ratatui Table widget for proper
// column alignment: Pid, Program, User, Mem%, Cpu%.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};
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
    let sections = build_proc_sections(inner);
    render_proc_header(frame, sections.header, procs);
    render_proc_table(frame, sections.table, procs);
}

struct ProcSections {
    header: Rect,
    table: Rect,
}

fn build_proc_sections(inner: Rect) -> ProcSections {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
    ProcSections {
        header: parts[0],
        table: parts[1],
    }
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

fn render_proc_header(
    frame: &mut Frame, area: Rect, procs: &[crate::sensors::ProcessInfo],
) {
    let hints = build_header_hints(procs);
    let widget = Paragraph::new(hints)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(widget, area);
}

fn build_header_hints(procs: &[crate::sensors::ProcessInfo]) -> Line<'_> {
    Line::from(vec![
        Span::styled("filter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("                        "),
        Span::styled(format!("{} procs", procs.len()), Style::default()),
    ])
}

fn render_proc_table(
    frame: &mut Frame, area: Rect, procs: &[crate::sensors::ProcessInfo],
) {
    let widths = build_column_widths(area.width);
    let header_row = build_header_row(&widths);
    let data_rows = build_data_rows(procs, &widths);
    let table = Table::default()
        .widths(&widths)
        .header(header_row)
        .rows(data_rows);
    frame.render_widget(table, area);
}

fn build_column_widths(width: u16) -> [Constraint; 5] {
    let avail = width.saturating_sub(5);
    let pid_w = 6.min(avail / 4);
    let user_w = 6.min(avail / 4);
    let mem_w = 5.min(avail / 4);
    let cpu_w = 5.min(avail / 4);
    let prog_w = avail.saturating_sub(pid_w + user_w + mem_w + cpu_w);
    [
        Constraint::Length(pid_w),
        Constraint::Length(prog_w.max(10)),
        Constraint::Length(user_w),
        Constraint::Length(mem_w),
        Constraint::Length(cpu_w),
    ]
}

fn build_header_row(_widths: &[Constraint; 5]) -> Row<'_> {
    let cells = vec![
        Cell::from(" Pid").style(header_style()),
        Cell::from(" Program").style(header_style()),
        Cell::from(" User").style(header_style()),
        Cell::from(" Mem%").style(header_style()),
        Cell::from(" Cpu%").style(header_style()),
    ];
    Row::new(cells).style(header_style())
}

fn header_style() -> Style {
    Style::default()
        .fg(Color::Gray)
        .add_modifier(Modifier::BOLD)
}

fn build_data_rows<'a>(
    procs: &'a [crate::sensors::ProcessInfo],
    _widths: &'a [Constraint; 5],
) -> Vec<Row<'a>> {
    procs.iter().map(|p| {
        let mem_str = format_mem(p.mem_mb);
        let name = truncate(&p.name, 20);
        let cells = vec![
            Cell::from(format!("{:<6}", p.pid)).style(Style::default().fg(Color::White)),
            Cell::from(name).style(Style::default().fg(Color::Cyan)),
            Cell::from(p.user.as_str()).style(Style::default().fg(Color::Magenta)),
            Cell::from(mem_str).style(Style::default().fg(Color::Green)),
            Cell::from(format!("{:.1}", p.cpu_pct)).style(Style::default().fg(Color::Yellow)),
        ];
        Row::new(cells)
    }).collect()
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
        format!("{}..", &s[..max.saturating_sub(2)])
    }
}
